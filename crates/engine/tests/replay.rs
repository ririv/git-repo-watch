use engine::{
    cancel::Cancel,
    local::files::IndexAction,
    operations::{
        mutation::{Plan, Request},
        replay::{Control, Kind, Preparation, State},
    },
    scheduler::Manager,
    service::Service,
    state::Store,
};
use std::{fs, num::NonZeroUsize};
use test_support::{Fixture, git, write};
fn service(f: &Fixture) -> (tempfile::TempDir, Service) {
    let home = tempfile::tempdir().unwrap();
    let store = Store::custom(home.path().into());
    store.initialize().unwrap();
    let manager = Manager::for_user(home.path().into(), unsafe { libc::getuid() }).unwrap();
    let service = Service::new(store, manager);
    service.add(std::slice::from_ref(&f.repo)).unwrap();
    (home, service)
}
fn prepare(s: &Service, f: &Fixture, kind: Kind, oid: &str) -> Plan {
    let Preparation::Ready(plan) = s
        .prepare_replay(&f.repo, kind, oid.parse().unwrap(), &Cancel::default())
        .unwrap()
    else {
        panic!("ordinary commit")
    };
    Plan::Replay(plan)
}
fn control(s: &Service, f: &Fixture, action: Control) {
    let plan = s
        .prepare_mutation(&f.repo, Request::ReplayControl(action), &Cancel::default())
        .unwrap();
    s.mutate(&plan, &Cancel::default()).unwrap();
}
fn conflicting() -> (Fixture, String) {
    let f = Fixture::create();
    f.commit("conflict.txt", "base\n");
    git(&f.repo, &["checkout", "-b", "source"]);
    f.commit("conflict.txt", "source\n");
    let oid = git(&f.repo, &["rev-parse", "HEAD"]);
    git(&f.repo, &["checkout", "main"]);
    f.commit("conflict.txt", "ours\n");
    (f, oid)
}
#[test]
fn cherry_pick_preserves_original_author_and_revert_creates_an_inverse_commit() {
    let f = Fixture::create();
    git(&f.repo, &["checkout", "-b", "source"]);
    write(f.repo.join("picked.txt"), "picked\n");
    git(&f.repo, &["add", "picked.txt"]);
    git(
        &f.repo,
        &[
            "-c",
            "user.name=Original Author",
            "-c",
            "user.email=original@example.invalid",
            "commit",
            "-m",
            "feature",
        ],
    );
    let source = git(&f.repo, &["rev-parse", "HEAD"]);
    git(&f.repo, &["checkout", "main"]);
    f.commit("unrelated.txt", "main only\n");
    let before = git(&f.repo, &["rev-parse", "HEAD"]);
    let (_home, s) = service(&f);
    let plan = prepare(&s, &f, Kind::CherryPick, &source);
    assert!(!f.repo.join("picked.txt").exists());
    s.mutate(&plan, &Cancel::default()).unwrap();
    assert_eq!(git(&f.repo, &["show", "HEAD:picked.txt"]), "picked");
    assert_eq!(
        git(&f.repo, &["show", "-s", "--format=%an <%ae>", "HEAD"]),
        "Original Author <original@example.invalid>"
    );
    let picked = git(&f.repo, &["rev-parse", "HEAD"]);
    assert_ne!(picked, source);
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD~1"]), before);
    let plan = prepare(&s, &f, Kind::Revert, &picked);
    s.mutate(&plan, &Cancel::default()).unwrap();
    assert!(!f.repo.join("picked.txt").exists());
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD~1"]), picked);
    assert_eq!(git(&f.repo, &["show", "HEAD:unrelated.txt"]), "main only");
    assert_eq!(s.store.operation_history(&f.repo, 10).unwrap().len(), 2);
}
#[test]
fn conflict_files_are_visible_can_be_staged_and_continue_survives_service_restart() {
    let (f, source) = conflicting();
    let (_home, s) = service(&f);
    let plan = prepare(&s, &f, Kind::CherryPick, &source);
    assert!(s.mutate(&plan, &Cancel::default()).is_err());
    assert!(
        matches!(s.replay_state(&f.repo, &Cancel::default()).unwrap().state, State::Active(ref active) if active.kind == Kind::CherryPick && active.commit.to_string() == source)
    );
    let files = s.files(&f.repo, &Cancel::default()).unwrap();
    let conflicted = files
        .changes
        .iter()
        .find(|c| c.kind == 'U')
        .unwrap()
        .clone();
    assert!(
        s.diff(&files, &conflicted, &Cancel::default())
            .unwrap()
            .text()
            .contains("<<<<<<<")
    );
    assert!(
        s.prepare_mutation(
            &f.repo,
            Request::ReplayControl(Control::Continue),
            &Cancel::default()
        )
        .is_err()
    );
    write(f.repo.join("conflict.txt"), "resolved\n");
    let files = s.files(&f.repo, &Cancel::default()).unwrap();
    let selection = files
        .changes
        .iter()
        .filter(|c| c.kind == 'U')
        .cloned()
        .collect::<Vec<_>>();
    s.edit_index(&files, IndexAction::Stage, &selection, &Cancel::default())
        .unwrap();
    assert!(git(&f.repo, &["ls-files", "-u"]).is_empty());
    git(&f.repo, &["config", "core.editor", "/usr/bin/false"]);
    let hook = f.repo.join(".git/hooks/prepare-commit-msg");
    write(
        &hook,
        "#!/bin/sh\nprintf '%s' \"$GIT_EDITOR\" > .git/replay-editor\n",
    );
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();
    let restarted = Service::new(s.store.clone(), s.scheduler.clone());
    control(&restarted, &f, Control::Continue);
    assert_eq!(git(&f.repo, &["show", "HEAD:conflict.txt"]), "resolved");
    assert_eq!(
        fs::read_to_string(f.repo.join(".git/replay-editor")).unwrap(),
        ":"
    );
    assert_eq!(
        s.replay_state(&f.repo, &Cancel::default()).unwrap().state,
        State::Idle
    );
}
#[test]
fn skip_and_abort_restore_the_pre_operation_commit_and_keep_untracked_files() {
    for action in [Control::Skip, Control::Abort] {
        let (f, source) = conflicting();
        let (_home, s) = service(&f);
        let before = git(&f.repo, &["rev-parse", "HEAD"]);
        write(f.repo.join("untracked.txt"), "keep me\n");
        let plan = prepare(&s, &f, Kind::CherryPick, &source);
        assert!(s.mutate(&plan, &Cancel::default()).is_err());
        control(&s, &f, action);
        assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), before);
        assert_eq!(
            fs::read_to_string(f.repo.join("conflict.txt")).unwrap(),
            "ours\n"
        );
        assert_eq!(
            fs::read_to_string(f.repo.join("untracked.txt")).unwrap(),
            "keep me\n"
        );
        assert_eq!(
            s.replay_state(&f.repo, &Cancel::default()).unwrap().state,
            State::Idle
        );
    }
}
#[test]
fn merge_commits_require_an_explicit_mainline_and_apply_only_that_parents_delta() {
    let f = Fixture::create();
    git(&f.repo, &["checkout", "-b", "left"]);
    f.commit("left.txt", "left\n");
    git(&f.repo, &["checkout", "main"]);
    git(&f.repo, &["checkout", "-b", "right"]);
    f.commit("right.txt", "right\n");
    git(&f.repo, &["checkout", "left"]);
    git(&f.repo, &["merge", "--no-ff", "-m", "merge", "right"]);
    let source = git(&f.repo, &["rev-parse", "HEAD"]);
    git(&f.repo, &["checkout", "main"]);
    let (_home, s) = service(&f);
    let Preparation::Parents(candidate) = s
        .prepare_replay(
            &f.repo,
            Kind::CherryPick,
            source.parse().unwrap(),
            &Cancel::default(),
        )
        .unwrap()
    else {
        panic!("parent choice required")
    };
    let plan = s
        .choose_replay_parent(candidate, NonZeroUsize::new(2).unwrap(), &Cancel::default())
        .unwrap();
    s.mutate(&plan, &Cancel::default()).unwrap();
    assert!(f.repo.join("left.txt").exists());
    assert!(!f.repo.join("right.txt").exists());
}
#[test]
fn dirty_or_changed_reviews_never_execute_and_resolved_conflict_reviews_are_frozen() {
    let f = Fixture::create();
    f.commit("tracked.txt", "before\n");
    let source = git(&f.repo, &["rev-parse", "HEAD"]);
    let (_home, s) = service(&f);
    write(f.repo.join("tracked.txt"), "dirty\n");
    assert!(
        s.prepare_replay(
            &f.repo,
            Kind::Revert,
            source.parse().unwrap(),
            &Cancel::default()
        )
        .is_err()
    );
    git(&f.repo, &["restore", "tracked.txt"]);
    let plan = prepare(&s, &f, Kind::Revert, &source);
    write(f.repo.join("new.txt"), "new work\n");
    assert!(s.mutate(&plan, &Cancel::default()).is_err());
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), source);
    let (f, source) = conflicting();
    let (_home, s) = service(&f);
    assert!(
        s.mutate(
            &prepare(&s, &f, Kind::CherryPick, &source),
            &Cancel::default()
        )
        .is_err()
    );
    let plan = s
        .prepare_mutation(
            &f.repo,
            Request::ReplayControl(Control::Abort),
            &Cancel::default(),
        )
        .unwrap();
    write(f.repo.join("conflict.txt"), "manual resolution\n");
    assert!(s.mutate(&plan, &Cancel::default()).is_err());
    assert_eq!(
        fs::read_to_string(f.repo.join("conflict.txt")).unwrap(),
        "manual resolution\n"
    );
}
#[test]
fn revert_conflicts_can_resolve_to_deletion_and_continue() {
    let f = Fixture::create();
    f.commit("file.txt", "base\n");
    f.commit("file.txt", "change\n");
    let source = git(&f.repo, &["rev-parse", "HEAD"]);
    f.commit("file.txt", "later\n");
    let (_home, s) = service(&f);
    assert!(
        s.mutate(&prepare(&s, &f, Kind::Revert, &source), &Cancel::default())
            .is_err()
    );
    assert!(
        matches!(s.replay_state(&f.repo, &Cancel::default()).unwrap().state, State::Active(ref a) if a.kind == Kind::Revert)
    );
    fs::remove_file(f.repo.join("file.txt")).unwrap();
    let files = s.files(&f.repo, &Cancel::default()).unwrap();
    let selection = files
        .changes
        .iter()
        .filter(|c| c.kind == 'U')
        .cloned()
        .collect::<Vec<_>>();
    s.edit_index(&files, IndexAction::Stage, &selection, &Cancel::default())
        .unwrap();
    assert!(git(&f.repo, &["ls-files", "-u"]).is_empty());
    control(&s, &f, Control::Continue);
    assert!(!f.repo.join("file.txt").exists());
    assert!(git(&f.repo, &["log", "-1", "--format=%s"]).starts_with("Revert"));
}
#[test]
fn linked_worktree_replay_state_is_independent_and_abort_restores_its_head() {
    let (f, source) = conflicting();
    let linked = f.temp.path().join("linked");
    git(
        &f.repo,
        &["worktree", "add", "-b", "linked", linked.to_str().unwrap()],
    );
    let linked = fs::canonicalize(linked).unwrap();
    let (_home, s) = service(&f);
    s.add(std::slice::from_ref(&linked)).unwrap();
    let preparation = s
        .prepare_replay(
            &linked,
            Kind::CherryPick,
            source.parse().unwrap(),
            &Cancel::default(),
        )
        .unwrap();
    let Preparation::Ready(plan) = preparation else {
        panic!("ordinary commit")
    };
    assert!(s.mutate(&Plan::Replay(plan), &Cancel::default()).is_err());
    assert!(matches!(
        s.replay_state(&linked, &Cancel::default()).unwrap().state,
        State::Active(_)
    ));
    assert_eq!(
        s.replay_state(&f.repo, &Cancel::default()).unwrap().state,
        State::Idle
    );
    let abort = s
        .prepare_mutation(
            &linked,
            Request::ReplayControl(Control::Abort),
            &Cancel::default(),
        )
        .unwrap();
    s.mutate(&abort, &Cancel::default()).unwrap();
    assert_eq!(
        fs::read_to_string(linked.join("conflict.txt")).unwrap(),
        "ours\n"
    );
    assert_eq!(
        git(&linked, &["rev-parse", "HEAD"]),
        git(&f.repo, &["rev-parse", "HEAD"])
    );
}
