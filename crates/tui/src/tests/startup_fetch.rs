use super::*;
use engine::{
    tasks::Source,
    workflow::{Plan, Step},
};

#[test]
fn startup_fetch_is_opt_in_and_saving_the_setting_never_starts_a_fetch() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let f = support::Fixture::create();
        f.advance();
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        let refs = support::git(&f.repo, &["show-ref"]);
        assert!(!s.config.fetch_on_startup);
        s.fetch_on_startup();
        assert!(s.check_task.is_none() && s.repository_tasks.is_empty());
        command(&mut s, "settings startup-fetch");
        let text = render(&mut s, 110, 30);
        assert!(text.contains(i18n::tr!(StartupFetchEnabled)), "{text}");
        assert!(text.contains(i18n::tr!(StartupFetchDisabled)), "{text}");
        command(&mut s, "settings startup-fetch true");
        wait(&mut s);
        assert!(s.config.fetch_on_startup && s.service.store.config().unwrap().fetch_on_startup);
        assert!(s.check_task.is_none() && s.repository_tasks.is_empty());
        assert_eq!(support::git(&f.repo, &["show-ref"]), refs);
        command(&mut s, "settings startup-fetch false");
        wait(&mut s);
        assert!(!s.config.fetch_on_startup);
        assert!(s.service.store.report().unwrap().is_none());
        s.shutdown();
    }
}

#[test]
fn enabled_startup_fetch_is_background_work_and_never_runs_scheduled_pull_or_push() {
    let f = support::Fixture::create();
    f.commit("local.txt", "local commit\n");
    support::write(f.repo.join("local.txt"), "keep local edit\n");
    f.advance();
    let head = support::git(&f.repo, &["rev-parse", "HEAD"]);
    let remote = support::git(&f.remote, &["rev-parse", "main"]);
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    s.config.fetch_on_startup = true;
    s.config.scheduled_actions = "pull push inspect".parse().unwrap();
    s.service.store.save_config(&s.config).unwrap();
    let gate = FetchGate::install(&f);
    s.fetch_on_startup();
    poll_background_until(&mut s, |_| gate.started.exists());
    assert!(matches!(s.mode, Mode::Browsing));
    command(&mut s, "help");
    assert_eq!(s.page, Page::Help);
    gate.release();
    poll_background_until(&mut s, |s| s.check_task.is_none());
    assert_eq!(s.page, Page::Help);
    let report = s.service.store.report().unwrap().unwrap();
    assert_eq!(
        report.execution.as_ref().unwrap().workflow,
        Some(Plan::fetch())
    );
    assert_eq!(
        report.execution.as_ref().unwrap().repositories[0]
            .steps
            .iter()
            .map(|step| step.step)
            .collect::<Vec<_>>(),
        [Step::Fetch, Step::Inspect]
    );
    assert!(report.repositories[0].pushes().is_empty());
    assert_eq!(support::git(&f.repo, &["rev-parse", "origin/main"]), remote);
    assert_eq!(support::git(&f.repo, &["rev-parse", "HEAD"]), head);
    assert_eq!(support::git(&f.remote, &["rev-parse", "main"]), remote);
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    assert_eq!(
        std::fs::read_to_string(f.repo.join("local.txt")).unwrap(),
        "keep local edit\n"
    );
    let records = s.service.store.tasks(10).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].spec.source, Source::Startup);
    s.reload(&Cancel::default()).unwrap();
    render(&mut s, 110, 30);
    assert!(s.check_task.is_none());
    assert_eq!(s.service.store.tasks(10).unwrap().len(), 1);
    s.shutdown();
}

#[test]
fn startup_fetch_includes_a_session_repository_once_without_registering_it() {
    for registered in [true, false] {
        let managed = support::Fixture::create();
        let current = support::Fixture::create();
        managed.advance();
        current.advance();
        let mut paths = vec![managed.repo.clone()];
        if registered {
            paths.push(current.repo.clone());
        }
        let (_home, old) = fixture(paths.clone());
        let mut config = old.config.clone();
        config.fetch_on_startup = true;
        old.service.store.save_config(&config).unwrap();
        let mut s = crate::startup::Entry::Repository(current.repo.clone())
            .open((*old.service).clone(), &Cancel::default())
            .unwrap();
        s.fetch_on_startup();
        poll_background_until(&mut s, |s| {
            s.check_task.is_none() && s.repository_tasks.is_empty()
        });
        for fixture in [&managed, &current] {
            assert_eq!(
                support::git(&fixture.repo, &["rev-parse", "origin/main"]),
                support::git(&fixture.remote, &["rev-parse", "main"])
            );
        }
        assert_eq!(s.service.store.config().unwrap().repositories, paths);
        let records = s.service.store.tasks(10).unwrap();
        assert_eq!(records.len(), if registered { 1 } else { 2 });
        assert!(
            records
                .iter()
                .all(|record| record.spec.source == Source::Startup)
        );
        s.shutdown();
    }
}

#[test]
fn failed_fetches_preserve_failed_state_and_task_details_in_both_entry_points() {
    for startup in [false, true] {
        let f = support::Fixture::create();
        let missing = f.repo.join("missing-remote.git");
        support::git(
            &f.repo,
            &["remote", "set-url", "origin", missing.to_str().unwrap()],
        );
        let head = support::git(&f.repo, &["rev-parse", "HEAD"]);
        let index = std::fs::read(f.repo.join(".git/index")).unwrap();
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        if startup {
            s.config.fetch_on_startup = true;
            s.service.store.save_config(&s.config).unwrap();
            s.fetch_on_startup();
        } else {
            command(&mut s, "repo fetch");
        }
        poll_background_until(&mut s, |s| {
            s.check_task.is_none() && s.repository_tasks.is_empty()
        });
        let report = s.service.store.repository_report(&f.repo).unwrap().unwrap();
        assert!(matches!(
            report.repositories[0],
            RepositoryResult::Failed { .. }
        ));
        let records = s.service.store.tasks(10).unwrap();
        assert_eq!(records.len(), 1);
        assert!(records[0].pending());
        assert!(matches!(&records[0].state,
            engine::tasks::State::Finished { outcome: engine::tasks::Outcome::Failed { detail, .. }, .. }
            if detail.contains("missing-remote.git")
        ));
        if startup {
            assert!(matches!(s.mode, Mode::Browsing));
        } else {
            assert!(s.repository_errors.contains_key(&f.repo));
        }
        assert_eq!(support::git(&f.repo, &["rev-parse", "HEAD"]), head);
        assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
        s.shutdown();
    }
}
