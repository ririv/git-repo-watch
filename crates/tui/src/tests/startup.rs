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

#[test]
fn current_directory_start_opens_registered_or_session_repositories_and_back_returns_home() {
    for registered in [true, false] {
        let f = support::Fixture::create();
        support::write(f.repo.join("文件.ts"), "export const value = 1;\n");
        let (_home, old) = fixture(if registered {
            vec![f.repo.clone()]
        } else {
            vec![]
        });
        let config = std::fs::read(old.service.store.config_path()).unwrap();
        let index = std::fs::read(f.repo.join(".git/index")).unwrap();
        let entry = crate::Start::CurrentDirectory
            .in_directory(&f.repo, &Cancel::default())
            .unwrap();
        let mut s = entry
            .open((*old.service).clone(), &Cancel::default())
            .unwrap();
        assert_eq!(s.page, Page::Detail);
        assert!(matches!(s.mode, Mode::Browsing));
        assert_eq!(s.scope_path(), Some(f.repo.clone()));
        assert_eq!(s.breadcrumbs().len(), 2);
        wait_workspace(&mut s);
        super::overview::wait_overview(&mut s);
        assert!(matches!(
            &s.workspace.session().unwrap().files,
            crate::workspace::Panel::Ready(_)
        ));
        assert!(matches!(
            &s.workspace.session().unwrap().branches,
            crate::workspace::Panel::Ready(_)
        ));
        assert!(matches!(
            &s.workspace.session().unwrap().history,
            crate::workspace::Panel::Ready(_)
        ));
        assert!(render(&mut s, 160, 36).contains("文件.ts"));
        command(&mut s, "repo diff");
        wait(&mut s);
        assert_eq!(s.mode.file_session().unwrap().snapshot.path, f.repo);
        key(&mut s, KeyCode::Esc);
        key(&mut s, KeyCode::Left);
        assert_eq!(s.page, Page::Repositories);
        assert_eq!(
            std::fs::read(s.service.store.config_path()).unwrap(),
            config
        );
        assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
        s.shutdown();
    }
}

#[test]
fn explicit_repository_list_start_skips_directory_discovery_and_outside_git_opens_home() {
    let f = support::Fixture::create();
    let outside = tempfile::tempdir().unwrap();
    for (start, directory) in [
        (crate::Start::Repositories, f.repo.as_path()),
        (
            crate::Start::Repositories,
            std::path::Path::new("/missing/directory"),
        ),
        (crate::Start::CurrentDirectory, outside.path()),
    ] {
        let entry = start.in_directory(directory, &Cancel::default()).unwrap();
        assert_eq!(entry, crate::startup::Entry::Repositories);
        let (_home, old) = fixture(vec![f.repo.clone()]);
        let mut s = entry
            .open((*old.service).clone(), &Cancel::default())
            .unwrap();
        assert_eq!(s.page, Page::Repositories);
        assert!(s.workspace.path().is_none());
        assert!(s.history.is_empty());
        s.shutdown();
    }
}

#[test]
fn session_repository_observations_survive_reload_without_joining_the_repository_list() {
    let f = support::Fixture::create();
    let (_home, old) = fixture(vec![]);
    let config = std::fs::read(old.service.store.config_path()).unwrap();
    let entry = crate::Start::CurrentDirectory
        .in_directory(&f.repo, &Cancel::default())
        .unwrap();
    let mut s = entry
        .open((*old.service).clone(), &Cancel::default())
        .unwrap();
    s.service
        .check_repository(&f.repo, &Cancel::default(), |_| {})
        .unwrap();
    s.reload(&Cancel::default()).unwrap();
    assert!(s.latest_update(&f.repo).is_some());
    assert!(s.rows().is_empty());
    assert_eq!(s.scope_path(), Some(f.repo));
    assert_eq!(
        std::fs::read(s.service.store.config_path()).unwrap(),
        config
    );
    s.shutdown();
}
