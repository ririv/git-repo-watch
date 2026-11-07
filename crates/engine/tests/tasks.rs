use engine::{
    cancel::Cancel,
    state::Store,
    tasks::{Outcome, Resolution, Retry, Run, Scope, Source, Spec, State},
};
use std::{
    thread,
    time::{Duration, Instant},
};
fn fixture() -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::custom(dir.path().into());
    store.migrate(&Cancel::default()).unwrap();
    (dir, store)
}
fn spec() -> Spec {
    Spec {
        title: "test task".into(),
        scope: Scope::Application,
        source: Source::Interactive,
        retry: Retry::Unavailable,
        retry_of: None,
        operation: None,
    }
}
fn failed(detail: &str) -> Outcome {
    Outcome::Failed {
        detail: detail.into(),
        resolution: Resolution::Pending,
    }
}
#[test]
fn history_survives_reopening_and_reading_does_not_acknowledge_failure() {
    let (_home, store) = fixture();
    let run = Run::begin(&store, spec(), &Cancel::default()).unwrap();
    let id = run.record.id.clone();
    run.finish(failed("full\nerror 中文")).unwrap();
    let reopened = Store::custom(store.config_dir());
    for _ in 0..3 {
        let records = reopened.tasks(10).unwrap();
        assert!(records[0].pending());
        assert!(
            matches!(&records[0].state,State::Finished{outcome:Outcome::Failed{detail,..},..} if detail=="full\nerror 中文")
        );
    }
    reopened.resolve_task(&id).unwrap();
    assert!(!store.task(&id).unwrap().unwrap().pending());
}

