use super::*;
use engine::tasks::{Outcome as Completion, State, progress::Stage};

#[test]
fn slow_sync_fetch_publishes_real_progress_without_moving_input_and_records_stage_times() {
    let f = support::Fixture::create();
    f.advance();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    let gate = gate_git(&mut s, &f, GitOperation::Fetch);
    let head = support::git(&f.repo, &["rev-parse", "HEAD"]);
    render(&mut s, 100, 30);
    let input = s.input_area;
    command(&mut s, "repo sync");
    poll_background_until(&mut s, |s| {
        gate.started.exists()
            && s.repository_tasks[&f.repo].label() == Stage::FetchingRemote.label()
    });
    for language in [i18n::Language::Chinese, i18n::Language::English] {
        let _language = i18n::using(language);
        let text = render(&mut s, 100, 30);
        assert_eq!(s.input_area, input);
        assert!(
            text.lines()
                .last()
                .unwrap()
                .contains(Stage::FetchingRemote.label()),
            "{text}"
        );
        assert_eq!(
            text.lines()
                .filter(|line| line.contains(Stage::FetchingRemote.label()))
                .count(),
            1
        );
    }
    let active = s
        .service
        .store
        .tasks(10)
        .unwrap()
        .into_iter()
        .find(|record| record.spec.title == i18n::tr!(SyncPreparing))
        .unwrap();
    assert!(
        matches!(&active.state, State::Running { progress } if progress == Stage::FetchingRemote.label())
    );
    gate.release();
    wait(&mut s);
    assert!(matches!(s.mode, Mode::Confirm { .. }));
    assert_eq!(support::git(&f.repo, &["rev-parse", "HEAD"]), head);
    let record = s.service.store.task(&active.id).unwrap().unwrap();
    assert!(matches!(
        record.state,
        State::Finished {
            outcome: Completion::Succeeded { .. },
            ..
        }
    ));
    let stages = record
        .stage_timings
        .iter()
        .map(|timing| timing.stage)
        .collect::<Vec<_>>();
    assert_eq!(
        stages,
        vec![
            Stage::WaitingRepository,
            Stage::WaitingCapacity,
            Stage::InspectingRepository,
            Stage::FetchingRemote,
            Stage::ComparingBranches,
            Stage::CheckingWorktree,
            Stage::PreparingPreview,
            Stage::RefreshingState
        ]
    );
    assert!(
        record
            .stage_timings
            .iter()
            .any(|timing| timing.stage == Stage::FetchingRemote && timing.elapsed_ms > 0)
    );
    s.tasks.records = vec![record.clone()];
    s.tasks.selection = Some(record.id.clone());
    s.tasks.session = crate::task_center::Session::Detail {
        id: record.id.clone(),
        scroll: 0,
    };
    for language in [i18n::Language::Chinese, i18n::Language::English] {
        let _language = i18n::using(language);
        let text = render(&mut s, 120, 44);
        assert!(text.contains(i18n::tr!(ProgressTimings)), "{text}");
        assert!(text.contains(Stage::FetchingRemote.label()), "{text}");
        assert!(text.contains(" s"), "{text}");
    }
    s.shutdown();
}

#[test]
fn waiting_for_a_repository_is_visible_and_cancellable_without_fetching_or_overwriting_input() {
    let f = support::Fixture::create();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    let common = engine::local::common_dir(&engine::local::open(&f.repo).unwrap()).unwrap();
    let held = s
        .service
        .store
        .repository_lock(&common, &Cancel::default())
        .unwrap();
    command(&mut s, "repo sync");
    poll_background_until(&mut s, |s| {
        s.repository_tasks[&f.repo].label() == Stage::WaitingRepository.label()
    });
    let text = render(&mut s, 100, 30);
    assert!(
        text.lines()
            .last()
            .unwrap()
            .contains(Stage::WaitingRepository.label()),
        "{text}"
    );
    let record = s
        .service
        .store
        .tasks(10)
        .unwrap()
        .into_iter()
        .find(|record| record.active())
        .unwrap();
    s.paste("keep this search");
    s.service.store.cancel_task(&record.id).unwrap();
    poll_background_until(&mut s, |s| s.repository_tasks.is_empty());
    assert_eq!(s.bar.text(), "keep this search");
    let completed = s.service.store.task(&record.id).unwrap().unwrap();
    assert!(matches!(
        completed.state,
        State::Finished {
            outcome: Completion::Cancelled { .. },
            ..
        }
    ));
    assert_eq!(
        completed
            .stage_timings
            .iter()
            .map(|timing| timing.stage)
            .collect::<Vec<_>>(),
        vec![Stage::WaitingRepository]
    );
    drop(held);
    s.shutdown();
}

#[test]
fn waiting_for_execution_capacity_is_reported_separately_and_can_be_cancelled() {
    let f = support::Fixture::create();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    let mut config = s.service.store.config().unwrap();
    config.max_concurrency = 1;
    s.service.store.save_config(&config).unwrap();
    s.config = config;
    let held = s
        .service
        .store
        .operation_slot(1, &Cancel::default())
        .unwrap();
    command(&mut s, "repo sync");
    poll_background_until(&mut s, |s| {
        s.repository_tasks[&f.repo].label() == Stage::WaitingCapacity.label()
    });
    let text = render(&mut s, 100, 30);
    assert!(
        text.lines()
            .last()
            .unwrap()
            .contains(Stage::WaitingCapacity.label()),
        "{text}"
    );
    let record = s
        .service
        .store
        .tasks(10)
        .unwrap()
        .into_iter()
        .find(|record| record.active())
        .unwrap();
    s.service.store.cancel_task(&record.id).unwrap();
    poll_background_until(&mut s, |s| s.repository_tasks.is_empty());
    let completed = s.service.store.task(&record.id).unwrap().unwrap();
    assert!(matches!(
        completed.state,
        State::Finished {
            outcome: Completion::Cancelled { .. },
            ..
        }
    ));
    assert_eq!(
        completed
            .stage_timings
            .iter()
            .map(|timing| timing.stage)
            .collect::<Vec<_>>(),
        vec![Stage::WaitingRepository, Stage::WaitingCapacity]
    );
    drop(held);
    s.shutdown();
}
