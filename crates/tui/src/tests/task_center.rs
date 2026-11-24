use super::*;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use engine::tasks::{Outcome as Completion, Retry, Run, State};
fn synchronize(s: &mut Screen, condition: impl Fn(&Screen) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !condition(s) {
        assert!(Instant::now() < deadline, "task cache did not synchronize");
        s.tasks_refresh_for_test();
        s.poll_tasks();
        thread::sleep(Duration::from_millis(5));
    }
}
fn present_tasks(s: &mut Screen, width: u16, height: u16) -> String {
    let text = render(s, width, height);
    s.acknowledge_task_frame();
    text
}
#[test]
fn narrow_list_marks_the_failure_read_after_showing_details_and_keeps_the_command_draft() {
    let (_home, mut s) = fixture(vec![]);
    let run = Run::begin(
        &s.service.store,
        crate::task_center::spec("failed command", None, Retry::Unavailable),
        &Cancel::default(),
    )
    .unwrap();
    let id = run.record.id.clone();
    run.finish(Completion::failure("first\n完整错误\nlast"))
        .unwrap();
    synchronize(&mut s, |s| s.tasks.unread_failures == 1);
    s.key(
        KeyEvent::new(KeyCode::Enter, KeyModifiers::ALT),
        &Cancel::default(),
    )
    .unwrap();
    assert!(!s.tasks.session.open());
    s.paste("/settings");
    let draft = s.bar.text();
    s.open_tasks();
    assert!(s.tasks.session.open());
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        for (w, h) in [(99, 30), (60, 20), (24, 8), (1, 1), (0, 0)] {
            present_tasks(&mut s, w, h);
        }
    }
    assert!(s.service.store.task(&id).unwrap().unwrap().unread_failure());
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    let mut assert_summary_color = |s: &mut Screen, color| {
        terminal.draw(|frame| view::render(frame, s)).unwrap();
        let row = s.tasks.rows[0].0;
        let buffer = terminal.backend().buffer();
        for (offset, symbol) in ["╰", "─", "f"].into_iter().enumerate() {
            let cell = &buffer[(row.x + offset as u16, row.y + 1)];
            assert_eq!(cell.symbol(), symbol);
            assert_eq!(cell.fg, color);
        }
    };
    assert_summary_color(&mut s, ratatui::style::Color::LightRed);
    key(&mut s, KeyCode::Enter);
    assert_eq!(s.tasks.unread_failures, 1);
    let text = present_tasks(&mut s, 80, 24);
    synchronize(&mut s, |s| s.tasks.unread_failures == 0);
    assert!(text.contains("完整错误"));
    assert!(s.service.store.task(&id).unwrap().unwrap().pending());
    s.paste("do not replace my draft");
    key(&mut s, KeyCode::Esc);
    assert_summary_color(&mut s, view::MUTED);
    let buffer = terminal.backend().buffer();
    let row = s.tasks.rows[0].0;
    assert_eq!(buffer[(row.x - 1, row.y)].symbol(), " ");
    let status = &buffer[(row.x, row.y)];
    assert_eq!(status.symbol(), "×");
    assert_eq!(status.fg, ratatui::style::Color::LightRed);
    key(&mut s, KeyCode::Esc);
    assert_eq!(s.bar.text(), draft);
    let record = s.service.store.task(&id).unwrap().unwrap();
    assert!(record.pending());
    assert!(!record.unread_failure());
    s.shutdown();
}
#[test]
fn wide_list_marks_only_the_visible_failure_read_and_keeps_navigation_in_order() {
    let (_home, mut s) = fixture(vec![]);
    let mut ids = Vec::new();
    for (title, result) in [
        ("older", Completion::failure("older error body")),
        (
            "success",
            Completion::Succeeded {
                detail: String::new(),
            },
        ),
        ("newest", Completion::failure("newest error body")),
    ] {
        let run = Run::begin(
            &s.service.store,
            crate::task_center::spec(title, None, Retry::Unavailable),
            &Cancel::default(),
        )
        .unwrap();
        ids.push(run.record.id.clone());
        run.finish(result).unwrap();
    }
    synchronize(&mut s, |s| s.tasks.unread_failures == 2);
    s.open_tasks();
    render(&mut s, 100, 7);
    assert!(s.tasks.visible_unread_failure.is_none());
    s.acknowledge_task_frame();
    assert_eq!(s.tasks.unread_failures, 2);
    let preview = present_tasks(&mut s, 100, 24);
    assert_eq!(preview.matches("newest error body").count(), 2, "{preview}");
    synchronize(&mut s, |s| s.tasks.unread_failures == 1);
    assert_eq!(s.tasks.selection.as_ref(), Some(&ids[2]));
    assert!(
        s.service
            .store
            .task(&ids[0])
            .unwrap()
            .unwrap()
            .unread_failure()
    );
    key(&mut s, KeyCode::Down);
    assert_eq!(s.tasks.selection.as_ref(), Some(&ids[1]));
    key(&mut s, KeyCode::Down);
    assert_eq!(s.tasks.selection.as_ref(), Some(&ids[0]));
    present_tasks(&mut s, 100, 24);
    synchronize(&mut s, |s| s.tasks.unread_failures == 0);
    assert!(matches!(s.tasks.session, crate::task_center::Session::List));
    s.shutdown();
}
#[test]
fn task_filter_keeps_the_underlying_draft_and_limits_navigation_to_visible_records() {
    let _language = i18n::using(i18n::Language::English);
    let (_home, mut s) = fixture(vec![]);
    let mut ids = Vec::new();
    for (title, path, outcome) in [
        (
            "refresh alpha",
            fixture_path!("/example/alpha"),
            Completion::failure("alpha error"),
        ),
        (
            "check beta",
            fixture_path!("/example/beta"),
            Completion::failure("beta error"),
        ),
        (
            "successful task",
            fixture_path!("/example/beta"),
            Completion::Succeeded {
                detail: String::new(),
            },
        ),
    ] {
        let run = Run::begin(
            &s.service.store,
            crate::task_center::spec(title, Some(path.into()), Retry::Unavailable),
            &Cancel::default(),
        )
        .unwrap();
        ids.push(run.record.id.clone());
        run.finish(outcome).unwrap();
    }
    synchronize(&mut s, |s| s.tasks.records.len() == 3);
    s.paste("/settings");
    let original = s.bar.text();
    s.open_tasks();
    assert!(render(&mut s, 110, 26).contains("Filter tasks"));
    key(&mut s, KeyCode::Char('r'));
    s.paste("efresh alpha");
    assert_eq!(s.tasks.search.text, "refresh alpha");
    assert_eq!(s.tasks.selection.as_ref(), Some(&ids[0]));
    assert!(render(&mut s, 110, 26).contains("refresh alpha"));
    assert_eq!(s.tasks.rows.len(), 1);
    key(&mut s, KeyCode::Down);
    assert_eq!(s.tasks.selection.as_ref(), Some(&ids[0]));
    key(&mut s, KeyCode::Enter);
    key(&mut s, KeyCode::Esc);
    assert_eq!(s.tasks.search.text, "refresh alpha");
    key(&mut s, KeyCode::Esc);
    s.paste("BETA");
    render(&mut s, 110, 26);
    assert_eq!(s.tasks.rows.len(), 2);
    key(&mut s, KeyCode::Esc);
    s.paste("failed");
    render(&mut s, 110, 26);
    assert_eq!(s.tasks.rows.len(), 2);
    s.paste(" none");
    assert!(present_tasks(&mut s, 110, 26).contains("No matching tasks"));
    let run = Run::begin(
        &s.service.store,
        crate::task_center::spec("new task", None, Retry::Unavailable),
        &Cancel::default(),
    )
    .unwrap();
    run.finish(Completion::failure("later")).unwrap();
    synchronize(&mut s, |s| s.tasks.records.len() == 4);
    assert!(s.tasks.selection.is_none());
    s.key(
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
        &Cancel::default(),
    )
    .unwrap();
    assert!(s.tasks.search.text.is_empty());
    assert!(s.exit_sequence.notice().is_none());
    assert!(s.tasks.session.open());
    key(&mut s, KeyCode::Esc);
    assert_eq!(s.bar.text(), original);
    s.shutdown();
}

