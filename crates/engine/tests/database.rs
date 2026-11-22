use engine::{
    cancel::{Cancel, Cancelled},
    domain::{Issue, IssueKind, Issues, PushPolicy, Report, ReportStatus, RepositoryResult},
    state::{Completion, Operation, OperationOrigin, OperationRecord, Store},
};
use rusqlite::Connection;
use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    sync::{Arc, Barrier},
    thread,
    time::{Duration, Instant},
};

const AT: &str = "2026-10-22T12:00:00+00:00";
#[test]
fn occurrence_claims_are_atomic_across_first_writes() {
    let (_temp, store) = fixture();
    let store = Arc::new(store);
    let barrier = Arc::new(Barrier::new(8));
    let threads = (0..8)
        .map(|_| {
            let store = store.clone();
            let barrier = barrier.clone();
            thread::spawn(move || {
                barrier.wait();
                store
                    .claim_schedule_occurrence(
                        "same-plan",
                        "2026-10-23 06:00:00",
                        &Cancel::default(),
                    )
                    .unwrap()
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(
        threads
            .into_iter()
            .map(|thread| thread.join().unwrap())
            .filter(|claimed| *claimed)
            .count(),
        1
    );
    assert!(
        store
            .claim_schedule_occurrence("same-plan", "2026-10-23 12:00:00", &Cancel::default())
            .unwrap()
    );
    assert!(
        store
            .claim_schedule_occurrence("another-plan", "2026-10-23 06:00:00", &Cancel::default())
            .unwrap()
    );
}

fn report(path: &Path, revision: u64, dirty: bool) -> Report {
    let result = if dirty {
        RepositoryResult::Attention {
            synchronization: Default::default(),
            path: path.to_owned(),
            issues: Issues::new(
                Issue {
                    kind: IssueKind::Dirty,
                    detail: "changed".into(),
                },
                vec![],
            ),
            branches_without_upstream: vec![],
            pushes: vec![],
        }
    } else {
        RepositoryResult::Clean {
            synchronization: Default::default(),
            path: path.to_owned(),
            branches_without_upstream: vec![],
            pushes: vec![],
        }
    };
    Report {
        schema_version: 3,
        repository_revisions: if revision == 0 {
            BTreeMap::new()
        } else {
            [(path.to_owned(), revision)].into()
        },
        repository_finished_at: [(path.to_owned(), AT.into())].into(),
        push_policy: PushPolicy::Disabled,
        notifications: vec![],
        started_at: AT.into(),
        finished_at: AT.into(),
        status: if dirty {
            ReportStatus::Attention
        } else {
            ReportStatus::Clean
        },
        repositories: vec![result],
        classification_updated_at: None,
        execution: None,
    }
}
fn fixture() -> (tempfile::TempDir, Store) {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::custom(temp.path().to_owned());
    (temp, store)
}
fn record(path: &Path) -> OperationRecord {
    OperationRecord {
        path: path.to_owned(),
        at: AT.into(),
        operation: Operation::Stage {
            completion: Completion::Succeeded { value: () },
        },
    }
}
#[test]
fn fresh_store_reads_do_not_create_a_database_and_updates_are_per_repository_rows() {
    let (_temp, store) = fixture();
    assert!(store.report().unwrap().is_none());
    assert!(store.repository_reports().unwrap().is_empty());
    assert!(store.repository_checks().unwrap().is_empty());
    assert!(!store.database_path().exists());
    let path = Path::new("/example/中文 仓库");
    let mut first = report(path, 0, false);
    store
        .save_repository_report(&mut first, &Cancel::default())
        .unwrap();
    assert_eq!(first.repository_revisions[path], 1);
    let mut second = report(path, 0, true);
    store
        .save_operation_report(&mut second, &record(path), &Cancel::default())
        .unwrap();
    assert_eq!(second.repository_revisions[path], 2);
    let other = Path::new("/example/only-local");
    store
        .save_operation_report(
            &mut report(other, 0, true),
            &record(other),
            &Cancel::default(),
        )
        .unwrap();
    let checks = store.repository_checks().unwrap();
    assert_eq!(checks.len(), 1);
    assert_eq!(checks[path].repository_revisions[path], 1);
    assert_eq!(store.repository_history(path, 10).unwrap().len(), 2);
    let events = store.operation_history(path, 10).unwrap();
    assert_eq!(events.len(), 1);
    assert!(matches!(events[0].origin,OperationOrigin::Observation{revision} if revision.get()==2));
    assert!(store.report().unwrap().is_none());
    assert!(!store.state_dir().join("repository-checks.json").exists());
    let connection = Connection::open(store.database_path()).unwrap();
    assert_eq!(
        connection
            .pragma_query_value(None, "journal_mode", |row| row.get::<_, String>(0))
            .unwrap(),
        "wal"
    );
    assert!(rusqlite::version_number() >= 3_051_003);
    for path in [
        store.database_path(),
        store.state_dir().join("state.sqlite3-wal"),
        store.state_dir().join("state.sqlite3-shm"),
    ] {
        if path.exists() {
            assert_eq!(
                fs::metadata(path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }
}
#[test]
fn legacy_state_is_refused_without_creating_a_database_or_changing_the_source() {
    let (_temp, store) = fixture();
    fs::create_dir(store.state_dir()).unwrap();
    let legacy = store.state_dir().join("latest.json");
    fs::write(&legacy, b"legacy state").unwrap();
    assert!(store.report().is_err());
    assert!(
        store
            .save_repository_report(
                &mut report(Path::new("/example/repo"), 0, false),
                &Cancel::default()
            )
            .is_err()
    );
    assert_eq!(fs::read(legacy).unwrap(), b"legacy state");
    assert!(!store.database_path().exists());
}

#[test]
fn a_failed_operation_insert_rolls_back_the_state_revision_head_and_observation() {
    let (_temp, store) = fixture();
    let path = Path::new("/example/repo");
    let mut initial = report(path, 0, false);
    store
        .save_repository_report(&mut initial, &Cancel::default())
        .unwrap();
    let connection = Connection::open(store.database_path()).unwrap();
    connection.execute_batch("CREATE TRIGGER reject_operation BEFORE INSERT ON operation_events BEGIN SELECT RAISE(ABORT,'injected failure'); END;").unwrap();
    let mut next = report(path, 0, true);
    assert!(
        store
            .save_operation_report(&mut next, &record(path), &Cancel::default())
            .is_err()
    );
    assert!(next.repository_revisions.is_empty());
    assert_eq!(
        store
            .repository_report(path)
            .unwrap()
            .unwrap()
            .repository_revisions[path],
        1
    );
    assert_eq!(store.repository_history(path, 10).unwrap().len(), 1);
    assert!(store.operation_history(path, 10).unwrap().is_empty());
    connection
        .execute_batch("DROP TRIGGER reject_operation;")
        .unwrap();
    store
        .save_operation_report(&mut next, &record(path), &Cancel::default())
        .unwrap();
    assert_eq!(next.repository_revisions[path], 2);
}
#[test]
fn simultaneous_first_writes_and_updates_allocate_unique_monotonic_revisions() {
    let (_temp, store) = fixture();
    let barrier = Arc::new(Barrier::new(8));
    let handles = (0..8)
        .map(|_| {
            let barrier = barrier.clone();
            let store = store.clone();
            thread::spawn(move || {
                barrier.wait();
                let mut value = report(Path::new("/example/repo"), 0, false);
                store
                    .save_repository_report(&mut value, &Cancel::default())
                    .unwrap();
                value.repository_revisions[Path::new("/example/repo")]
            })
        })
        .collect::<Vec<_>>();
    let mut revisions = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect::<Vec<_>>();
    revisions.sort();
    assert_eq!(revisions, (1..=8).collect::<Vec<_>>());
    assert_eq!(
        store
            .repository_history(Path::new("/example/repo"), 100)
            .unwrap()
            .len(),
        8
    );
}
#[test]
fn wal_reads_continue_during_a_writer_and_a_waiting_write_can_be_cancelled() {
    let (_temp, store) = fixture();
    let path = Path::new("/example/repo");
    let mut initial = report(path, 0, false);
    store
        .save_repository_report(&mut initial, &Cancel::default())
        .unwrap();
    let connection = Connection::open(store.database_path()).unwrap();
    connection.execute_batch("BEGIN IMMEDIATE;").unwrap();
    let start = Instant::now();
    assert_eq!(
        store
            .repository_report(path)
            .unwrap()
            .unwrap()
            .repository_revisions[path],
        1
    );
    assert!(start.elapsed() < Duration::from_secs(1));
    let cancel = Cancel::default();
    let worker_cancel = cancel.clone();
    let worker_store = store.clone();
    let worker = thread::spawn(move || {
        let mut next = report(Path::new("/example/repo"), 0, true);
        worker_store.save_operation_report(
            &mut next,
            &record(Path::new("/example/repo")),
            &worker_cancel,
        )
    });
    thread::sleep(Duration::from_millis(60));
    assert!(!worker.is_finished());
    let start = Instant::now();
    cancel.cancel();
    assert!(
        worker
            .join()
            .unwrap()
            .unwrap_err()
            .downcast_ref::<Cancelled>()
            .is_some()
    );
    assert!(start.elapsed() < Duration::from_secs(1));
    connection.execute_batch("ROLLBACK;").unwrap();
    assert_eq!(store.repository_history(path, 10).unwrap().len(), 1);
    assert!(store.operation_history(path, 10).unwrap().is_empty());
}
#[test]
fn invalid_databases_and_incompatible_schema_versions_are_never_recreated() {
    let path = Path::new("/example/repo");
    let cancel = Cancel::default();
    let (_temp, store) = fixture();
    fs::create_dir(store.state_dir()).unwrap();
    fs::write(store.database_path(), b"not a database").unwrap();
    assert!(store.report().is_err());
    assert!(
        store
            .save_repository_report(&mut report(path, 0, false), &cancel)
            .is_err()
    );
    assert_eq!(fs::read(store.database_path()).unwrap(), b"not a database");

    for version in [4, 99] {
        let (_temp, store) = fixture();
        store
            .save_repository_report(&mut report(path, 0, false), &cancel)
            .unwrap();
        let connection = Connection::open(store.database_path()).unwrap();
        connection
            .pragma_update(None, "user_version", version)
            .unwrap();
        assert!(store.report().is_err());
        assert!(
            store
                .save_repository_report(&mut report(path, 0, false), &cancel)
                .is_err()
        );
        assert_eq!(
            connection
                .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
                .unwrap(),
            version
        );
        assert_eq!(
            connection
                .query_row("SELECT count(*) FROM repository_observations", [], |row| {
                    row.get::<_, i64>(0)
                })
                .unwrap(),
            1
        );
    }
}

#[test]
fn transferring_a_home_copies_wal_commits_histories_and_exact_configuration_without_overwriting() {
    let (_temp, source) = fixture();
    source
        .save_config(&engine::state::Config::default())
        .unwrap();
    let path = Path::new("/example/中文 仓库");
    let mut first = report(path, 0, false);
    source
        .save_repository_report(&mut first, &Cancel::default())
        .unwrap();
    let keep_wal = Connection::open(source.database_path()).unwrap();
    let mut next = report(path, 0, true);
    source
        .save_operation_report(&mut next, &record(path), &Cancel::default())
        .unwrap();
    source.save_report(&next).unwrap();
    let destination_home = tempfile::tempdir().unwrap();
    let destination =
        engine::directories::UserDirectories::linux(destination_home.path(), None, None)
            .unwrap()
            .store();
    let imported = destination
        .migrate_from(&source, &Cancel::default())
        .unwrap();
    assert!(matches!(imported, engine::state::Transfer::Database { .. }));
    assert_eq!(
        fs::read(destination.config_path()).unwrap(),
        fs::read(source.config_path()).unwrap()
    );
    assert_eq!(
        serde_json::to_value(destination.report().unwrap()).unwrap(),
        serde_json::to_value(source.report().unwrap()).unwrap()
    );
    assert_eq!(destination.repository_history(path, 100).unwrap().len(), 2);
    assert_eq!(destination.operation_history(path, 100).unwrap().len(), 1);
    assert_eq!(
        destination
            .repository_report(path)
            .unwrap()
            .unwrap()
            .repository_revisions[path],
        2
    );
    assert!(
        destination
            .migrate_from(&source, &Cancel::default())
            .is_err()
    );
    assert_eq!(destination.repository_history(path, 100).unwrap().len(), 2);
    assert_eq!(source.repository_history(path, 100).unwrap().len(), 2);
    assert!(source.database_path().exists());
    drop(keep_wal);
}
#[test]
fn transferring_requires_an_idle_source_and_cancellation_preserves_existing_files() {
    let (_temp, source) = fixture();
    source
        .save_config(&engine::state::Config::default())
        .unwrap();
    let destination_home = tempfile::tempdir().unwrap();
    let destination = Store::custom(destination_home.path().to_owned());
    let slot = source.operation_slot(4, &Cancel::default()).unwrap();
    let busy = destination
        .migrate_from(&source, &Cancel::default())
        .unwrap_err();
    assert!(busy.downcast_ref::<engine::state::Busy>().is_some());
    assert!(!destination.config_path().exists());
    drop(slot);
    let cancel = Cancel::default();
    cancel.cancel();
    assert!(destination.migrate_from(&source, &cancel).is_err());
    assert!(!destination.config_path().exists());
    assert!(!destination.database_path().exists());
    assert!(matches!(
        destination
            .migrate_from(&source, &Cancel::default())
            .unwrap(),
        engine::state::Transfer::Configuration { .. }
    ));
    assert!(destination.config_path().exists());
    assert!(!destination.database_path().exists());
}

#[test]
fn transferring_a_json_profile_converts_configuration_and_preserves_its_database() {
    let (_temp, source) = fixture();
    let config = engine::state::Config {
        repositories: vec!["/example/中文 空格".into()],
        max_concurrency: 8,
        fetch_on_startup: false,
        auto_fetch: Default::default(),
        experimental_features: true,
        chinese_command_trigger: true,
        text_wrap: false,
        file_icons: true,
        view: Default::default(),
        mouse: Default::default(),
        editor: Default::default(),
        ai: Default::default(),
        ..engine::state::Config::default()
    };
    fs::write(
        source.legacy_config_path(),
        serde_json::to_vec(&config).unwrap(),
    )
    .unwrap();
    let report = report(Path::new("/example/中文 空格"), 0, false);
    source.save_report(&report).unwrap();
    let destination_home = tempfile::tempdir().unwrap();
    let destination = Store::custom(destination_home.path().to_owned());
    assert!(matches!(
        destination
            .migrate_from(&source, &Cancel::default())
            .unwrap(),
        engine::state::Transfer::Database { .. }
    ));
    assert!(!source.legacy_config_path().exists());
    assert!(!destination.legacy_config_path().exists());
    assert_eq!(source.config().unwrap(), config);
    assert_eq!(destination.config().unwrap(), config);
    assert_eq!(
        fs::read(source.config_path()).unwrap(),
        fs::read(destination.config_path()).unwrap()
    );
    assert_eq!(
        serde_json::to_value(source.report().unwrap()).unwrap(),
        serde_json::to_value(destination.report().unwrap()).unwrap()
    );
}

#[test]
fn current_schema_records_discard_and_retains_prior_operations() {
    let (_temp, store) = fixture();
    let path = Path::new("/example/discard");
    let cancel = Cancel::default();
    store
        .save_operation_report(&mut report(path, 0, false), &record(path), &cancel)
        .unwrap();
    let before = store.operation_history(path, 10).unwrap()[0].id;
    let discard = OperationRecord {
        path: path.to_owned(),
        at: AT.into(),
        operation: Operation::Discard {
            completion: Completion::Succeeded { value: () },
        },
    };
    store
        .save_operation_report(&mut report(path, 0, false), &discard, &cancel)
        .unwrap();
    let history = store.operation_history(path, 10).unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(history[1].id, before);
    assert!(matches!(
        history[0].record.operation,
        Operation::Discard {
            completion: Completion::Succeeded { .. }
        }
    ));
}
