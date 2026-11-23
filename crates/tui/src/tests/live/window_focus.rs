use super::*;
use crate::live::WindowFocus;

#[test]
fn unfocused_start_waits_for_focus_before_the_initial_local_read() {
    let _native = NATIVE.lock().unwrap();
    let f = support::Fixture::create();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    s.live = live::State::start();
    s.window_focus_changed(WindowFocus::Unfocused);
    until(&mut s, |s| s.live.pending() == 1);
    assert_eq!(s.live.reading(), 0);
    assert!(!observed(&s, &f.repo));
    s.window_focus_changed(WindowFocus::Focused);
    until(&mut s, |s| observed(s, &f.repo));
    assert!(s.service.store.tasks(10).unwrap().is_empty());
    s.shutdown();
}

#[test]
fn unfocused_changes_are_coalesced_and_resume_without_replacing_selection_or_input() {
    let _native = NATIVE.lock().unwrap();
    let a = support::Fixture::create();
    let b = support::Fixture::create();
    b.commit("file.ts", "export const value = 1;\n");
    support::git(&b.repo, &["push", "origin", "main"]);
    let (_home, mut s) = fixture(vec![a.repo.clone(), b.repo.clone()]);
    s.live = live::State::start();
    until(&mut s, |s| {
        observed(s, &a.repo) && observed(s, &b.repo) && s.live.reading() == 0
    });
    s.focus(Some(a.repo.clone()));
    s.paste("/help");
    let input = s.bar.text();
    s.window_focus_changed(WindowFocus::Unfocused);
    for value in 2..=8 {
        support::write(
            b.repo.join("file.ts"),
            &format!("export const value = {value};\n"),
        );
    }
    until(&mut s, |s| s.live.pending() == 1);
    assert_eq!(s.live.reading(), 0);
    assert!(matches!(
        s.result_for(&b.repo),
        Some(RepositoryResult::Clean { .. })
    ));
    s.window_focus_changed(WindowFocus::Focused);
    until(&mut s, |s| {
        matches!(
            s.result_for(&b.repo),
            Some(RepositoryResult::Attention { .. })
        ) && s.live.reading() == 0
            && s.live.pending() == 0
    });
    assert!(matches!(
        s.result_for(&a.repo),
        Some(RepositoryResult::Clean { .. })
    ));
    assert_eq!(s.selected_path(), Some(a.repo.clone()));
    assert_eq!(s.bar.text(), input);
    assert!(s.service.store.report().unwrap().is_none());
    assert!(s.service.store.tasks(10).unwrap().is_empty());
    // 重复聚焦事件不应主动让全部仓库失效。
    s.window_focus_changed(WindowFocus::Focused);
    tick(&mut s);
    assert_eq!(s.live.reading(), 0);
    assert_eq!(s.live.pending(), 0);
    s.shutdown();
}

#[test]
fn losing_focus_cancels_a_blocked_auto_reader_and_resumes_it_without_an_error() {
    let _native = NATIVE.lock().unwrap();
    let f = support::Fixture::create();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    let common = engine::local::common_dir(&engine::local::open(&f.repo).unwrap()).unwrap();
    let lock = s
        .service
        .store
        .repository_lock(&common, &Cancel::default())
        .unwrap();
    s.live = live::State::start();
    until(&mut s, |s| s.live.reading() == 1);
    s.window_focus_changed(WindowFocus::Unfocused);
    until(&mut s, |s| s.live.reading() == 0);
    assert_eq!(s.live.pending(), 1);
    assert!(s.live.error().is_none());
    assert!(!observed(&s, &f.repo));
    drop(lock);
    s.window_focus_changed(WindowFocus::Focused);
    until(&mut s, |s| observed(s, &f.repo));
    assert!(s.live.error().is_none());
    s.shutdown();
}

#[test]
fn manual_and_startup_fetches_continue_while_local_observation_is_unfocused() {
    let _native = NATIVE.lock().unwrap();
    for startup in [false, true] {
        let f = support::Fixture::create();
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        s.live = live::State::start();
        until(&mut s, |s| observed(s, &f.repo) && s.live.reading() == 0);
        s.window_focus_changed(WindowFocus::Unfocused);
        f.advance();
        let remote = support::git(&f.remote, &["rev-parse", "main"]);
        if startup {
            s.config.fetch_on_startup = true;
            s.service.store.save_config(&s.config).unwrap();
            s.fetch_on_startup();
        } else {
            command(&mut s, "repo fetch");
        }
        let fetching = |s: &Screen| s.check_task.is_some() || !s.repository_tasks.is_empty();
        assert!(fetching(&s), "startup={startup}; {}", s.message);
        // 用实际的获取结果验证失焦行为，等待期限与 Git 子进程一致。
        until_with_timeout(&mut s, engine::transport::Git::default().timeout, |s| {
            !fetching(s)
        });
        assert_eq!(support::git(&f.repo, &["rev-parse", "origin/main"]), remote);
        assert_eq!(s.live.reading(), 0);
        assert!(s.live.error().is_none());
        let records = s.service.store.tasks(10).unwrap();
        assert_eq!(records.len(), 1);
        assert!(matches!(
            records[0].state,
            engine::tasks::State::Finished {
                outcome: engine::tasks::Outcome::Succeeded { .. },
                ..
            }
        ));
        s.shutdown();
    }
}
