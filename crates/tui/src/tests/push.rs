use super::*;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

#[test]
fn force_push_reviews_counts_and_commits_and_cannot_run_by_accepting_the_default_cancel() {
    let f = support::Fixture::create();
    f.advance();
    f.commit("local", "local");
    let before = support::git(&f.remote, &["rev-parse", "main"]);
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo push");
    wait(&mut s);
    assert!(
        matches!(&s.mode, Mode::PushConfirm { choice: confirmation::Choice::Return, plan, .. } if plan.force_behind() == Some(1))
    );
    for language in [i18n::Language::Chinese, i18n::Language::English] {
        let _language = i18n::using(language);
        for (width, height) in [(60, 18), (100, 28), (180, 36)] {
            let text = render(&mut s, width, height);
            assert!(text.contains(i18n::tr!(PushForceTitle)), "{text}");
            assert!(text.contains(i18n::tr!(PushRemoteTip)), "{text}");
            assert!(text.contains(i18n::tr!(PushNewTip)), "{text}");
        }
    }
    key(&mut s, KeyCode::Enter);
    assert_eq!(before, support::git(&f.remote, &["rev-parse", "main"]));
    command(&mut s, "repo push");
    wait(&mut s);
    key(&mut s, KeyCode::Left);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert!(!matches!(s.mode, Mode::Error { .. }), "{:?}", s.mode);
    assert_eq!(
        support::git(&f.repo, &["rev-parse", "HEAD"]),
        support::git(&f.remote, &["rev-parse", "main"])
    );
    s.shutdown();
}

#[test]
fn first_publication_edits_the_branch_selects_a_remote_and_cancels_without_config_changes() {
    let f = support::Fixture::create();
    support::git(&f.repo, &["branch", "--unset-upstream"]);
    support::git(&f.repo, &["config", "push.autoSetupRemote", "false"]);
    let second = f.temp.path().join("second.git");
    std::fs::create_dir(&second).unwrap();
    support::git(&second, &["init", "--bare"]);
    support::git(
        &f.repo,
        &["remote", "add", "backup", second.to_str().unwrap()],
    );
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo push");
    wait(&mut s);
    assert!(matches!(&s.mode, Mode::PushTarget(session) if session.editor().text == "main"));
    for language in [i18n::Language::Chinese, i18n::Language::English] {
        let _language = i18n::using(language);
        let text = render(&mut s, 100, 28);
        assert!(text.contains(i18n::tr!(PushTargetBranch)), "{text}");
        assert!(text.contains("origin") && text.contains("backup"));
    }
    key(&mut s, KeyCode::Esc);
    assert!(
        engine::local::open(&f.repo)
            .unwrap()
            .config()
            .unwrap()
            .get_string("branch.main.remote")
            .is_err()
    );
    assert!(support::git(&second, &["for-each-ref"]).is_empty());
    command(&mut s, "repo push");
    wait(&mut s);
    // 鼠标与键盘使用同一远程选区；目标名保留中文输入。
    render(&mut s, 100, 28);
    let area = s.body_area.unwrap();
    s.mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: area.x + 2,
            row: area.y + 4,
            modifiers: KeyModifiers::NONE,
        },
        &Cancel::default(),
    )
    .unwrap();
    assert!(matches!(&s.mode, Mode::PushTarget(session) if session.selected == 0));
    s.key(
        KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
        &Cancel::default(),
    )
    .unwrap();
    s.paste("发布/功能");
    key(&mut s, KeyCode::Enter);
    assert!(matches!(&s.mode, Mode::PushTarget(session) if !session.editing()));
    // 准备期间忽略编辑与导航按键，任务仍能正常收尾。
    key(&mut s, KeyCode::Down);
    s.paste("错误的追加");
    wait(&mut s);
    assert!(!matches!(s.mode, Mode::Error { .. }), "{:?}", s.mode);
    assert_eq!(
        support::git(&second, &["rev-parse", "发布/功能"]),
        support::git(&f.repo, &["rev-parse", "HEAD"])
    );
    assert_eq!(
        support::git(&f.repo, &["config", "branch.main.remote"]),
        "backup"
    );
    assert_eq!(
        support::git(&f.repo, &["config", "branch.main.merge"]),
        "refs/heads/发布/功能"
    );
    s.shutdown();
}

