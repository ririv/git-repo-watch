use engine::{
    cancel::{Cancel, Cancelled},
    domain::{Issue, IssueKind, Issues, PushPolicy, Report, ReportStatus, RepositoryResult},
    state::{Completion, Migration, Operation, OperationOrigin, OperationRecord, Store},
};
use rusqlite::Connection;
use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::{Arc, Barrier},
    thread,
    time::{Duration, Instant},
};

const AT: &str = "2026-10-22T12:00:00+00:00";
fn report(path: &Path, revision: u64, dirty: bool) -> Report {
    let result = if dirty {
        RepositoryResult::Attention {
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
    let store = Store {
        home: temp.path().to_owned(),
    };
    (temp, store)
}
fn legacy(store: &Store, name: &str, value: &impl serde::Serialize) -> Vec<u8> {
    fs::create_dir_all(store.state_dir()).unwrap();
    let bytes = serde_json::to_vec_pretty(value).unwrap();
    fs::write(store.state_dir().join(name), &bytes).unwrap();
    bytes
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
fn legacy_batch_latest_rows_operations_and_scan_are_imported_once_and_original_bytes_are_archived()
{
    let (_temp, store) = fixture();
    let path = Path::new("/example/repo");
    let batch = report(path, 4, false);
    let current = report(path, 5, true);
    let batch_bytes = legacy(&store, "latest.json", &batch);
    let row_bytes = legacy(
        &store,
        "repository-checks.json",
        &BTreeMap::from([(path.to_owned(), current)]),
    );
    legacy(
        &store,
        "manual-commit.json",
        &serde_json::json!({"at":AT,"path":path,"branch":"main","result":{"oid":"0123456789012345678901234567890123456789","review":"exact"}}),
    );
    legacy(
        &store,
        "manual-push.json",
        &serde_json::json!({"at":AT,"path":path,"push":{"status":"pushed","target":{"branch":"main","local_ref":"refs/heads/main","local_oid":"0123456789012345678901234567890123456789","remote":"origin","remote_ref":"refs/heads/main","remote_oid":"abcdefabcdefabcdefabcdefabcdefabcdefabcd","ahead_commits":1}}}),
    );
    legacy(
        &store,
        "import-report.json",
        &engine::discovery::Report {
            root: PathBuf::from("/example"),
            scanned_at: AT.into(),
            excluded_roots: vec![],
            repositories: vec![engine::discovery::Record::Scanned {
                path: path.to_owned(),
            }],
            non_repository_markers: vec![],
        },
    );
    assert!(store.report().is_err());
    assert!(matches!(
        store.migrate(&Cancel::default()).unwrap(),
        Migration::Imported { files: 5 }
    ));
    assert_eq!(
        serde_json::to_value(store.report().unwrap().unwrap()).unwrap(),
        serde_json::to_value(batch).unwrap()
    );
    assert_eq!(
        store
            .repository_report(path)
            .unwrap()
            .unwrap()
            .repository_revisions[path],
        5
    );
    assert_eq!(store.scan_report().unwrap().unwrap().repositories.len(), 1);
    assert_eq!(store.operation_history(path, 10).unwrap().len(), 2);
    assert!(
        store
            .operation_history(path, 10)
            .unwrap()
            .iter()
            .all(|entry| matches!(entry.origin, OperationOrigin::Imported))
    );
    assert_eq!(
        fs::read(store.state_dir().join("legacy-json/latest.json")).unwrap(),
        batch_bytes
    );
    assert_eq!(
        fs::read(store.state_dir().join("legacy-json/repository-checks.json")).unwrap(),
        row_bytes
    );
    assert!(!store.state_dir().join("latest.json").exists());
    assert!(matches!(
        store.migrate(&Cancel::default()).unwrap(),
        Migration::Ready
    ));
    assert_eq!(store.operation_history(path, 10).unwrap().len(), 2);
    let mut next = report(path, 0, false);
    store
        .save_repository_report(&mut next, &Cancel::default())
        .unwrap();
    assert_eq!(next.repository_revisions[path], 6);
}
#[test]
fn unversioned_legacy_reports_are_compared_by_their_actual_times_before_assigning_versions() {
    let (_temp, store) = fixture();
    let path = Path::new("/example/repo");
    let mut batch = report(path, 0, false);
    batch.finished_at = "2026-10-22T13:00:00+00:00".into();
    batch.repository_finished_at.clear();
    let old = report(path, 0, true);
    legacy(&store, "latest.json", &batch);
    legacy(
        &store,
        "repository-checks.json",
        &BTreeMap::from([(path.to_owned(), old)]),
    );
    store.migrate(&Cancel::default()).unwrap();
    assert!(matches!(
        store.repository_report(path).unwrap().unwrap().repositories[0],
        RepositoryResult::Clean { .. }
    ));
}
#[test]
fn conflicting_or_malformed_legacy_data_is_not_discarded_or_replaced_by_an_empty_database() {
    for corrupt in [true, false] {
        let (_temp, store) = fixture();
        let path = Path::new("/example/repo");
        legacy(&store, "latest.json", &report(path, 7, false));
        if corrupt {
            fs::write(store.state_dir().join("repository-checks.json"), b"broken").unwrap();
        } else {
            legacy(
                &store,
                "repository-checks.json",
                &BTreeMap::from([(path.to_owned(), report(path, 7, true))]),
            );
        }
        let before = fs::read(store.state_dir().join("latest.json")).unwrap();
        assert!(store.migrate(&Cancel::default()).is_err());
        assert_eq!(
            fs::read(store.state_dir().join("latest.json")).unwrap(),
            before
        );
        assert!(!store.state_dir().join("legacy-json").exists());
        assert!(store.report().is_err());
        let connection = Connection::open(store.database_path()).unwrap();
        assert_eq!(
            connection
                .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
}
#[test]
fn interrupted_archival_resumes_without_duplicating_imported_records() {
    let (_temp, store) = fixture();
    let path = Path::new("/example/repo");
    legacy(
        &store,
        "repository-checks.json",
        &BTreeMap::from([(path.to_owned(), report(path, 3, true))]),
    );
    store.migrate(&Cancel::default()).unwrap();
    let connection = Connection::open(store.database_path()).unwrap();
    connection
        .execute("UPDATE legacy_imports SET state='pending'", [])
        .unwrap();
    fs::rename(
        store.state_dir().join("legacy-json/repository-checks.json"),
        store.state_dir().join("repository-checks.json"),
    )
    .unwrap();
    assert!(store.repository_reports().is_err());
    store.migrate(&Cancel::default()).unwrap();
    assert_eq!(store.repository_history(path, 100).unwrap().len(), 1);
    assert!(!store.state_dir().join("repository-checks.json").exists());
}
#[test]
fn completed_archives_can_be_deleted_without_reimporting_or_accepting_recreated_legacy_json() {
    let (_temp, store) = fixture();
    let path = Path::new("/example/repo");
    let bytes = legacy(
        &store,
        "repository-checks.json",
        &BTreeMap::from([(path.to_owned(), report(path, 3, true))]),
    );
    store.migrate(&Cancel::default()).unwrap();
    fs::remove_dir_all(store.state_dir().join("legacy-json")).unwrap();
    assert!(matches!(
        store.migrate(&Cancel::default()).unwrap(),
        Migration::Ready
    ));
    assert!(!store.state_dir().join("legacy-json").exists());
    assert_eq!(store.repository_history(path, 100).unwrap().len(), 1);
    assert_eq!(
        store
            .repository_report(path)
            .unwrap()
            .unwrap()
            .repository_revisions[path],
        3
    );
    fs::write(store.state_dir().join("repository-checks.json"), &bytes).unwrap();
    assert!(store.migrate(&Cancel::default()).is_err());
    assert!(store.repository_reports().is_err());
    assert_eq!(
        fs::read(store.state_dir().join("repository-checks.json")).unwrap(),
        bytes
    );
}

#[test]
fn an_old_process_writing_json_after_migration_is_reported_instead_of_becoming_a_second_source() {
    let (_temp, store) = fixture();
    let path = Path::new("/example/repo");
    let mut original = report(path, 0, false);
    store
        .save_repository_report(&mut original, &Cancel::default())
        .unwrap();
    let bytes = legacy(
        &store,
        "repository-checks.json",
        &BTreeMap::from([(path.to_owned(), report(path, 2, true))]),
    );
    assert!(store.repository_reports().is_err());
    assert!(store.migrate(&Cancel::default()).is_err());
    assert_eq!(
        fs::read(store.state_dir().join("repository-checks.json")).unwrap(),
        bytes
    );
    fs::remove_file(store.state_dir().join("repository-checks.json")).unwrap();
    assert_eq!(
        store
            .repository_report(path)
            .unwrap()
            .unwrap()
            .repository_revisions[path],
        1
    );
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
fn invalid_databases_and_future_schema_versions_are_never_recreated() {
    let (_temp, store) = fixture();
    fs::create_dir(store.state_dir()).unwrap();
    fs::write(store.database_path(), b"not a database").unwrap();
    assert!(store.report().is_err());
    assert!(store.migrate(&Cancel::default()).is_err());
    assert_eq!(fs::read(store.database_path()).unwrap(), b"not a database");
    let (_temp, store) = fixture();
    store.migrate(&Cancel::default()).unwrap();
    let connection = Connection::open(store.database_path()).unwrap();
    connection.pragma_update(None, "user_version", 99).unwrap();
    assert!(store.report().is_err());
    assert!(store.migrate(&Cancel::default()).is_err());
    assert_eq!(
        connection
            .pragma_query_value(None, "user_version", |row| row.get::<_, i64>(0))
            .unwrap(),
        99
    );
}