#[test]
fn stage_timings_survive_all_completion_kinds_without_changing_the_original_detail() {
    use engine::tasks::progress::{Measurement, Stage};
    let (_home, store) = fixture();
    let timings = vec![Measurement {
        stage: Stage::FetchingRemote,
        elapsed_ms: 1234,
    }];
    for outcome in [
        Outcome::Succeeded {
            detail: "original".into(),
        },
        failed("original"),
        Outcome::Cancelled {
            detail: "original".into(),
        },
    ] {
        let run = Run::begin(&store, spec(), &Cancel::default()).unwrap();
        let id = run.record.id.clone();
        run.finish_with_timings(outcome.clone(), timings.clone())
            .unwrap();
        let reopened = Store::custom(store.config_dir());
        let record = reopened.task(&id).unwrap().unwrap();
        assert_eq!(record.stage_timings, timings);
        assert!(
            matches!(&record.state, State::Finished { outcome: actual, .. } if actual == &outcome)
        );
        let mut legacy = serde_json::to_value(&record).unwrap();
        legacy.as_object_mut().unwrap().remove("stage_timings");
        let legacy: engine::tasks::Record = serde_json::from_value(legacy).unwrap();
        assert!(legacy.stage_timings.is_empty());
    }
}
#[test]
fn active_and_pending_tasks_are_not_hidden_by_the_history_limit() {
    let (_home, store) = fixture();
    let pending = Run::begin(&store, spec(), &Cancel::default()).unwrap();
    pending.finish(failed("needs action")).unwrap();
    for _ in 0..4 {
        Run::begin(&store, spec(), &Cancel::default())
            .unwrap()
            .finish(Outcome::Succeeded {
                detail: "done".into(),
            })
            .unwrap();
    }
    let mut active = Run::begin(&store, spec(), &Cancel::default()).unwrap();
    active.progress("2/4".into()).unwrap();
    let records = store.tasks(1).unwrap();
    assert_eq!(records.len(), 3);
    assert!(records[0].pending());
    assert!(records[1].active());
    assert!(matches!(&records[1].state,State::Running{progress} if progress=="2/4"));
    active
        .finish(Outcome::Succeeded {
            detail: "done".into(),
        })
        .unwrap();
}
#[test]
fn cancellation_reaches_a_worker_owned_by_another_store_instance() {
    let (_home, store) = fixture();
    let cancel = Cancel::default();
    let mut run = Run::begin(&store, spec(), &cancel).unwrap();
    store.cancel_task(&run.record.id).unwrap();
    run.progress("still stopping".into()).unwrap();
    assert!(matches!(
        store.task(&run.record.id).unwrap().unwrap().state,
        State::Cancelling { .. }
    ));
    let deadline = Instant::now() + Duration::from_secs(3);
    while !cancel.is_cancelled() {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(10));
    }
    run.finish(Outcome::Cancelled {
        detail: "cancelled".into(),
    })
    .unwrap();
    assert!(!store.tasks(10).unwrap()[0].active());
}
#[test]
fn dropped_owner_is_reported_as_interrupted_instead_of_remaining_running() {
    let (_home, store) = fixture();
    let run = Run::begin(&store, spec(), &Cancel::default()).unwrap();
    let id = run.record.id.clone();
    assert!(store.tasks(10).unwrap()[0].active());
    drop(run);
    let record = store.tasks(10).unwrap().remove(0);
    assert_eq!(record.id, id);
    assert!(record.pending());
}
#[test]
fn only_successful_retry_resolves_the_original_failure() {
    let (_home, store) = fixture();
    let original = Run::begin(&store, spec(), &Cancel::default()).unwrap();
    let id = original.record.id.clone();
    original.finish(failed("original")).unwrap();
    let mut retry = spec();
    retry.retry_of = Some(id.clone());
    Run::begin(&store, retry.clone(), &Cancel::default())
        .unwrap()
        .finish(failed("retry failed"))
        .unwrap();
    assert!(store.task(&id).unwrap().unwrap().pending());
    Run::begin(&store, retry, &Cancel::default())
        .unwrap()
        .finish(Outcome::Succeeded {
            detail: "retry worked".into(),
        })
        .unwrap();
    assert!(!store.task(&id).unwrap().unwrap().pending());
}
#[test]
fn concurrent_finishes_and_acknowledgements_preserve_all_records() {
    let (_home, store) = fixture();
    thread::scope(|scope| {
        for _ in 0..8 {
            scope.spawn(|| {
                let run = Run::begin(&store, spec(), &Cancel::default()).unwrap();
                let id = run.record.id.clone();
                run.finish(failed("error")).unwrap();
                store.resolve_task(&id).unwrap();
            });
        }
    });
    let records = store.tasks(100).unwrap();
    assert_eq!(records.len(), 8);
    assert!(records.iter().all(|r| !r.pending()));
}
#[test]
fn checks_share_the_journal_and_dirty_repositories_are_successful_tasks() {
    use engine::{scheduler::Manager, service::Service, state::Config};
    use test_support::{Fixture, write};
    let (_home, store) = fixture();
    let repository = Fixture::create();
    write(repository.repo.join("untracked"), "work in progress");
    store
        .save_config(&Config {
            repositories: vec![repository.repo.clone()],
            ..Config::default()
        })
        .unwrap();
    let service = Service::new(
        store.clone(),
        Manager::for_user(store.config_dir(), unsafe { libc::getuid() }).unwrap(),
    );
    service
        .check_repository(&repository.repo, &Cancel::default(), |_| {})
        .unwrap();
    let records = store.tasks(10).unwrap();
    assert_eq!(records.len(), 1);
    assert!(matches!(
        records[0].state,
        State::Finished {
            outcome: Outcome::Succeeded { .. },
            ..
        }
    ));
    service.run_scheduled(&Cancel::default(), |_| {}).unwrap();
    assert_eq!(store.tasks(10).unwrap()[0].spec.source, Source::Scheduled);
    assert_eq!(store.tasks(10).unwrap()[0].spec.retry, Retry::Unavailable);
    let missing = store.config_dir().join("missing-repository");
    assert!(
        service
            .check_repository(&missing, &Cancel::default(), |_| {})
            .is_err()
    );
    let records = store.tasks(10).unwrap();
    assert!(records[0].pending());
    assert!(matches!(records[0].spec.retry, Retry::Fetch { .. }));
}
#[test]
fn version_three_migration_imports_operation_history_and_is_repeatable() {
    use engine::state::{Completion, Operation, OperationRecord};
    use rusqlite::{Connection, params};
    let (_home, store) = fixture();
    let operation = OperationRecord {
        path: "/example/repository".into(),
        at: "2026-10-20T10:00:00+00:00".into(),
        operation: Operation::Stage {
            completion: Completion::Succeeded { value: () },
        },
    };
    let db = Connection::open(store.database_path()).unwrap();
    db.execute(
        "INSERT INTO operation_events(path,revision,at,kind,record) VALUES(?1,NULL,?2,'stage',?3)",
        params![
            b"/example/repository".as_slice(),
            operation.at,
            serde_json::to_string(&operation).unwrap()
        ],
    )
    .unwrap();
    db.execute_batch("DROP TABLE tasks; PRAGMA user_version=3;")
        .unwrap();
    drop(db);
    store.migrate(&Cancel::default()).unwrap();
    let records = store.tasks(10).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].spec.source, Source::Imported);
    assert_eq!(records[0].started_at, operation.at);
    store.migrate(&Cancel::default()).unwrap();
    assert_eq!(store.tasks(10).unwrap(), records);
    assert_eq!(
        store.operation_history(&operation.path, 10).unwrap().len(),
        1
    );
}

#[test]
fn legacy_check_retries_decode_as_canonical_fetch_requests() {
    use engine::tasks::Retry;
    let all: Retry = serde_json::from_str(r#"{"kind":"check_all"}"#).unwrap();
    assert_eq!(all, Retry::FetchAll);
    assert_eq!(serde_json::to_value(all).unwrap()["kind"], "fetch_all");
    let one: Retry =
        serde_json::from_str(r#"{"kind":"check_repository","path":"/example/repository"}"#)
            .unwrap();
    assert_eq!(
        one,
        Retry::Fetch {
            path: "/example/repository".into()
        }
    );
    assert_eq!(serde_json::to_value(one).unwrap()["kind"], "fetch");
}
