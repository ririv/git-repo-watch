use engine::{
    cancel::Cancel,
    operations::{
        mutation::{Plan, Request},
        replay::{Control, State, state::Queue},
    },
    scheduler::Manager,
    service::Service,
    state::Store,
};
use std::fs;
use test_support::{Fixture, git, write};

fn service(f: &Fixture) -> (tempfile::TempDir, Service) {
    let home = tempfile::tempdir().unwrap();
    let store = Store::custom(home.path().into());
    store.initialize().unwrap();
    let manager = Manager::for_current_user_in(home.path().into()).unwrap();
    let service = Service::new(store, manager);
    service.add(std::slice::from_ref(&f.repo)).unwrap();
    (home, service)
}

fn batch(s: &Service, f: &Fixture, commits: &[String]) -> Plan {
    s.prepare_cherry_pick_sequence(
        &f.repo,
        &commits
            .iter()
            .map(|oid| oid.parse().unwrap())
            .collect::<Vec<_>>(),
        &Cancel::default(),
    )
    .unwrap()
}

fn resume(s: &Service, f: &Fixture, control: Control) {
    let plan = s
        .prepare_mutation(&f.repo, Request::ReplayControl(control), &Cancel::default())
        .unwrap();
    s.mutate(&plan, &Cancel::default()).unwrap();
}

fn sequence_with_conflict() -> (Fixture, Vec<String>) {
    let f = Fixture::create();
    f.commit("conflict.txt", "base\n");
    git(&f.repo, &["checkout", "-b", "source"]);
    let mut commits = Vec::new();
    for (path, text) in [
        ("first.txt", "first\n"),
        ("conflict.txt", "source\n"),
        ("last.txt", "last\n"),
    ] {
        f.commit(path, text);
        commits.push(git(&f.repo, &["rev-parse", "HEAD"]));
    }
    git(&f.repo, &["checkout", "main"]);
    f.commit("conflict.txt", "ours\n");
    (f, commits)
}

#[test]
fn sequence_preserves_reviewed_order_and_applies_dependent_commits() {
    let f = Fixture::create();
    git(&f.repo, &["checkout", "-b", "source"]);
    f.commit("feature.txt", "first\n");
    let first = git(&f.repo, &["rev-parse", "HEAD"]);
    f.commit("feature.txt", "second\n");
    let second = git(&f.repo, &["rev-parse", "HEAD"]);
    git(&f.repo, &["checkout", "main"]);
    f.commit("target.txt", "target\n");
    let before = git(&f.repo, &["rev-parse", "HEAD"]);
    let (_home, s) = service(&f);
    let commits = vec![first, second];
    let plan = batch(&s, &f, &commits);
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), before);
    let Plan::Replay(engine::operations::replay::Plan::Batch(review)) = &plan else {
        panic!("batch review");
    };
    assert_eq!(
        review
            .steps()
            .map(|step| step.candidate.commit.to_string())
            .collect::<Vec<_>>(),
        commits
    );
    s.mutate(&plan, &Cancel::default()).unwrap();
    assert_eq!(
        fs::read_to_string(f.repo.join("feature.txt")).unwrap(),
        "second\n"
    );
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD~2"]), before);
    assert_eq!(git(&f.repo, &["show", "HEAD~1:feature.txt"]), "first");
    assert_eq!(
        s.replay_state(&f.repo, &Cancel::default()).unwrap().state,
        State::Idle
    );
    assert!(f.repo.join("target.txt").exists());
}

#[test]
fn second_commit_conflict_retains_remaining_sequence_across_restart() {
    let (f, commits) = sequence_with_conflict();
    let before = git(&f.repo, &["rev-parse", "HEAD"]);
    let (_home, s) = service(&f);
    assert!(
        s.mutate(&batch(&s, &f, &commits), &Cancel::default())
            .is_err()
    );
    let State::Active(active) = s.replay_state(&f.repo, &Cancel::default()).unwrap().state else {
        panic!("active sequence");
    };
    assert_eq!(active.commit.to_string(), commits[1]);
    assert!(
        matches!(&active.queue, Queue::Sequence { start, remaining } if start.to_string() == before && remaining.iter().map(ToString::to_string).collect::<Vec<_>>() == commits[2..])
    );
    assert_eq!(
        active.conflicts,
        vec![std::path::PathBuf::from("conflict.txt")]
    );
    assert!(f.repo.join("first.txt").exists());
    assert!(!f.repo.join("last.txt").exists());
    write(f.repo.join("conflict.txt"), "resolved\n");
    git(&f.repo, &["add", "conflict.txt"]);
    let restarted = Service::new(s.store.clone(), s.scheduler.clone());
    let State::Active(ready) = restarted
        .replay_state(&f.repo, &Cancel::default())
        .unwrap()
        .state
    else {
        panic!("active sequence");
    };
    assert!(ready.conflicts.is_empty());
    resume(&restarted, &f, Control::Continue);
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD~3"]), before);
    assert_eq!(
        fs::read_to_string(f.repo.join("conflict.txt")).unwrap(),
        "resolved\n"
    );
    assert!(f.repo.join("last.txt").exists());
    assert_eq!(
        s.replay_state(&f.repo, &Cancel::default()).unwrap().state,
        State::Idle
    );
}

