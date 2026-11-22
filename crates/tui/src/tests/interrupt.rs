use super::*;
use crossterm::event::{KeyEventKind, MouseButton, MouseEvent, MouseEventKind};

fn interrupt(s: &mut Screen) -> bool {
    s.key(
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
        &Cancel::default(),
    )
    .unwrap()
}

fn warn(s: &mut Screen) {
    assert!(!interrupt(s));
    assert_eq!(
        s.exit_sequence.notice(),
        Some(i18n::tr!(PressCtrlCAgainToQuit))
    );
}

#[test]
fn three_presses_clear_all_command_input_warn_then_exit_without_writes() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec!["/example/alpha".into(), "/example/beta".into()]);
        let config = std::fs::read(s.service.store.config_path()).unwrap();
        let selected = s.selected_path();
        s.paste("beta");
        assert_eq!(s.selected_path(), Some(PathBuf::from("/example/beta")));
        s.paste("//settings concurrency ");
        assert!(matches!(s.bar, bar::InputBar::Command { .. }));
        assert!(!interrupt(&mut s));
        assert!(s.bar.text().is_empty());
        assert!(s.query.is_empty());
        assert_eq!(s.page, Page::Repositories);
        assert_eq!(s.selected_path(), selected);
        assert!(s.exit_sequence.notice().is_none());
        assert!(!interrupt(&mut s));
        for (width, height) in [(60, 8), (60, 18), (120, 34)] {
            let text = render(&mut s, width, height);
            assert!(text.contains(i18n::tr!(PressCtrlCAgainToQuit)), "{text}");
        }
        assert!(interrupt(&mut s));
        assert_eq!(
            std::fs::read(s.service.store.config_path()).unwrap(),
            config
        );
        assert!(s.repository_tasks.is_empty() && s.check_task.is_none());
        s.shutdown();
    }
}

#[test]
fn empty_input_warns_immediately_and_command_prefixes_are_cleared_first() {
    for entry in [
        None,
        Some("settings"),
        #[cfg(feature = "developer-scheduler")]
        Some("schedule"),
        Some("help"),
    ] {
        for draft in [false, true] {
            let (_home, mut s) = fixture(vec![]);
            if let Some(entry) = entry {
                command(&mut s, entry);
            }
            if draft {
                key(&mut s, KeyCode::Char('/'));
                assert!(!s.bar.text().is_empty());
                assert!(!interrupt(&mut s));
                assert!(s.bar.text().is_empty());
                assert!(s.exit_sequence.notice().is_none());
            }
            let page = s.page;
            let depth = s.history.len();
            warn(&mut s);
            assert!(s.bar.text().is_empty());
            assert_eq!(s.page, page);
            assert_eq!(s.history.len(), depth);
            assert!(interrupt(&mut s));
            s.shutdown();
        }
    }
}

#[test]
fn keyboard_paste_click_and_wheel_reset_the_exit_sequence() {
    for action in 0..5 {
        let (_home, mut s) = fixture(vec!["/example/alpha".into(), "/example/beta".into()]);
        warn(&mut s);
        render(&mut s, 120, 32);
        match action {
            0 => assert!(!key(&mut s, KeyCode::Down)),
            1 => assert!(!key(&mut s, KeyCode::Esc)),
            2 => {
                s.paste("alpha");
                assert!(s.exit_sequence.notice().is_none());
                assert!(!interrupt(&mut s));
                assert!(s.bar.text().is_empty());
            }
            _ => {
                let area = s.repository_area.unwrap();
                assert!(
                    s.mouse(
                        MouseEvent {
                            kind: if action == 3 {
                                MouseEventKind::Down(MouseButton::Left)
                            } else {
                                MouseEventKind::ScrollDown
                            },
                            column: area.x + 3,
                            row: area.y + 3,
                            modifiers: KeyModifiers::NONE,
                        },
                        &Cancel::default(),
                    )
                    .unwrap()
                );
            }
        }
        assert!(s.exit_sequence.notice().is_none());
        warn(&mut s);
        assert!(interrupt(&mut s));
        s.shutdown();
    }
}

#[test]
fn repeats_and_releases_do_not_advance_or_reset_exit_steps() {
    let (_home, mut s) = fixture(vec![]);
    assert!(!interrupt(&mut s));
    for kind in [KeyEventKind::Repeat, KeyEventKind::Release] {
        for _ in 0..3 {
            assert!(
                !s.key(
                    KeyEvent::new_with_kind(KeyCode::Char('c'), KeyModifiers::CONTROL, kind),
                    &Cancel::default(),
                )
                .unwrap()
            );
        }
    }
    assert!(s.exit_sequence.notice().is_some());
    assert!(
        !s.mouse(
            MouseEvent {
                kind: MouseEventKind::Moved,
                column: 0,
                row: 0,
                modifiers: KeyModifiers::NONE,
            },
            &Cancel::default(),
        )
        .unwrap()
    );
    assert!(interrupt(&mut s));
    s.shutdown();
}

