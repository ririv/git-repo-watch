use engine::{
    cancel::Cancel,
    domain::PushPolicy,
    local::{
        branches::edit,
        stash::{self, SaveMode},
    },
    operations::mutation::{Change, Request},
    scheduler::Manager,
    service::Service,
    state::{Config, Store},
};
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};
use test_support::{Fixture, git, write};
fn service(paths: Vec<PathBuf>) -> (tempfile::TempDir, Service) {
    let home = tempfile::tempdir().unwrap();
    let store = Store::custom(home.path().into());
    store
        .save_config(&Config {
            interval_seconds: 21600,
            schedule_anchor: Default::default(),
            repositories: paths,
            excluded_roots: vec![],
            push_policy: PushPolicy::Disabled,
            max_concurrency: 4,
            check_on_startup: false,
            chinese_command_trigger: true,
            text_wrap: false,
            interaction_mode: Default::default(),
            mouse: Default::default(),
            ai: Default::default(),
        })
        .unwrap();
    let manager = Manager::for_user(home.path().into(), unsafe { libc::getuid() }).unwrap();
    (home, Service::new(store, manager))
}
fn branch(action: edit::Action, name: &str) -> Request {
    Request::Branch {
        action,
        name: name.into(),
    }
}
fn run(s: &Service, f: &Fixture, request: Request) -> Change {
    let cancel = Cancel::default();
    let plan = s.prepare_mutation(&f.repo, request, &cancel).unwrap();
    s.mutate(&plan, &cancel).unwrap()
}
#[test]
fn branches_create_without_switching_and_safe_switch_runs_the_post_checkout_hook() {
    let f = Fixture::create();
    f.commit("file", "before\n");
    let (_home, s) = service(vec![f.repo.clone()]);
    run(&s, &f, branch(edit::Action::Create, "feature/中文"));
    assert_eq!(git(&f.repo, &["branch", "--show-current"]), "main");
    let hook = f.repo.join(".git/hooks/post-checkout");
    write(
        &hook,
        "#!/bin/sh\nprintf '%s\\n' \"$1\" \"$2\" \"$3\" > .hook-result\n",
    );
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o700)).unwrap();
    run(&s, &f, branch(edit::Action::Switch, "feature/中文"));
    assert_eq!(git(&f.repo, &["branch", "--show-current"]), "feature/中文");
    assert_eq!(
        fs::read_to_string(f.repo.join(".hook-result"))
            .unwrap()
            .lines()
            .last(),
        Some("1")
    );
    assert!(s.store.repository_report(&f.repo).unwrap().is_some());
    assert_eq!(s.store.operation_history(&f.repo, 10).unwrap().len(), 2);
}
#[test]
fn branch_review_protects_dirty_files_changed_targets_and_other_worktrees() {
    let f = Fixture::create();
    f.commit("file", "main\n");
    let (_home, s) = service(vec![f.repo.clone()]);
    run(&s, &f, branch(edit::Action::Create, "other"));
    write(f.repo.join("file"), "dirty\n");
    assert!(
        s.prepare_mutation(
            &f.repo,
            branch(edit::Action::Switch, "other"),
            &Cancel::default()
        )
        .is_err()
    );
    assert_eq!(fs::read_to_string(f.repo.join("file")).unwrap(), "dirty\n");
    git(&f.repo, &["restore", "file"]);
    let plan = s
        .prepare_mutation(
            &f.repo,
            branch(edit::Action::Switch, "other"),
            &Cancel::default(),
        )
        .unwrap();
    git(&f.repo, &["branch", "-f", "other", "HEAD~1"]);
    assert!(s.mutate(&plan, &Cancel::default()).is_err());
    assert_eq!(git(&f.repo, &["branch", "--show-current"]), "main");
    let linked = f.temp.path().join("linked");
    git(
        &f.repo,
        &["worktree", "add", linked.to_str().unwrap(), "other"],
    );
    assert!(
        s.prepare_mutation(
            &f.repo,
            branch(edit::Action::Switch, "other"),
            &Cancel::default()
        )
        .is_err()
    );
    assert!(
        s.prepare_mutation(
            &f.repo,
            branch(edit::Action::Create, "other"),
            &Cancel::default()
        )
        .is_err()
    );
    assert!(
        s.prepare_mutation(
            &f.repo,
            branch(edit::Action::Create, "--invalid"),
            &Cancel::default()
        )
        .is_err()
    );
}
#[test]
fn stash_modes_apply_pop_and_drop_preserve_their_distinct_effects() {
    let f = Fixture::create();
    f.commit("tracked", "before\n");
    let (_home, s) = service(vec![f.repo.clone()]);
    write(f.repo.join("tracked"), "tracked change\n");
    write(f.repo.join("中文 new"), "new\n");
    run(
        &s,
        &f,
        Request::Stash(stash::Request::Save {
            mode: SaveMode::Tracked,
            message: "tracked only".into(),
        }),
    );
    assert_eq!(
        fs::read_to_string(f.repo.join("tracked")).unwrap(),
        "before\n"
    );
    assert!(f.repo.join("中文 new").exists());
    let first = s.stashes(&f.repo, &Cancel::default()).unwrap().entries[0].oid;
    run(&s, &f, Request::Stash(stash::Request::Apply(first)));
    assert_eq!(
        s.stashes(&f.repo, &Cancel::default())
            .unwrap()
            .entries
            .len(),
        1
    );
    git(&f.repo, &["restore", "tracked"]);
    run(&s, &f, Request::Stash(stash::Request::Pop(first)));
    assert!(
        s.stashes(&f.repo, &Cancel::default())
            .unwrap()
            .entries
            .is_empty()
    );
    run(
        &s,
        &f,
        Request::Stash(stash::Request::Save {
            mode: SaveMode::IncludeUntracked,
            message: "all".into(),
        }),
    );
    assert!(!f.repo.join("中文 new").exists());
    let second = s.stashes(&f.repo, &Cancel::default()).unwrap().entries[0].oid;
    run(&s, &f, Request::Stash(stash::Request::Drop(second)));
    assert!(!f.repo.join("中文 new").exists());
    assert!(
        s.stashes(&f.repo, &Cancel::default())
            .unwrap()
            .entries
            .is_empty()
    );
    assert_eq!(s.store.operation_history(&f.repo, 20).unwrap().len(), 5);
}
#[test]
fn stash_review_detects_content_and_reflog_changes_before_any_destructive_write() {
    let f = Fixture::create();
    f.commit("file", "base\n");
    let (_home, s) = service(vec![f.repo.clone()]);
    write(f.repo.join("file"), "first\n");
    let request = Request::Stash(stash::Request::Save {
        mode: SaveMode::Tracked,
        message: "first".into(),
    });
    let plan = s
        .prepare_mutation(&f.repo, request.clone(), &Cancel::default())
        .unwrap();
    write(f.repo.join("file"), "later\n");
    assert!(s.mutate(&plan, &Cancel::default()).is_err());
    assert_eq!(fs::read_to_string(f.repo.join("file")).unwrap(), "later\n");
    run(&s, &f, request);
    let oid = s.stashes(&f.repo, &Cancel::default()).unwrap().entries[0].oid;
    let plan = s
        .prepare_mutation(
            &f.repo,
            Request::Stash(stash::Request::Drop(oid)),
            &Cancel::default(),
        )
        .unwrap();
    write(f.repo.join("file"), "second\n");
    git(&f.repo, &["stash", "push", "-m", "external stash"]);
    assert!(s.mutate(&plan, &Cancel::default()).is_err());
    assert_eq!(
        s.stashes(&f.repo, &Cancel::default())
            .unwrap()
            .entries
            .len(),
        2
    );
}