#[test]
fn task_status_filter_uses_the_option_menu_and_combines_with_repository_search() {
    use crate::task_center::Filter;
    let _language = i18n::using(i18n::Language::English);
    let (_home, mut s) = fixture(vec![]);
    for (title, path, outcome) in [
        (
            "failed alpha",
            fixture_path!("/example/alpha"),
            Completion::failure("alpha failure"),
        ),
        (
            "failed beta",
            fixture_path!("/example/beta"),
            Completion::failure("beta failure"),
        ),
        (
            "successful alpha",
            fixture_path!("/example/alpha"),
            Completion::Succeeded {
                detail: String::new(),
            },
        ),
    ] {
        Run::begin(
            &s.service.store,
            crate::task_center::spec(title, Some(path.into()), Retry::Unavailable),
            &Cancel::default(),
        )
        .unwrap()
        .finish(outcome)
        .unwrap();
    }
    synchronize(&mut s, |s| s.tasks.records.len() == 3);
    s.open_tasks();
    s.paste("alpha");
    let heading = render(&mut s, 90, 24);
    let title = heading.lines().nth(1).unwrap();
    assert!(title.starts_with("┌Tasks"), "{title}");
    assert!(title.ends_with("Filter: All tasks ┐"), "{title}");
    let (left, _) = title.split_once("Filter: All tasks").unwrap();
    let column = unicode_width::UnicodeWidthStr::width(left) as u16;
    let click = |s: &mut Screen, column, row| {
        s.mouse(
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column,
                row,
                modifiers: KeyModifiers::NONE,
            },
            &Cancel::default(),
        )
        .unwrap();
    };
    s.config.mouse.precise_click = false;
    click(&mut s, column, 1);
    assert_eq!(s.active_bar().text(), "alpha");
    s.config.mouse.precise_click = true;
    click(&mut s, column, 1);
    let menu = render(&mut s, 90, 24);
    for option in ["all", "unread", "failed", "running", "success", "cancelled"] {
        assert!(menu.contains(option), "{menu}");
    }
    let bar::InputBar::Command {
        menu: command::completion::Menu::Options(options),
        ..
    } = s.active_bar()
    else {
        panic!("task filter options should open on click");
    };
    let failed = options
        .options
        .iter()
        .position(|option| option.token == "failed")
        .unwrap();
    let &(row, _) = s
        .command_items
        .iter()
        .find(|(_, index)| *index == failed)
        .unwrap();
    click(&mut s, row.x, row.y);
    assert_eq!(s.tasks.filter, Filter::Failed);
    assert_eq!(s.active_bar().text(), "alpha");
    let text = render(&mut s, 90, 24);
    assert!(
        text.contains("Filter: Failed") && text.contains("failed alpha"),
        "{text}"
    );
    assert!(
        !text.contains("failed beta") && !text.contains("successful alpha"),
        "{text}"
    );

    key(&mut s, KeyCode::Esc);
    key(&mut s, KeyCode::Esc);
    s.open_tasks();
    assert_eq!(s.tasks.filter, Filter::Failed);
    key(&mut s, KeyCode::Char('/'));
    s.paste("filter running");
    key(&mut s, KeyCode::Enter);
    assert_eq!(s.tasks.filter, Filter::Active);
    assert!(s.tasks.selection.is_none());
    assert!(render(&mut s, 90, 24).contains("No matching tasks"));
    key(&mut s, KeyCode::Char('/'));
    s.paste("filter all");
    key(&mut s, KeyCode::Enter);
    assert_eq!(s.tasks.visible_indices().len(), 3);
    s.shutdown();
}

