use super::*;
use crate::live::WindowFocus;

fn enable(s: &mut Screen) {
    s.config.auto_fetch.enabled = true;
    s.config.auto_fetch.interval_seconds = 60;
    s.service.store.save_config(&s.config).unwrap();
}
fn finish(s: &mut Screen, path: &std::path::Path, at: Instant) {
    let started = Instant::now();
    while s.auto_fetch.running(path) {
        s.poll_auto_fetch_at(at);
        assert!(started.elapsed() < Duration::from_secs(12));
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn automatic_fetch_is_opt_in_and_settings_persist_without_changing_the_system_schedule() {
    for language in [i18n::Language::Chinese, i18n::Language::English] {
        let _language = i18n::using(language);
        let f = support::Fixture::create();
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        assert!(!s.config.auto_fetch.enabled);
        assert_eq!(s.config.auto_fetch.interval_seconds, 180);
        s.poll_auto_fetch();
        assert!(s.service.store.tasks(10).unwrap().is_empty());
        command(&mut s, "settings auto-fetch");
        assert!(render(&mut s, 120, 30).contains(i18n::tr!(AutoFetchEnabled)));
        command(&mut s, "settings auto-fetch true");
        wait(&mut s);
        assert!(s.config.auto_fetch.enabled);
        command(&mut s, "settings auto-fetch-interval 6m");
        assert!(matches!(s.mode, Mode::Confirm { .. }));
        key(&mut s, KeyCode::Enter);
        wait(&mut s);
        assert_eq!(s.config.auto_fetch.interval_seconds, 360);
        assert_eq!(
            s.service.store.config().unwrap().auto_fetch,
            s.config.auto_fetch
        );
        command(&mut s, "settings auto-fetch false");
        wait(&mut s);
        assert!(!s.config.auto_fetch.enabled);
        assert_eq!(s.config.interval_seconds, 21600);
        assert_eq!(s.config.scheduled_actions, engine::workflow::Plan::fetch());
        s.shutdown();
    }
}

#[test]
fn focused_fetches_repeat_after_completion_and_preserve_input_selection_and_scroll() {
    let f = support::Fixture::create();
    f.advance();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    enable(&mut s);
    s.paste("//repos fil");
    let input = s.bar.text();
    s.scroll = 9;
    s.message = "keep notice".into();
    let selected = s.selected_path();
    let at = Instant::now();
    s.poll_auto_fetch_at(at);
    assert!(s.auto_fetch.running(&f.repo));
    finish(&mut s, &f.repo, at);
    assert_eq!(
        support::git(&f.repo, &["rev-parse", "origin/main"]),
        support::git(&f.remote, &["rev-parse", "main"])
    );
    assert_eq!(s.bar.text(), input);
    assert_eq!(s.scroll, 9);
    assert_eq!(s.message, "keep notice");
    assert_eq!(s.selected_path(), selected);
    assert_eq!(s.service.store.tasks(20).unwrap().len(), 1);
    s.poll_auto_fetch_at(at + Duration::from_secs(59));
    assert!(!s.auto_fetch.running(&f.repo));
    s.poll_auto_fetch_at(at + Duration::from_secs(60));
    assert!(s.auto_fetch.running(&f.repo));
    finish(&mut s, &f.repo, at + Duration::from_secs(60));
    let records = s.service.store.tasks(20).unwrap();
    assert_eq!(records.len(), 2);
    assert!(
        records
            .iter()
            .all(|record| record.spec.source == engine::tasks::Source::Automatic)
    );
    s.shutdown();
}

#[test]
fn unfocused_time_produces_one_due_fetch_on_return_and_disabling_stops_new_fetches() {
    let f = support::Fixture::create();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    enable(&mut s);
    let at = Instant::now();
    s.window_focus_changed(WindowFocus::Unfocused);
    s.poll_auto_fetch_at(at);
    s.poll_auto_fetch_at(at + Duration::from_secs(600));
    assert!(!s.auto_fetch.running(&f.repo));
    assert!(s.service.store.tasks(10).unwrap().is_empty());
    s.window_focus_changed(WindowFocus::Focused);
    s.poll_auto_fetch_at(at + Duration::from_secs(600));
    finish(&mut s, &f.repo, at + Duration::from_secs(600));
    assert_eq!(s.service.store.tasks(10).unwrap().len(), 1);
    s.config.auto_fetch.enabled = false;
    s.poll_auto_fetch_at(at + Duration::from_secs(2000));
    assert!(!s.auto_fetch.running(&f.repo));
    assert_eq!(s.service.store.tasks(10).unwrap().len(), 1);
    s.shutdown();
}

#[test]
fn manual_and_startup_fetches_reset_the_automatic_deadline() {
    for startup in [false, true] {
        let f = support::Fixture::create();
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        enable(&mut s);
        if startup {
            s.config.fetch_on_startup = true;
            s.service.store.save_config(&s.config).unwrap();
            s.fetch_on_startup();
            poll_background_until(&mut s, |s| s.check_task.is_none());
        } else {
            command(&mut s, "repo fetch");
            wait(&mut s);
        }
        s.poll_auto_fetch_at(Instant::now() + Duration::from_secs(10));
        assert!(!s.auto_fetch.running(&f.repo));
        assert_eq!(s.service.store.tasks(20).unwrap().len(), 1);
        s.shutdown();
    }
}

#[test]
fn enabling_automatic_fetch_during_a_manual_fetch_keeps_its_completion_deadline() {
    let f = support::Fixture::create();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    let gate = FetchGate::install(&f);
    command(&mut s, "repo fetch");
    poll_background_until(&mut s, |_| gate.started.exists());
    enable(&mut s);
    gate.release();
    wait(&mut s);
    s.poll_auto_fetch_at(Instant::now() + Duration::from_secs(10));
    assert!(!s.auto_fetch.running(&f.repo));
    assert_eq!(s.service.store.tasks(20).unwrap().len(), 1);
    s.shutdown();
}

#[test]
fn failed_automatic_fetch_stays_in_task_history_without_opening_an_error_page_or_retrying_immediately()
 {
    let f = support::Fixture::create();
    support::git(
        &f.repo,
        &[
            "remote",
            "set-url",
            "origin",
            f.repo.join("missing.git").to_str().unwrap(),
        ],
    );
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    enable(&mut s);
    s.paste("//help");
    let input = s.bar.text();
    let at = Instant::now();
    s.poll_auto_fetch_at(at);
    finish(&mut s, &f.repo, at);
    assert!(matches!(s.mode, Mode::Browsing));
    assert_eq!(s.bar.text(), input);
    assert!(matches!(
        s.result_for(&f.repo),
        Some(RepositoryResult::Failed { .. })
    ));
    let records = s.service.store.tasks(20).unwrap();
    assert_eq!(records.len(), 1);
    assert!(records[0].pending());
    s.poll_auto_fetch_at(at + Duration::from_secs(59));
    assert!(!s.auto_fetch.running(&f.repo));
    s.shutdown();
}

#[test]
fn busy_repositories_wait_without_preventing_other_repositories_from_fetching() {
    let a = support::Fixture::create();
    let b = support::Fixture::create();
    let (_home, mut s) = fixture(vec![a.repo.clone(), b.repo.clone()]);
    enable(&mut s);
    let (release, gate) = mpsc::channel();
    s.start_repository(a.repo.clone(), "busy", Effect::ReadOnly, move |_, _, _| {
        gate.recv_timeout(Duration::from_secs(12)).unwrap();
        Ok(Outcome::Text("done".into()))
    });
    let at = Instant::now();
    s.poll_auto_fetch_at(at);
    assert!(!s.auto_fetch.running(&a.repo));
    assert!(s.auto_fetch.running(&b.repo));
    finish(&mut s, &b.repo, at);
    release.send(()).unwrap();
    wait(&mut s);
    s.poll_auto_fetch_at(at);
    assert!(s.auto_fetch.running(&a.repo));
    finish(&mut s, &a.repo, at);
    s.shutdown();
}