#[test]
fn exit_prompt_survives_background_feedback_and_is_visible_on_task_pages() {
    let (_home, mut s) = fixture(vec![]);
    s.paste("hidden search");
    s.tasks.session = crate::task_center::Session::List;
    warn(&mut s);
    assert_eq!(s.bar.text(), "hidden search");
    s.tasks.error = Some("task storage issue".into());
    s.message = "background operation finished".into();
    let text = render(&mut s, 120, 32);
    assert!(text.contains(i18n::tr!(PressCtrlCAgainToQuit)), "{text}");
    key(&mut s, KeyCode::Esc);
    assert!(s.exit_sequence.notice().is_none());
    assert_eq!(s.bar.text(), "hidden search");
    assert!(!interrupt(&mut s));
    assert!(s.bar.text().is_empty());
    assert!(s.exit_sequence.notice().is_none());
    warn(&mut s);
    let text = render(&mut s, 120, 32);
    assert!(text.contains(i18n::tr!(PressCtrlCAgainToQuit)), "{text}");
    assert!(s.tasks.footer.is_empty());
    assert_eq!(s.tasks.error.as_deref(), Some("task storage issue"));
    s.shutdown();
}

#[test]
fn clearing_layout_parameters_keeps_the_diff_view_and_existing_layout() {
    let f = support::Fixture::create();
    support::write(f.repo.join("draft.rs"), "fn draft() {}\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo diff");
    wait(&mut s);
    s.paste("draft");
    command(&mut s, "repo diff layout");
    assert!(matches!(
        s.bar,
        bar::InputBar::Command {
            menu: crate::command::completion::Menu::Options(_),
            ..
        }
    ));
    let page = s.page;
    let depth = s.history.len();
    assert!(!interrupt(&mut s));
    assert_eq!(s.page, page);
    assert_eq!(s.history.len(), depth);
    assert_eq!(s.diff_preference, crate::diff::Preference::Auto);
    assert!(s.mode.file_session().unwrap().query.is_empty());
    assert!(s.bar.text().is_empty());
    s.shutdown();
}

#[test]
fn ctrl_c_does_not_clear_independent_multiline_editors_or_submit_a_commit() {
    let f = support::Fixture::create();
    support::write(f.repo.join("staged.txt"), "staged value\n");
    support::git(&f.repo, &["add", "staged.txt"]);
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    let head = support::git(&f.repo, &["rev-parse", "HEAD"]);
    command(&mut s, "repo commit");
    wait(&mut s);
    s.paste("feat: 标题\n\n完整说明");
    warn(&mut s);
    assert!(
        matches!(&s.mode, Mode::CommitMessage { editor, .. } if editor.text() == "feat: 标题\n\n完整说明")
    );
    assert_eq!(support::git(&f.repo, &["rev-parse", "HEAD"]), head);
    s.mode = Mode::PromptEditor(Box::new(prompt_editor::Session::new(
        &engine::ai::Prompt::Custom {
            text: "保留指令\n第二行".into(),
        },
    )));
    key(&mut s, KeyCode::Home);
    warn(&mut s);
    assert!(
        matches!(&s.mode, Mode::PromptEditor(session) if session.editor.text() == "保留指令\n第二行")
    );
    s.shutdown();
}

#[test]
fn blocking_work_is_cancelled_and_collected_before_ctrl_c_can_request_exit() {
    let (_home, mut s) = fixture(vec![]);
    let (started, ready) = mpsc::channel();
    s.start(
        "isolated cancellation",
        Effect::ReadOnly,
        move |_, cancel, _| {
            started.send(()).unwrap();
            loop {
                cancel.check()?;
                thread::sleep(Duration::from_millis(3));
            }
        },
    );
    ready.recv_timeout(Duration::from_secs(5)).unwrap();
    for _ in 0..3 {
        assert!(!interrupt(&mut s));
        assert!(
            matches!(&s.mode, Mode::Busy(pending) if pending.cancel.is_cancelled() && pending.state == TaskState::Cancelling)
        );
        assert!(s.exit_sequence.notice().is_none());
    }
    wait(&mut s);
    assert!(!matches!(s.mode, Mode::Busy(_)));
    warn(&mut s);
    assert!(interrupt(&mut s));
    s.shutdown();
}

#[test]
fn quit_command_still_exits_immediately() {
    let (_home, mut s) = fixture(vec![]);
    s.paste("//quit");
    assert!(key(&mut s, KeyCode::Enter));
    s.shutdown();
}