#[test]
fn unread_task_filter_keeps_the_visible_failure_after_the_preview_marks_it_read() {
    let (_home, mut s) = fixture(vec![]);
    for title in ["first failure", "second failure"] {
        Run::begin(
            &s.service.store,
            crate::task_center::spec(title, None, Retry::Unavailable),
            &Cancel::default(),
        )
        .unwrap()
        .finish(Completion::failure("error detail"))
        .unwrap();
    }
    synchronize(&mut s, |s| s.tasks.unread_failures == 2);
    s.open_tasks();
    key(&mut s, KeyCode::Char('/'));
    s.paste("filter unread");
    key(&mut s, KeyCode::Enter);
    let first = s.tasks.selection.clone().unwrap();
    present_tasks(&mut s, 110, 24);
    synchronize(&mut s, |s| s.tasks.unread_failures == 1);
    present_tasks(&mut s, 110, 24);
    assert_eq!(s.tasks.selection.as_deref(), Some(first.as_str()));
    assert_eq!(s.tasks.visible_indices().len(), 2);
    key(&mut s, KeyCode::Down);
    render(&mut s, 110, 24);
    assert_ne!(s.tasks.selection.as_deref(), Some(first.as_str()));
    assert_eq!(s.tasks.visible_indices().len(), 1);
    assert_eq!(s.tasks.unread_failures, 1);
    s.shutdown();
}

