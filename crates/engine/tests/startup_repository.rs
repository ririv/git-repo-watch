use engine::{
    cancel::Cancel,
    local::{self, files::IndexAction},
    scheduler::Manager,
    service::Service,
    state::Store,
};
use std::{fs, os::unix::fs::symlink};
use test_support::{Fixture, git, init, write};

#[test]
fn containing_repository_resolves_nested_directories_symlinks_and_linked_worktrees() {
    let f = Fixture::create();
    let nested = f.repo.join("中文 目录/子目录");
    fs::create_dir_all(&nested).unwrap();
    let cancel = Cancel::default();
    for directory in [&f.repo, &nested] {
        assert_eq!(
            local::containing_repository(directory, &cancel).unwrap(),
            Some(f.repo.clone())
        );
    }
    let link = f.temp.path().join("仓库 链接");
    symlink(&nested, &link).unwrap();
    assert_eq!(
        local::containing_repository(&link, &cancel).unwrap(),
        Some(f.repo.clone())
    );
    let linked = f.temp.path().join("独立 工作树");
    git(
        &f.repo,
        &["worktree", "add", "-b", "linked", linked.to_str().unwrap()],
    );
    let linked_nested = linked.join("src/deep");
    fs::create_dir_all(&linked_nested).unwrap();
    assert_eq!(
        local::containing_repository(&linked_nested, &cancel).unwrap(),
        Some(fs::canonicalize(linked).unwrap())
    );
    assert_eq!(
        local::containing_repository(f.temp.path(), &cancel).unwrap(),
        None
    );
}

#[test]
fn discovery_handles_initial_and_nested_repositories_and_reports_bare_and_cancelled_states() {
    let f = Fixture::create();
    let nested = f.repo.join("nested");
    init(&nested);
    let cancel = Cancel::default();
    assert_eq!(
        local::containing_repository(&nested, &cancel).unwrap(),
        Some(nested)
    );
    assert!(local::containing_repository(&f.remote, &cancel).is_err());
    cancel.cancel();
    assert!(local::containing_repository(&f.repo, &cancel).is_err());
}

#[test]
fn session_repository_access_keeps_registration_and_batch_scope_unchanged() {
    let registered = Fixture::create();
    let temporary = Fixture::create();
    let unrelated = Fixture::create();
    let home = tempfile::tempdir().unwrap();
    let store = Store::custom(home.path().into());
    let manager = Manager::for_user(home.path().into(), unsafe { libc::getuid() }).unwrap();
    let plain = Service::new(store, manager);
    plain.store.initialize().unwrap();
    plain.add(std::slice::from_ref(&registered.repo)).unwrap();
    let config = fs::read(plain.store.config_path()).unwrap();
    let cancel = Cancel::default();
    assert!(plain.files(&temporary.repo, &cancel).is_err());
    let scoped = plain
        .clone()
        .with_session_repository(temporary.repo.clone(), &cancel)
        .unwrap();
    assert!(scoped.files(&registered.repo, &cancel).is_ok());
    assert!(scoped.files(&temporary.repo, &cancel).is_ok());
    assert!(scoped.clone().files(&temporary.repo, &cancel).is_ok());
    assert!(scoped.files(&unrelated.repo, &cancel).is_err());
    assert!(plain.files(&temporary.repo, &cancel).is_err());
    assert_eq!(scoped.snapshot(&cancel).unwrap().repository_count, 1);
    let report = scoped.run_scheduled(&cancel, |_| {}).unwrap();
    assert_eq!(report.repositories.len(), 1);
    assert_eq!(report.repositories[0].path(), registered.repo);
    assert_eq!(fs::read(scoped.store.config_path()).unwrap(), config);
}

#[test]
fn explicitly_staging_in_a_session_repository_uses_existing_locks_and_operation_history() {
    let f = Fixture::create();
    write(f.repo.join("session.txt"), "session work\n");
    let home = tempfile::tempdir().unwrap();
    let store = Store::custom(home.path().into());
    store.initialize().unwrap();
    let config = fs::read(store.config_path()).unwrap();
    let manager = Manager::for_user(home.path().into(), unsafe { libc::getuid() }).unwrap();
    let cancel = Cancel::default();
    let service = Service::new(store, manager)
        .with_session_repository(f.repo.clone(), &cancel)
        .unwrap();
    let files = service.files(&f.repo, &cancel).unwrap();
    let changes = files.changes.clone();
    service
        .edit_index(&files, IndexAction::Stage, &changes, &cancel)
        .unwrap();
    assert!(git(&f.repo, &["diff", "--cached", "--name-only"]).contains("session.txt"));
    assert_eq!(
        service.store.operation_history(&f.repo, 10).unwrap().len(),
        1
    );
    assert_eq!(fs::read(service.store.config_path()).unwrap(), config);
    assert!(service.store.config().unwrap().repositories.is_empty());
}