#[test]
fn pull_fetches_once_and_fast_forwards_the_exact_reviewed_upstream() {
    let f = Fixture::create();
    let (_home, s) = service(vec![f.repo.clone()]);
    let before = git(&f.repo, &["rev-parse", "HEAD"]);
    f.advance();
    let remote = git(&f.remote, &["rev-parse", "main"]);
    let plan = s
        .prepare_mutation(&f.repo, Request::Pull, &Cancel::default())
        .unwrap();
    assert!(!plan.current());
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), before);
    s.mutate(&plan, &Cancel::default()).unwrap();
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), remote);
    assert!(
        s.prepare_mutation(&f.repo, Request::Pull, &Cancel::default())
            .unwrap()
            .current()
    );
    assert_eq!(s.store.operation_history(&f.repo, 10).unwrap().len(), 3);
}

#[test]
fn pull_refuses_divergence_stale_reviews_and_deleted_upstreams_without_merging() {
    let f = Fixture::create();
    let (_home, s) = service(vec![f.repo.clone()]);
    f.advance();
    let plan = s
        .prepare_mutation(&f.repo, Request::Pull, &Cancel::default())
        .unwrap();
    git(&f.repo, &["commit", "--allow-empty", "-m", "local ahead"]);
    let local = git(&f.repo, &["rev-parse", "HEAD"]);
    assert!(s.mutate(&plan, &Cancel::default()).is_err());
    assert!(
        s.prepare_mutation(&f.repo, Request::Pull, &Cancel::default())
            .is_err()
    );
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), local);
    git(&f.remote, &["update-ref", "-d", "refs/heads/main"]);
    assert!(
        s.prepare_mutation(&f.repo, Request::Pull, &Cancel::default())
            .is_err()
    );
    assert!(
        git2::Repository::open(&f.repo)
            .unwrap()
            .find_reference("refs/remotes/origin/main")
            .is_err()
    );
}