#[test]
fn task_view_uses_its_own_scoped_commands_and_preserves_the_filter_and_origin() {
    let (_home, mut s) = fixture(vec![]);
    let run = Run::begin(
        &s.service.store,
        crate::task_center::spec("target job", None, Retry::FetchAll),
        &Cancel::default(),
    )
    .unwrap();
    run.finish(Completion::failure("reason")).unwrap();
    synchronize(&mut s, |s| s.tasks.records.len() == 1);

    s.paste("/settings");
    let original = s.bar.text();
    s.open_tasks();
    s.paste("target");
    key(&mut s, KeyCode::Char('/'));
    let text = render(&mut s, 110, 26);
    assert!(text.contains("/tasks"), "{text}");
    assert!(text.contains("target"), "{text}");
    assert!(text.contains("cancel") && text.contains("retry"), "{text}");

    key(&mut s, KeyCode::Esc);
    assert_eq!(s.active_bar().text(), "target");
    key(&mut s, KeyCode::Enter);
    key(&mut s, KeyCode::Char('/'));
    assert_eq!(s.active_bar().text(), "/tasks ");
    s.paste("retry");
    key(&mut s, KeyCode::Enter);
    assert!(s.check_task.is_some());
    assert!(matches!(
        s.tasks.session,
        crate::task_center::Session::Detail { .. }
    ));
    key(&mut s, KeyCode::Esc);
    assert_eq!(s.active_bar().text(), "target");
    key(&mut s, KeyCode::Esc);
    key(&mut s, KeyCode::Esc);
    assert_eq!(s.bar.text(), original);
    s.shutdown();
}

