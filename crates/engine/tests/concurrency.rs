use engine::{
    cancel::Cancel,
    domain::{IssueKind, PushPolicy, RepositoryResult},
    local::{self, files::IndexAction},
    scheduler::Manager,
    service::{Event, Service},
    state::{Config, Store},
};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};
use test_support::{Fixture, git, write};

fn service(paths: Vec<PathBuf>, concurrency: usize) -> (tempfile::TempDir, Service) {
    let home = tempfile::tempdir().unwrap();
    let store = Store::custom(home.path().to_owned());
    store
        .save_config(&Config {
            interval_seconds: 21600,
            schedule_anchor: Default::default(),
            repositories: paths,
            excluded_roots: vec![],
            push_policy: PushPolicy::Disabled,
            max_concurrency: concurrency,
            check_on_startup: false,
        })
        .unwrap();
    let manager = Manager::for_user(home.path().to_owned(), unsafe { libc::getuid() }).unwrap();
    let service = Service::new(store, manager);
    (home, service)
}
struct Gate {
    started: PathBuf,
    release: PathBuf,
}
impl Gate {
    fn hook(fixture: &Fixture) -> Self {
        let hook = fixture.repo.join(".git/hooks/pre-commit");
        let started = fixture.temp.path().join("commit.started");
        let release = fixture.temp.path().join("commit.release");
        write(
            &hook,
            &format!(
                "#!/bin/sh\ntouch '{}'\nwhile [ ! -f '{}' ]; do sleep 0.01; done\n",
                started.display(),
                release.display()
            ),
        );
        fs::set_permissions(hook, fs::Permissions::from_mode(0o700)).unwrap();
        Self { started, release }
    }
    fn open(&self) {
        write(&self.release, "release");
    }
    fn wait(&self) {
        let deadline = Instant::now() + Duration::from_secs(8);
        while !self.started.exists() {
            assert!(Instant::now() < deadline, "Git hook did not start");
            thread::sleep(Duration::from_millis(5));
        }
    }
}
impl Drop for Gate {
    fn drop(&mut self) {
        self.open();
    }
}
fn has_issue(result: &RepositoryResult, kind: IssueKind) -> bool {
    matches!(result, RepositoryResult::Attention { issues, .. } if issues.iter().any(|issue| issue.kind == kind))
}
#[test]
fn slow_commit_does_not_lock_another_repository_or_lose_its_saved_result() {
    let first = Fixture::create();
    let second = Fixture::create();
    let (_home, service) = service(vec![first.repo.clone(), second.repo.clone()], 4);
    service.run(&Cancel::default(), |_| {}).unwrap();
    write(first.repo.join("first"), "first");
    git(&first.repo, &["add", "first"]);
    let plan = service
        .prepare_commit(&first.repo, &Cancel::default())
        .unwrap();
    let gate = Gate::hook(&first);
    let worker_service = service.clone();
    let worker =
        thread::spawn(move || worker_service.commit(&plan, "first commit", &Cancel::default()));
    gate.wait();
    write(second.repo.join("second"), "second");
    let files = service.files(&second.repo, &Cancel::default()).unwrap();
    let selected = files.changes.clone();
    service
        .edit_index(&files, IndexAction::Stage, &selected, &Cancel::default())
        .unwrap();
    assert!(!worker.is_finished());
    assert_eq!(
        git(&second.repo, &["diff", "--cached", "--name-only"]),
        "second"
    );
    assert!(service.files(&first.repo, &Cancel::default()).is_ok());
    let reports = service.store.repository_reports().unwrap();
    assert!(has_issue(
        &reports[&second.repo].repositories[0],
        IssueKind::Dirty
    ));
    let second_revision = reports[&second.repo].repository_revisions[&second.repo];
    gate.open();
    worker.join().unwrap().unwrap();
    let reports = service.store.repository_reports().unwrap();
    assert_eq!(reports.len(), 2);
    assert_eq!(
        reports[&second.repo].repository_revisions[&second.repo],
        second_revision
    );
    assert!(has_issue(
        &reports[&first.repo].repositories[0],
        IssueKind::OutOfSync
    ));
}
#[test]
fn shared_worktrees_wait_for_the_same_lock_and_waiting_is_cancellable() {
    let first = Fixture::create();
    let worktree = first.temp.path().join("linked");
    git(
        &first.repo,
        &[
            "worktree",
            "add",
            "-b",
            "linked",
            worktree.to_str().unwrap(),
        ],
    );
    let second = Fixture::create();
    let (_home, service) = service(
        vec![first.repo.clone(), worktree.clone(), second.repo.clone()],
        4,
    );
    let common = local::common_dir(&local::open(&first.repo).unwrap()).unwrap();
    assert_eq!(
        common,
        local::common_dir(&local::open(&worktree).unwrap()).unwrap()
    );
    let lock = service
        .store
        .repository_lock(&common, &Cancel::default())
        .unwrap();
    let cancel = Cancel::default();
    let worker_cancel = cancel.clone();
    let worker_service = service.clone();
    let worker =
        thread::spawn(move || worker_service.check_repository(&worktree, &worker_cancel, |_| {}));
    service
        .check_repository(&second.repo, &Cancel::default(), |_| {})
        .unwrap();
    assert!(!worker.is_finished());
    cancel.cancel();
    assert!(
        worker
            .join()
            .unwrap()
            .unwrap_err()
            .downcast_ref::<engine::cancel::Cancelled>()
            .is_some()
    );
    drop(lock);
}
#[test]
fn a_failed_hook_still_refreshes_repository_state_and_does_not_overwrite_other_repositories() {
    let first = Fixture::create();
    let second = Fixture::create();
    let (_home, service) = service(vec![first.repo.clone(), second.repo.clone()], 4);
    service.run(&Cancel::default(), |_| {}).unwrap();
    write(first.repo.join("first"), "first");
    git(&first.repo, &["add", "first"]);
    let plan = service
        .prepare_commit(&first.repo, &Cancel::default())
        .unwrap();
    let before = service.store.repository_reports().unwrap();
    let hook = first.repo.join(".git/hooks/pre-commit");
    write(&hook, "#!/bin/sh\nprintf changed > hook-created\nexit 1\n");
    fs::set_permissions(hook, fs::Permissions::from_mode(0o700)).unwrap();
    assert!(service.commit(&plan, "failed", &Cancel::default()).is_err());
    let after = service.store.repository_reports().unwrap();
    assert!(
        after[&first.repo].repository_revisions[&first.repo]
            > before[&first.repo].repository_revisions[&first.repo]
    );
    assert_eq!(
        after[&second.repo].repository_revisions[&second.repo],
        before[&second.repo].repository_revisions[&second.repo]
    );
    assert!(has_issue(
        &after[&first.repo].repositories[0],
        IssueKind::Dirty
    ));
}
#[test]
fn concurrency_slots_are_shared_across_services_and_do_not_hold_the_state_transaction() {
    let first = Fixture::create();
    let second = Fixture::create();
    let (_home, service) = service(vec![first.repo.clone(), second.repo.clone()], 1);
    let slot = service.store.operation_slot(1, &Cancel::default()).unwrap();
    let (started, ready) = mpsc::channel();
    let worker_service = service.clone();
    let path = second.repo.clone();
    let worker = thread::spawn(move || {
        worker_service.check_repository(&path, &Cancel::default(), |event| {
            if matches!(event, Event::Checking(_)) {
                started.send(()).unwrap();
            }
        })
    });
    ready.recv_timeout(Duration::from_secs(2)).unwrap();
    assert!(!worker.is_finished());
    let _transaction = service.store.transaction(&Cancel::default()).unwrap();
    drop(_transaction);
    drop(slot);
    worker.join().unwrap().unwrap();
}

#[test]
fn push_result_and_local_state_are_saved_together_for_success_and_failure() {
    for succeed in [true, false] {
        let f = Fixture::create();
        f.commit("ahead", "ahead");
        let (_home, service) = service(vec![f.repo.clone()], 4);
        let plan = service.prepare_push(&f.repo, &Cancel::default()).unwrap();
        if !succeed {
            let hook = f.repo.join(".git/hooks/pre-push");
            write(&hook, "#!/bin/sh\nexit 1\n");
            fs::set_permissions(hook, fs::Permissions::from_mode(0o700)).unwrap();
        }
        let result = service.push(&plan, &Cancel::default()).unwrap();
        assert_eq!(
            matches!(result, engine::domain::Publication::Pushed { .. }),
            succeed
        );
        let saved = service.store.repository_reports().unwrap();
        let report = &saved[&f.repo];
        assert_eq!(report.repositories[0].pushes().len(), 1);
        assert_eq!(
            has_issue(&report.repositories[0], IssueKind::PushFailed),
            !succeed
        );
        assert_eq!(
            matches!(report.repositories[0], RepositoryResult::Clean { .. }),
            succeed
        );
    }
}
