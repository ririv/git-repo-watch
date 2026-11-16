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
#[test]
fn task_view_keeps_the_command_draft_and_does_not_acknowledge_on_read() {
    let (_home, mut s) = fixture(vec![]);
    let run = Run::begin(
        &s.service.store,
        crate::task_center::spec("failed command", None, Retry::Unavailable),
        &Cancel::default(),
    )
    .unwrap();
    let id = run.record.id.clone();
    run.finish(Completion::Failed {
        detail: "first\n完整错误\nlast".into(),
        resolution: engine::tasks::Resolution::Pending,
    })
    .unwrap();
    synchronize(&mut s, |s| s.tasks.records.len() == 1);
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
        for (w, h) in [(110, 30), (60, 20), (24, 8), (1, 1), (0, 0)] {
            render(&mut s, w, h);
        }
    }
    key(&mut s, KeyCode::Enter);
    let text = render(&mut s, 80, 24);
    assert!(text.contains("完整错误"));
    assert!(s.service.store.task(&id).unwrap().unwrap().pending());
    s.paste("do not replace my draft");
    key(&mut s, KeyCode::Esc);
    key(&mut s, KeyCode::Esc);
    assert_eq!(s.bar.text(), draft);
    assert!(s.service.store.task(&id).unwrap().unwrap().pending());
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
    key(&mut s, KeyCode::Char('c'));
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
fn footer_stays_one_row_and_failure_marker_opens_tasks() {
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
        crate::task_center::spec("broken", None, Retry::Unavailable),
        &Cancel::default(),
    )
    .unwrap();
    run.finish(Completion::Failed {
        detail: "reason".into(),
        resolution: engine::tasks::Resolution::Pending,
    })
    .unwrap();
    synchronize(&mut s, |s| s.tasks.records.len() == 1);
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
    key(&mut s, KeyCode::Char('d'));
    synchronize(&mut s, |s| s.tasks.records.iter().all(|r| !r.pending()));
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
fn retry_uses_a_new_check_and_clears_only_the_successfully_retried_failure() {
    let (_home, mut s) = fixture(vec![]);
    let mut spec = crate::task_center::spec("failed check", None, Retry::FetchAll);
    spec.scope = engine::tasks::Scope::All;
    let run = Run::begin(&s.service.store, spec, &Cancel::default()).unwrap();
    let original = run.record.id.clone();
    run.finish(Completion::Failed {
        detail: "old failure".into(),
        resolution: engine::tasks::Resolution::Pending,
    })
    .unwrap();
    synchronize(&mut s, |s| s.tasks.records.len() == 1);
    s.open_tasks();
    key(&mut s, KeyCode::Char('r'));
    let deadline = Instant::now() + Duration::from_secs(5);
    while s.check_task.is_some() {
        assert!(Instant::now() < deadline);
        s.poll_check();
        thread::sleep(Duration::from_millis(5));
    }
    synchronize(&mut s, |s| s.tasks.records.len() == 2);
    assert!(!s.service.store.task(&original).unwrap().unwrap().pending());
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
        .finish(Completion::Succeeded {
            detail: title.into(),
        })
        .unwrap();
    }
    synchronize(&mut s, |s| s.tasks.records.len() == 2);
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
    s.mouse(event, &Cancel::default()).unwrap();
    assert!(
        matches!(&s.tasks.session,crate::task_center::Session::Detail{id:selected,..} if selected==&id)
    );
    s.shutdown();
}
fn presentation_record(id: usize, pending: bool) -> engine::tasks::Record {
    use engine::tasks::{Record, Resolution, Scope, Source, Spec};
    Record {
        stage_timings: vec![],
        id: format!("presentation-{id}"),
        spec: Spec {
            title: "Fetch remote".into(),
            scope: Scope::Repository {
                path: format!("/example/中文-repo-{id}").into(),
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
                Completion::Failed {
                    detail: "Authentication failed\n请重新登录后重试\nextra error detail".into(),
                    resolution: Resolution::Pending,
                }
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
                "待"
            } else {
                "N"
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