#[test]
fn task_view_does_not_stop_the_underlying_worker_and_can_cancel_it() {
    let (_home, mut s) = fixture(vec![]);
    s.start("long task", Effect::ReadOnly, |_, cancel, _| {
        while !cancel.is_cancelled() {
            thread::sleep(Duration::from_millis(5));
        }
        cancel.check()?;
        Ok(Outcome::Text("done".into()))
    });
    synchronize(&mut s, |s| s.tasks.records.iter().any(|r| r.active()));
    s.open_tasks();
    s.paste("//settings");
    key(&mut s, KeyCode::Enter);
    assert!(matches!(s.mode, Mode::Busy(_)));
    assert!(s.tasks.session.open());
    assert!(s.message.contains(i18n::tr!(TaskCommandBusy)));
    key(&mut s, KeyCode::Esc);
    key(&mut s, KeyCode::Char('/'));
    s.paste("cancel");
    key(&mut s, KeyCode::Enter);
    let deadline = Instant::now() + Duration::from_secs(5);
    while matches!(s.mode, Mode::Busy(_)) {
        assert!(Instant::now() < deadline);
        s.poll(&Cancel::default());
        thread::sleep(Duration::from_millis(5));
    }
    assert!(s.tasks.session.open());
    synchronize(&mut s, |s| {
        s.tasks.records.iter().any(|r| {
            matches!(
                r.state,
                State::Finished {
                    outcome: Completion::Cancelled { .. },
                    ..
                }
            )
        })
    });
    assert!(s.tasks.records.iter().all(|r| !r.pending()));
    s.shutdown();
}
#[test]
fn footer_stays_one_row_and_unread_failure_opens_tasks() {
    let (_home, mut s) = fixture(vec![]);
    let empty = render(&mut s, 80, 25);
    let area = s.input_area;
    s.message = "success\nextra output that belongs in task details".into();
    s.poll_tasks();
    let text = render(&mut s, 80, 25);
    assert_eq!(s.input_area, area);
    assert!(text.lines().last().unwrap().contains("success"));
    assert!(!text.contains("extra output"));
    assert_eq!(empty.lines().count(), text.lines().count());
    let run = Run::begin(
        &s.service.store,
        crate::task_center::spec("broken", None, Retry::FetchAll),
        &Cancel::default(),
    )
    .unwrap();
    run.finish(Completion::failure("reason")).unwrap();
    synchronize(&mut s, |s| s.tasks.unread_failures == 1);
    let text = render(&mut s, 80, 25);
    assert!(
        text.lines()
            .last()
            .unwrap()
            .contains(i18n::tr!(TasksOpenCommand)),
        "{text}"
    );
    assert!(!text.contains("Alt+Enter"));
    let footer = s.tasks.footer;
    s.mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: footer.x,
            row: footer.y,
            modifiers: KeyModifiers::NONE,
        },
        &Cancel::default(),
    )
    .unwrap();
    assert!(s.tasks.session.open());
    let text = render(&mut s, 100, 25);
    assert!(!text.contains(" · d "), "{text}");
    assert_eq!(s.tasks.unread_failures, 1);
    s.shutdown();
}
#[test]
fn command_opens_tasks_and_background_errors_remain_queryable_after_leaving() {
    let (_home, mut s) = fixture(vec![]);
    s.start("failing command", Effect::ReadOnly, |_, _, _| {
        anyhow::bail!("long error\nsecond line")
    });
    wait(&mut s);
    synchronize(&mut s, |s| !s.tasks.records.is_empty());
    command(&mut s, "tasks");
    assert!(s.tasks.session.open());
    assert!(render(&mut s, 110, 26).contains("second line"));
    key(&mut s, KeyCode::Esc);
    assert!(matches!(s.mode, Mode::Error { .. }));
    s.shutdown();
}
#[test]
fn retry_keeps_the_original_failure_unread_until_its_details_are_opened() {
    let (_home, mut s) = fixture(vec![]);
    let mut spec = crate::task_center::spec("failed check", None, Retry::FetchAll);
    spec.scope = engine::tasks::Scope::All;
    let run = Run::begin(&s.service.store, spec, &Cancel::default()).unwrap();
    let original = run.record.id.clone();
    run.finish(Completion::failure("old failure")).unwrap();
    synchronize(&mut s, |s| s.tasks.unread_failures == 1);
    s.open_tasks();
    s.key(
        KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL),
        &Cancel::default(),
    )
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while s.check_task.is_some() {
        assert!(Instant::now() < deadline);
        s.poll_check();
        thread::sleep(Duration::from_millis(5));
    }
    synchronize(&mut s, |s| {
        s.tasks.records.len() == 2 && s.tasks.unread_failures == 1
    });
    let original = s.service.store.task(&original).unwrap().unwrap();
    assert!(!original.pending());
    assert!(original.unread_failure());
    assert!(s.tasks.session.open());
    s.shutdown();
}
#[test]
fn task_mouse_selection_double_click_and_preferences_match_other_lists() {
    let (_home, mut s) = fixture(vec![]);
    for title in ["first", "second"] {
        Run::begin(
            &s.service.store,
            crate::task_center::spec(title, None, Retry::Unavailable),
            &Cancel::default(),
        )
        .unwrap()
        .finish(Completion::failure(title))
        .unwrap();
    }
    synchronize(&mut s, |s| s.tasks.unread_failures == 2);
    s.open_tasks();
    render(&mut s, 80, 25);
    let (area, id) = s.tasks.rows[1].clone();
    let event = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: area.x + 2,
        row: area.y,
        modifiers: KeyModifiers::NONE,
    };
    s.config.mouse.precise_click = false;
    s.mouse(event, &Cancel::default()).unwrap();
    assert_ne!(s.tasks.selection.as_ref(), Some(&id));
    s.config.mouse.precise_click = true;
    s.mouse(event, &Cancel::default()).unwrap();
    assert_eq!(s.tasks.selection.as_ref(), Some(&id));
    assert!(matches!(s.tasks.session, crate::task_center::Session::List));
    assert!(s.service.store.task(&id).unwrap().unwrap().unread_failure());
    s.mouse(event, &Cancel::default()).unwrap();
    assert!(
        matches!(&s.tasks.session,crate::task_center::Session::Detail{id:selected,..} if selected==&id)
    );
    present_tasks(&mut s, 80, 25);
    synchronize(&mut s, |s| s.tasks.unread_failures == 1);
    assert!(!s.service.store.task(&id).unwrap().unwrap().unread_failure());
    s.shutdown();
}
fn presentation_record(id: usize, pending: bool) -> engine::tasks::Record {
    use engine::tasks::{Record, Scope, Source, Spec};
    Record {
        stage_timings: vec![],
        id: format!("presentation-{id}"),
        spec: Spec {
            title: "Fetch remote".into(),
            scope: Scope::Repository {
                path: format!("{}/中文-repo-{id}", fixture_path!("/example")).into(),
            },
            source: Source::Interactive,
            retry: Retry::Unavailable,
            retry_of: None,
            operation: None,
        },
        started_at: "2026-10-24T09:30:00-05:00".into(),
        state: State::Finished {
            at: "2026-10-24T09:31:00-05:00".into(),
            outcome: if pending {
                Completion::failure("Authentication failed\n请重新登录后重试\nextra error detail")
            } else {
                Completion::Succeeded {
                    detail: "Fetched refs".into(),
                }
            },
        },
    }
}
#[test]
fn task_list_shows_local_time_and_failure_lines_share_their_task_hit_region() {
    let expected = chrono::DateTime::parse_from_rfc3339("2026-10-24T09:30:00-05:00")
        .unwrap()
        .with_timezone(&chrono::Local)
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec![]);
        s.tasks.records = vec![presentation_record(0, true), presentation_record(1, false)];
        s.tasks.selection = Some(s.tasks.records[0].id.clone());
        s.open_tasks();
        for width in [60, 110, 180] {
            let text = render(&mut s, width, 26);
            let (first, id) = s.tasks.rows[0].clone();
            let (second, _) = s.tasks.rows[1].clone();
            assert_eq!(first.height, 2, "{text}");
            assert_eq!(second.y, first.bottom(), "{text}");
            let reason = text.lines().nth(usize::from(first.y + 1)).unwrap();
            assert!(reason.contains("╰─"), "{text}");
            assert!(reason.contains('…'), "{text}");
            let line = text.lines().nth(first.y.into()).unwrap();
            assert!(line.contains(&expected), "{text}");
            assert!(
                line.trim_start_matches([' ', '│'])
                    .starts_with("•× 中文-repo-0")
            );
            let line = text.lines().nth(second.y.into()).unwrap();
            assert!(
                line.trim_start_matches([' ', '│'])
                    .starts_with("✓ 中文-repo-1")
            );
            assert!(
                text.lines()
                    .nth(usize::from(first.y + 1))
                    .unwrap()
                    .contains("Authentication failed"),
                "{text}"
            );
            key(&mut s, KeyCode::Down);
            s.mouse(
                MouseEvent {
                    kind: MouseEventKind::Down(MouseButton::Left),
                    column: first.x + 3,
                    row: first.y + 1,
                    modifiers: KeyModifiers::NONE,
                },
                &Cancel::default(),
            )
            .unwrap();
            assert_eq!(s.tasks.selection.as_ref(), Some(&id));
        }
        for (width, height) in [(40, 20), (24, 8), (8, 4), (1, 1), (0, 0)] {
            render(&mut s, width, height);
        }
        s.shutdown();
    }
}