#[test]
fn explicit_fetch_and_pull_use_custom_refspecs_and_do_not_push() {
    let f = Fixture::create();
    let (_home, s) = service(vec![f.repo.clone()]);
    git(
        &f.repo,
        &[
            "config",
            "--replace-all",
            "remote.origin.fetch",
            "+refs/heads/*:refs/remotes/custom/*",
        ],
    );
    f.advance();
    let head = git(&f.repo, &["rev-parse", "HEAD"]);
    let remote = git(&f.remote, &["rev-parse", "main"]);
    assert!(matches!(
        s.fetch(&f.repo, &Cancel::default()).unwrap(),
        Change::Fetched { .. }
    ));
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), head);
    assert_eq!(
        git(&f.repo, &["rev-parse", "refs/remotes/custom/main"]),
        remote
    );
    run(&s, &f, Request::Pull);
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), remote);
    assert_eq!(git(&f.remote, &["rev-parse", "main"]), remote);
}

#[test]
fn conflicted_stash_pop_retains_the_stash_and_records_the_actual_repository_state() {
    let f = Fixture::create();
    f.commit("file", "base\n");
    let (_home, s) = service(vec![f.repo.clone()]);
    write(f.repo.join("file"), "stashed\n");
    run(
        &s,
        &f,
        Request::Stash(stash::Request::Save {
            mode: SaveMode::Tracked,
            message: "conflict".into(),
        }),
    );
    let oid = s.stashes(&f.repo, &Cancel::default()).unwrap().entries[0].oid;
    f.commit("file", "other branch contents\n");
    let plan = s
        .prepare_mutation(
            &f.repo,
            Request::Stash(stash::Request::Pop(oid)),
            &Cancel::default(),
        )
        .unwrap();
    assert!(s.mutate(&plan, &Cancel::default()).is_err());
    assert_eq!(
        s.stashes(&f.repo, &Cancel::default()).unwrap().entries[0].oid,
        oid
    );
    assert!(
        git2::Repository::open(&f.repo)
            .unwrap()
            .index()
            .unwrap()
            .has_conflicts()
    );
    assert!(s.store.repository_report(&f.repo).unwrap().is_some());
}

#[test]
fn safe_switch_preserves_untracked_collisions_and_records_hook_failure_after_checkout() {
    let f = Fixture::create();
    git(&f.repo, &["checkout", "-b", "target"]);
    f.commit("collision", "committed\n");
    git(&f.repo, &["checkout", "main"]);
    write(f.repo.join("collision"), "untracked\n");
    let (_home, s) = service(vec![f.repo.clone()]);
    let plan = s
        .prepare_mutation(
            &f.repo,
            branch(edit::Action::Switch, "target"),
            &Cancel::default(),
        )
        .unwrap();
    assert!(s.mutate(&plan, &Cancel::default()).is_err());
    assert_eq!(git(&f.repo, &["branch", "--show-current"]), "main");
    assert_eq!(
        fs::read_to_string(f.repo.join("collision")).unwrap(),
        "untracked\n"
    );
    fs::remove_file(f.repo.join("collision")).unwrap();
    let hook = f.repo.join(".git/hooks/post-checkout");
    write(&hook, "#!/bin/sh\necho hook-failed >&2\nexit 1\n");
    fs::set_permissions(hook, fs::Permissions::from_mode(0o700)).unwrap();
    let plan = s
        .prepare_mutation(
            &f.repo,
            branch(edit::Action::Switch, "target"),
            &Cancel::default(),
        )
        .unwrap();
    assert!(s.mutate(&plan, &Cancel::default()).is_err());
    assert_eq!(git(&f.repo, &["branch", "--show-current"]), "target");
    assert_eq!(
        fs::read_to_string(f.repo.join("collision")).unwrap(),
        "committed\n"
    );
    let history = s.store.operation_history(&f.repo, 10).unwrap();
    assert_eq!(history.len(), 2);
    assert!(matches!(
        history[0].record.operation,
        engine::state::Operation::Workspace {
            completion: engine::state::Completion::Failed { .. },
            ..
        }
    ));
    assert!(s.store.repository_report(&f.repo).unwrap().is_some());
}

#[test]
fn local_upstream_fast_forward_requires_no_remote_and_never_implicitly_sets_tracking() {
    let f = Fixture::create();
    f.commit("local", "incoming\n");
    git(&f.repo, &["branch", "local-upstream"]);
    git(&f.repo, &["reset", "--hard", "HEAD~1"]);
    git(&f.repo, &["remote", "remove", "origin"]);
    git(&f.repo, &["config", "branch.main.remote", "."]);
    git(
        &f.repo,
        &["config", "branch.main.merge", "refs/heads/local-upstream"],
    );
    let (_home, s) = service(vec![f.repo.clone()]);
    run(&s, &f, Request::Pull);
    assert_eq!(
        git(&f.repo, &["rev-parse", "HEAD"]),
        git(&f.repo, &["rev-parse", "local-upstream"])
    );
    assert_eq!(git(&f.repo, &["config", "branch.main.remote"]), ".");
    assert_eq!(
        fs::read_to_string(f.repo.join("local")).unwrap(),
        "incoming\n"
    );
}
