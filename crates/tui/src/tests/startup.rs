use super::*;

#[test]
fn startup_checks_all_repositories_in_background_keeps_old_results_and_never_pushes() {
    let first = support::Fixture::create();
    let second = support::Fixture::create();
    support::write(first.repo.join("untracked"), "old result");
    let (_home, mut s) = fixture(vec![first.repo.clone(), second.repo.clone()]);
    s.config.check_on_startup = true;
    s.config.scheduled_actions = engine::workflow::Plan::from_policy(PushPolicy::FastForward);
    s.service.store.save_config(&s.config).unwrap();
    s.service.run(&Cancel::default(), |_| {}).unwrap();
    std::fs::remove_file(first.repo.join("untracked")).unwrap();
    support::git(
        &first.repo,
        &["commit", "--allow-empty", "-m", "unpublished"],
    );
    let remote_before = support::git(&first.remote, &["rev-parse", "main"]);
    s.reload(&Cancel::default()).unwrap();
    let previous = serde_json::to_vec(&s.service.store.report().unwrap()).unwrap();
    let gate = FetchGate::install(&first);
    s.check_on_startup();
    poll_background_until(&mut s, |_| gate.started.exists());
    assert!(matches!(s.mode, Mode::Browsing));
    assert!(s.check_task.as_ref().unwrap().checking(&first.repo));
    assert!(matches!(
        s.result_for(&first.repo),
        Some(RepositoryResult::Attention { .. })
    ));
    assert_eq!(
        serde_json::to_vec(&s.service.store.report().unwrap()).unwrap(),
        previous
    );
    command(&mut s, "help");
    assert_eq!(s.page, Page::Help);
    gate.release();
    poll_background_until(&mut s, |s| s.check_task.is_none());
    assert_eq!(s.page, Page::Help);
    let report = s.service.store.report().unwrap().unwrap();
    assert_eq!(report.repositories.len(), 2);
    assert_eq!(report.push_policy, PushPolicy::Disabled);
    assert!(
        report
            .repositories
            .iter()
            .all(|result| result.pushes().is_empty())
    );
    assert_eq!(
        support::git(&first.remote, &["rev-parse", "main"]),
        remote_before
    );
    s.reload(&Cancel::default()).unwrap();
    render(&mut s, 100, 28);
    assert!(s.check_task.is_none());
}

#[test]
fn disabled_empty_and_already_running_startups_do_not_launch_a_worker() {
    for (enabled, repositories, activity) in [
        (
            false,
            vec![PathBuf::from("/example/unavailable")],
            Activity::Idle,
        ),
        (true, vec![], Activity::Idle),
        (
            true,
            vec![PathBuf::from("/example/unavailable")],
            Activity::Running,
        ),
    ] {
        let (_home, mut s) = fixture(repositories);
        s.config.check_on_startup = enabled;
        s.snapshot.activity = activity;
        s.check_on_startup();
        assert!(s.check_task.is_none());
        assert!(s.service.store.report().unwrap().is_none());
    }
}

#[test]
fn closing_during_a_startup_check_cancels_it_and_retains_the_previous_report() {
    let f = support::Fixture::create();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    s.service.run(&Cancel::default(), |_| {}).unwrap();
    s.config.check_on_startup = true;
    s.service.store.save_config(&s.config).unwrap();
    s.reload(&Cancel::default()).unwrap();
    let previous = serde_json::to_vec(&s.service.store.report().unwrap()).unwrap();
    let gate = FetchGate::install(&f);
    s.check_on_startup();
    poll_background_until(&mut s, |_| gate.started.exists());
    s.shutdown();
    assert!(s.check_task.is_none());
    assert_eq!(
        serde_json::to_vec(&s.service.store.report().unwrap()).unwrap(),
        previous
    );
    assert!(!s.service.store.running().unwrap());
}

#[test]
fn startup_setting_is_localized_editable_persistent_and_effective_only_next_launch() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec![]);
        s.config.check_on_startup = true;
        s.service.store.save_config(&s.config).unwrap();
        command(&mut s, "settings");
        for (width, height) in [(60, 18), (100, 28)] {
            assert!(render(&mut s, width, height).contains(i18n::tr!(SettingsStartup)));
        }
        key(&mut s, KeyCode::Down);
        key(&mut s, KeyCode::Right);
        assert_eq!(s.bar.text(), "/settings startup enabled ");
        command(&mut s, "settings startup enabled false");
        assert!(matches!(s.mode, Mode::Confirm { .. }));
        key(&mut s, KeyCode::Esc);
        assert!(s.service.store.config().unwrap().check_on_startup);
        assert_eq!(s.bar.text(), "/settings startup enabled false");
        key(&mut s, KeyCode::Enter);
        key(&mut s, KeyCode::Enter);
        wait(&mut s);
        assert!(!s.service.store.config().unwrap().check_on_startup);
        assert!(!s.config.check_on_startup);
        assert!(s.check_task.is_none());
        assert!(s.service.store.report().unwrap().is_none());
        command(&mut s, "settings startup enabled invalid");
        assert!(matches!(s.mode, Mode::Browsing));
        assert!(!s.message.is_empty());
        assert!(!s.service.store.config().unwrap().check_on_startup);
    }
}