#[test]
fn current_and_legacy_successful_pushes_show_the_same_readable_details() {
    use engine::domain::{Publication, PushTarget, RemoteTip};

    let _language = i18n::using(i18n::Language::Chinese);
    let (_home, mut s) = fixture(vec![]);
    let publication = Publication::Pushed {
        target: PushTarget {
            branch: "feature/\"demo\"".into(),
            local_ref: "refs/heads/feature/\"demo\"".into(),
            local_oid: "6249075da20e93a11c87c07da098bb5445993a07".into(),
            remote: "origin".into(),
            remote_ref: "refs/heads/release".into(),
            remote_oid: RemoteTip::Existing("15e9af29988a2203fe20b1da730b9394dac630a8".into()),
            ahead_commits: 5,
        },
    };
    let legacy = Completion::Succeeded {
        detail: format!("{publication:#?}"),
    };
    let cancel = Cancel::default();
    let current = crate::task_center::outcome(
        &Ok(Outcome::Published(publication)),
        &cancel,
        i18n::tr!(PushCurrentBranch),
        Effect::Push,
    );

    for outcome in [legacy, current] {
        let mut record = presentation_record(0, false);
        record.spec.title = i18n::tr!(PushCurrentBranch).into();
        let State::Finished { outcome: saved, .. } = &mut record.state else {
            unreachable!()
        };
        *saved = outcome;
        s.tasks.selection = Some(record.id.clone());
        s.tasks.records = vec![record];
        s.open_tasks();
        for width in [70, 160] {
            if width < 100 {
                key(&mut s, KeyCode::Enter);
            } else {
                key(&mut s, KeyCode::Esc);
            }
            let details = render(&mut s, width, 26);
            assert!(
                details.contains("feature/\"demo\" → origin/release"),
                "{details}"
            );
            assert!(details.contains("已推送（5 个提交）"), "{details}");
            assert!(details.contains("推送后提交: 6249075da20e"), "{details}");
            assert!(!details.contains("PushTarget") && !details.contains("ahead_commits"));
        }
        s.close_tasks();
    }
    s.shutdown();
}