#[test]
fn skip_runs_later_commits_while_abort_restores_the_entire_sequence() {
    for control in [Control::Skip, Control::Abort] {
        let (f, commits) = sequence_with_conflict();
        let before = git(&f.repo, &["rev-parse", "HEAD"]);
        let (_home, s) = service(&f);
        write(f.repo.join("keep.txt"), "untracked\n");
        assert!(
            s.mutate(&batch(&s, &f, &commits), &Cancel::default())
                .is_err()
        );
        resume(&s, &f, control);
        assert_eq!(
            fs::read_to_string(f.repo.join("conflict.txt")).unwrap(),
            "ours\n"
        );
        assert_eq!(
            fs::read_to_string(f.repo.join("keep.txt")).unwrap(),
            "untracked\n"
        );
        match control {
            Control::Skip => {
                assert!(f.repo.join("first.txt").exists());
                assert!(f.repo.join("last.txt").exists());
                assert_eq!(git(&f.repo, &["rev-parse", "HEAD~2"]), before);
            }
            Control::Abort => {
                assert!(!f.repo.join("first.txt").exists());
                assert!(!f.repo.join("last.txt").exists());
                assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), before);
            }
            Control::Continue => unreachable!(),
        }
        assert_eq!(
            s.replay_state(&f.repo, &Cancel::default()).unwrap().state,
            State::Idle
        );
    }
}

#[test]
fn review_changes_and_merge_selection_are_rejected_before_any_sequence_writes() {
    let f = Fixture::create();
    git(&f.repo, &["checkout", "-b", "source"]);
    f.commit("picked.txt", "picked\n");
    let first = git(&f.repo, &["rev-parse", "HEAD"]);
    f.commit("second.txt", "second\n");
    let second = git(&f.repo, &["rev-parse", "HEAD"]);
    git(&f.repo, &["checkout", "main"]);
    f.commit("target.txt", "target\n");
    git(&f.repo, &["merge", "--no-ff", "-m", "merge", "source"]);
    let merge = git(&f.repo, &["rev-parse", "HEAD"]);
    let (_home, s) = service(&f);
    assert!(
        s.prepare_cherry_pick_sequence(
            &f.repo,
            &[first.parse().unwrap(), merge.parse().unwrap()],
            &Cancel::default()
        )
        .is_err()
    );
    let plan = batch(&s, &f, &[first, second]);
    write(f.repo.join("target.txt"), "new edit\n");
    assert!(s.mutate(&plan, &Cancel::default()).is_err());
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), merge);
    assert_eq!(
        fs::read_to_string(f.repo.join("target.txt")).unwrap(),
        "new edit\n"
    );
}

#[test]
fn an_empty_pick_can_be_skipped_without_losing_the_rest_of_the_batch() {
    let f = Fixture::create();
    git(&f.repo, &["checkout", "-b", "source"]);
    f.commit("already.txt", "same change\n");
    let first = git(&f.repo, &["rev-parse", "HEAD"]);
    f.commit("needed.txt", "new change\n");
    let second = git(&f.repo, &["rev-parse", "HEAD"]);
    git(&f.repo, &["checkout", "main"]);
    f.commit("target.txt", "target\n");
    f.commit("already.txt", "same change\n");
    let (_home, s) = service(&f);
    assert!(
        s.mutate(
            &batch(&s, &f, &[first.clone(), second.clone()]),
            &Cancel::default()
        )
        .is_err()
    );
    let State::Active(active) = s.replay_state(&f.repo, &Cancel::default()).unwrap().state else {
        panic!("paused sequence");
    };
    assert_eq!(active.commit.to_string(), first);
    assert!(active.conflicts.is_empty());
    assert_eq!(
        active
            .remaining()
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        [second]
    );
    resume(&s, &f, Control::Skip);
    assert!(f.repo.join("needed.txt").exists());
    assert_eq!(
        s.replay_state(&f.repo, &Cancel::default()).unwrap().state,
        State::Idle
    );
}