#[test]
fn configured_automatic_upstream_does_not_open_a_target_form() {
    let f = support::Fixture::create();
    support::git(&f.repo, &["switch", "-c", "feature"]);
    support::git(&f.repo, &["config", "push.autoSetupRemote", "true"]);
    support::git(&f.repo, &["config", "push.default", "current"]);
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo push");
    wait(&mut s);
    assert!(
        !matches!(
            s.mode,
            Mode::PushTarget(_) | Mode::PushConfirm { .. } | Mode::Error { .. }
        ),
        "{:?}",
        s.mode
    );
    assert_eq!(
        support::git(&f.repo, &["config", "branch.feature.merge"]),
        "refs/heads/feature"
    );
    assert_eq!(
        support::git(&f.remote, &["rev-parse", "feature"]),
        support::git(&f.repo, &["rev-parse", "HEAD"])
    );
    s.shutdown();
}

#[test]
fn invalid_target_can_be_corrected_after_the_error_page() {
    let f = support::Fixture::create();
    support::git(&f.repo, &["branch", "--unset-upstream"]);
    support::git(&f.repo, &["config", "push.autoSetupRemote", "false"]);
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo push");
    wait(&mut s);
    s.key(
        KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
        &Cancel::default(),
    )
    .unwrap();
    s.paste("bad name");
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert!(matches!(s.mode, Mode::Error { .. }));
    key(&mut s, KeyCode::Esc);
    assert!(
        matches!(&s.mode, Mode::PushTarget(session) if session.editing() && session.editor().text == "bad name")
    );
    s.key(
        KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
        &Cancel::default(),
    )
    .unwrap();
    s.paste("fixed");
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert!(!matches!(s.mode, Mode::Error { .. }), "{:?}", s.mode);
    assert_eq!(
        support::git(&f.remote, &["rev-parse", "fixed"]),
        support::git(&f.repo, &["rev-parse", "HEAD"])
    );
    s.shutdown();
}

#[test]
fn pushes_continue_when_navigating_away_but_pending_confirmations_never_claim_to_have_pushed() {
    for needs_confirmation in [false, true] {
        let f = support::Fixture::create();
        if needs_confirmation {
            f.advance();
        }
        f.commit("local", "local");
        let before = support::git(&f.remote, &["rev-parse", "main"]);
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        command(&mut s, "repo push");
        command(&mut s, "help");
        wait(&mut s);
        assert_eq!(s.page, Page::Help);
        let after = support::git(&f.remote, &["rev-parse", "main"]);
        if needs_confirmation {
            assert_eq!(before, after);
            assert_eq!(s.message, i18n::tr!(PushConfirmationNeeded));
        } else {
            assert_eq!(after, support::git(&f.repo, &["rev-parse", "HEAD"]));
        }
        s.shutdown();
    }
}

#[test]
fn remote_selection_keeps_offscreen_choices_visible_in_small_windows() {
    let f = support::Fixture::create();
    support::git(&f.repo, &["branch", "--unset-upstream"]);
    support::git(&f.repo, &["config", "push.autoSetupRemote", "false"]);
    for index in 0..12 {
        support::git(
            &f.repo,
            &[
                "remote",
                "add",
                &format!("z{index:02}"),
                f.remote.to_str().unwrap(),
            ],
        );
    }
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo push");
    wait(&mut s);
    for _ in 0..12 {
        key(&mut s, KeyCode::Down);
    }
    let text = render(&mut s, 60, 18);
    assert!(text.contains("z11"), "{text}");
    key(&mut s, KeyCode::Up);
    let text = render(&mut s, 60, 18);
    assert!(text.contains("z10"), "{text}");
    s.shutdown();
}