#[test]
fn stage_timings_have_a_separate_detail_section_and_never_replace_failure_previews() {
    use engine::tasks::progress::{Measurement, Stage};
    for language in [i18n::Language::Chinese, i18n::Language::English] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec![]);
        let mut record = presentation_record(0, true);
        record.stage_timings = vec![Measurement {
            stage: Stage::FetchingRemote,
            elapsed_ms: 1234,
        }];
        let id = record.id.clone();
        s.tasks.records = vec![record];
        s.tasks.selection = Some(id.clone());
        s.tasks.session = crate::task_center::Session::List;
        let list = render(&mut s, 60, 28);
        assert!(list.contains("Authentication failed"), "{list}");
        assert!(!list.contains("请重新登录后重试"), "{list}");
        assert!(!list.contains(i18n::tr!(ProgressTimings)), "{list}");
        s.tasks.session = crate::task_center::Session::Detail { id, scroll: 0 };
        let detail = render(&mut s, 110, 34);
        assert!(detail.contains("Authentication failed"), "{detail}");
        assert!(detail.contains("请重新登录后重试"), "{detail}");
        assert!(detail.contains(i18n::tr!(ProgressTimings)), "{detail}");
        assert!(detail.contains(Stage::FetchingRemote.label()), "{detail}");
        assert!(detail.contains("1.234 s"), "{detail}");
        s.shutdown();
    }
}
#[test]
fn batch_failure_details_group_repository_errors_and_keep_the_execution_log() {
    use engine::tasks::{Scope, Source};
    let (_home, mut s) = fixture(vec![]);
    let mut record = presentation_record(0, true);
    record.spec.scope = Scope::All;
    record.spec.source = Source::Scheduled;
    let example = fixture_path!("/example");
    let plan = "获取远程 → 推送提交 → 读取本地状态";
    record.spec.title = format!("定时任务 · {plan}");
    record.state = State::Finished {
        at: record.started_at.clone(),
        outcome: Completion::failure(format!(
            "{example}/AtomNote: Fetch failed\nfatal: could not connect\n\n\n\n\
             {example}/Gift: Fetch failed\nfatal: timeout\n\n\
             {example}/AtomNote: Push failed\n\nadditional stderr context\n\n{plan}\n\
             {example}/Gift\n– Read local state · skipped"
        )),
    };
    let id = record.id.clone();
    s.tasks.selection = Some(id.clone());
    s.tasks.records = vec![record];
    s.tasks.session = crate::task_center::Session::List;
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let preview = render(&mut s, 160, 37);
        let heading = format!("{} · 2", i18n::tr!(TasksFailedRepositories));
        let atom = preview.find("× AtomNote").unwrap();
        let gift = preview.find("× Gift").unwrap();
        let log = preview.find(i18n::tr!(TasksExecutionLog)).unwrap();
        assert!(preview.find(&heading).unwrap() < atom && atom < gift && gift < log);
        assert_eq!(preview.matches("× AtomNote").count(), 1);
        assert!(preview[atom..gift].contains("Push failed"));
        assert!(preview[atom..gift].contains("fatal: could not connect"));
        assert!(preview[atom..gift].contains("additional stderr context"));
        assert!(preview[gift..log].contains("fatal: timeout"));
        assert!(preview[log..].contains("Read local state · skipped"));
    }
    s.tasks.session = crate::task_center::Session::Detail { id, scroll: 0 };
    render(&mut s, 60, 14);
    key(&mut s, KeyCode::End);
    assert!(render(&mut s, 60, 14).contains("Read local state · skipped"));
    let manual_plan = {
        let _language = i18n::using(i18n::Language::English);
        engine::workflow::Plan::fetch().summary()
    };
    s.tasks.records[0].spec.source = Source::Interactive;
    s.tasks.records[0].spec.title = "Fetch all".into();
    s.tasks.records[0].state = State::Finished {
        at: s.tasks.records[0].started_at.clone(),
        outcome: Completion::failure(format!(
            "{example}/AtomNote: Fetch failed\n\n{manual_plan}\n{example}/Healthy\n\
             ✓ 01:02:03 Fetch remotes · origin\n{example}/AtomNote\n✗ 01:02:04 Fetch remotes"
        )),
    };
    s.tasks.session = crate::task_center::Session::List;
    let _language = i18n::using(i18n::Language::Chinese);
    let text = render(&mut s, 160, 34);
    assert!(text.contains("失败仓库 · 1"));
    assert!(text.contains("执行记录"));
    assert!(!text.contains(fixture_path!("/example/Healthy")));
    assert!(text.contains("01:02:04 Fetch remotes"));
    s.shutdown();
}
#[test]
fn legacy_scheduled_detail_keeps_one_copy_of_a_failed_push_and_a_step_timeline() {
    use engine::tasks::{Scope, Source};
    let _language = i18n::using(i18n::Language::Chinese);
    let (_home, mut s) = fixture(vec![]);
    let mut record = presentation_record(0, true);
    record.spec.scope = Scope::All;
    record.spec.source = Source::Scheduled;
    let example = fixture_path!("/example");
    let plan = "获取远程 → 推送提交 → 读取本地状态";
    record.spec.title = format!("定时任务 · {plan}");
    let hook = "Git rejected the branch\n\nPython environment lacks pyright";
    let step = format!("推送提交: {hook}\n\n✗ main → origin:refs/heads/main · {hook}");
    record.state = State::Finished {
        at: record.started_at.clone(),
        outcome: Completion::failure(format!(
            "{example}/rivetoo: {step}\n\n\n{example}/rivetoo: {hook}\n\n\
             {example}/Gift: Fetch failed\n\n{plan}\n{example}/rivetoo\n\
             ✓ 02:44:41 获取远程 · origin\n✗ 02:45:41 推送提交\n  {step}\n\
             – 读取本地状态 · 未执行\n{example}/Gift\n✗ 02:45:42 获取远程\n  Fetch failed"
        )),
    };
    s.tasks.session = crate::task_center::Session::Detail {
        id: record.id.clone(),
        scroll: 0,
    };
    s.tasks.records = vec![record];
    let text = render(&mut s, 100, 42);
    assert!(text.contains("本轮失败仓库 · 2"), "{text}");
    assert_eq!(
        text.matches("Python environment lacks pyright").count(),
        1,
        "{text}"
    );
    assert_eq!(text.matches("Git rejected the branch").count(), 1, "{text}");
    assert_eq!(text.matches("Fetch failed").count(), 1, "{text}");
    assert!(text.contains("main → origin:refs/heads/main"), "{text}");
    assert!(text.contains("02:45:41 推送提交"), "{text}");
    assert!(text.contains("02:45:42 获取远程"), "{text}");
    s.shutdown();
}
#[test]
fn task_details_have_semantic_styles_and_scroll_all_failure_output() {
    use ratatui::style::{Color, Modifier};
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec![]);
        let record = presentation_record(0, true);
        s.tasks.selection = Some(record.id.clone());
        s.tasks.session = crate::task_center::Session::Detail {
            id: record.id.clone(),
            scroll: 0,
        };
        s.tasks.records = vec![record];
        let mut terminal = Terminal::new(TestBackend::new(90, 26)).unwrap();
        terminal.draw(|frame| view::render(frame, &mut s)).unwrap();
        let buffer = terminal.backend().buffer();
        let styled = |symbol: &str, color: Color, bold: bool| {
            buffer.content.iter().any(|cell| {
                cell.symbol() == symbol
                    && cell.fg == color
                    && (!bold || cell.modifier.contains(Modifier::BOLD))
            })
        };
        assert!(styled("F", Color::LightBlue, true));
        assert!(styled(
            if language == i18n::Language::Chinese {
                "来"
            } else {
                "S"
            },
            Color::DarkGray,
            false
        ));
        assert!(styled(
            if language == i18n::Language::Chinese {
                "手"
            } else {
                "I"
            },
            Color::Gray,
            false
        ));
        assert!(styled(
            if language == i18n::Language::Chinese {
                "失"
            } else {
                "F"
            },
            Color::LightRed,
            true
        ));
        let text = render(&mut s, 90, 26);
        assert!(text.contains(i18n::tr!(TasksFailureReason)));
        assert!(!text.contains("Duration") && !text.contains("耗时"));
        if let State::Finished {
            outcome: Completion::Failed { detail, .. },
            ..
        } = &mut s.tasks.records[0].state
        {
            *detail = format!(
                "{}\nFINAL FAILURE DETAIL",
                (0..100)
                    .map(|i| format!("line {i} 中文"))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
        }
        render(&mut s, 40, 15);
        key(&mut s, KeyCode::End);
        assert!(render(&mut s, 40, 15).contains("FINAL FAILURE DETAIL"));
        s.shutdown();
    }
}
#[test]
fn multiline_task_rows_keep_selection_and_clicks_correct_after_scrolling_and_resize() {
    let (_home, mut s) = fixture(vec![]);
    s.tasks.records = (0..35).map(|id| presentation_record(id, id < 10)).collect();
    s.tasks.selection = Some(s.tasks.records[0].id.clone());
    s.open_tasks();
    for width in [80, 24, 110, 60] {
        render(&mut s, width, 18);
        key(&mut s, KeyCode::End);
        render(&mut s, width, 18);
        assert_eq!(s.tasks.selection, Some("presentation-34".into()));
        let (last, id) = s.tasks.rows.last().unwrap().clone();
        assert_eq!(id, "presentation-34");
        assert!(last.bottom() <= s.tasks.content.bottom());
        key(&mut s, KeyCode::Up);
        assert_eq!(s.tasks.selection, Some("presentation-33".into()));
        render(&mut s, width, 18);
        key(&mut s, KeyCode::Home);
        render(&mut s, width, 18);
        assert_eq!(s.tasks.rows[0].1, "presentation-0");
    }
    s.shutdown();
}
