use super::*;
use crate::command::{Context, Input, NamespaceEvent, context::DiffContext};

fn press(input: &mut Input, code: KeyCode) -> NamespaceEvent {
    input.namespace_key(KeyEvent::new(code, KeyModifiers::NONE))
}
fn selected(input: &Input) -> &str {
    let range = input.prefix_selection().expect("selected prefix");
    // 选区只覆盖静态命令名；调用方检查实际文本，因此这里不复制临时 canonical 字符串。
    let name = input.context().unwrap().name();
    if range.start == 0 {
        name
    } else {
        &name[range.start - 1..range.end - 1]
    }
}
fn branches(s: &mut Screen) {
    command(s, "repo branch");
    wait(s);
    s.paste("main");
    key(s, KeyCode::Home);
    key(s, KeyCode::Char('/'));
    key(s, KeyCode::Backspace);
    assert_eq!(selected(s.bar.command_input().unwrap()), "branch");
}
fn type_text(s: &mut Screen, text: &str) {
    for character in text.chars() {
        key(s, KeyCode::Char(character));
    }
}

#[test]
fn every_context_selects_its_last_segment_and_arrows_expand_and_shrink_at_boundaries() {
    for context in [
        Context::Diff(DiffContext::Worktree),
        Context::Diff(DiffContext::History),
        Context::History,
        Context::Branches,
        Context::Stash,
        Context::Files,
        Context::Composition,
        Context::Ai,
        Context::Repository,
        Context::Repositories,
        Context::Settings,
        Context::Schedule,
    ] {
        let mut input = Input::new(Some(context));
        assert_eq!(
            press(&mut input, KeyCode::Backspace),
            NamespaceEvent::Handled
        );
        assert_eq!(
            selected(&input),
            context.name().split_whitespace().last().unwrap()
        );
        assert_eq!(input.menu_context(), context.parent());
        let range = input.prefix_selection().unwrap();
        assert_eq!(
            &input.text()[range],
            if context.parent().is_some() {
                context.name().split_whitespace().last().unwrap().to_owned()
            } else {
                format!("/{}", context.name())
            }
        );
        press(&mut input, KeyCode::Left);
        assert_eq!(input.prefix_selection().unwrap().start, 0);
        assert_eq!(input.menu_context(), None);
        press(&mut input, KeyCode::Left);
        assert_eq!(input.prefix_selection().unwrap().start, 0);
        press(&mut input, KeyCode::Right);
        if context.parent().is_some() {
            assert_eq!(
                selected(&input),
                context.name().split_whitespace().last().unwrap()
            );
            assert_eq!(input.menu_context(), context.parent());
            press(&mut input, KeyCode::Right);
        }
        assert!(!input.prefix_selected());
        assert_eq!(input.context(), Some(context));
        assert_eq!(input.text(), format!("/{} ", context.name()));
    }
}

#[test]
fn typing_replaces_only_the_leaf_and_enter_opens_the_new_view_without_writes() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let f = support::Fixture::create();
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        let config = std::fs::read(s.service.store.config_path()).unwrap();
        let index = std::fs::read(f.repo.join(".git/index")).unwrap();
        let head = support::git(&f.repo, &["rev-parse", "HEAD"]);
        branches(&mut s);
        let page = s.page;
        let depth = s.history.len();
        let names = s
            .search_commands()
            .unwrap()
            .into_iter()
            .map(|c| c.name)
            .collect::<Vec<_>>();
        assert!(names.contains(&"repo log") && names.contains(&"repo branch"));
        assert!(names.iter().all(|name| name.starts_with("repo ")));
        assert_eq!(s.bar.text(), "/repo branch ");
        type_text(&mut s, "log");
        assert_eq!(s.bar.text(), "/repo log");
        assert_eq!(
            s.bar.command_input().unwrap().context(),
            Some(Context::Repository)
        );
        assert_eq!(s.page, page);
        assert_eq!(s.history.len(), depth);
        assert!(matches!(&s.mode, Mode::Branches(view) if view.session().query == "main"));
        assert!(s.repository_tasks.is_empty());
        key(&mut s, KeyCode::Enter);
        wait(&mut s);
        assert!(matches!(s.mode, Mode::History(_)));
        assert_eq!(s.scope_path(), Some(f.repo.clone()));
        assert_eq!(support::git(&f.repo, &["rev-parse", "HEAD"]), head);
        assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
        assert_eq!(
            std::fs::read(s.service.store.config_path()).unwrap(),
            config
        );
        assert!(s.check_task.is_none());
        s.shutdown();
    }
}

#[test]
fn expanding_selection_reaches_global_commands_and_escape_or_enter_restores_original_scope() {
    let (_home, mut s) = fixture(vec![]);
    for restore in [KeyCode::Esc, KeyCode::Enter] {
        for expanded in [false, true] {
            s.bar = bar::InputBar::Command {
                input: Input::new(Some(Context::Branches)),
                selected: 0,
                menu: Default::default(),
                path: None,
                selection: Default::default(),
            };
            key(&mut s, KeyCode::Backspace);
            if expanded {
                key(&mut s, KeyCode::Left);
                assert!(
                    s.search_commands()
                        .unwrap()
                        .iter()
                        .all(|c| !c.name.contains(' '))
                );
            }
            key(&mut s, restore);
            assert_eq!(s.bar.text(), "/repo branch ");
            assert_eq!(
                s.bar.command_input().unwrap().context(),
                Some(Context::Branches)
            );
            assert!(!s.bar.command_input().unwrap().prefix_selected());
            assert!(s.repository_tasks.is_empty());
        }
    }
    s.shutdown();
}

#[test]
fn deleting_the_selected_leaf_keeps_the_parent_and_a_full_selection_can_close_the_draft() {
    for code in [KeyCode::Backspace, KeyCode::Delete] {
        let mut input = Input::new(Some(Context::Branches));
        press(&mut input, KeyCode::Backspace);
        assert_eq!(press(&mut input, code), NamespaceEvent::Handled);
        assert_eq!(input.context(), Some(Context::Repository));
        assert_eq!(input.text(), "/repo ");
        assert!(!input.prefix_selected());
        press(&mut input, KeyCode::Backspace);
        assert_eq!(press(&mut input, code), NamespaceEvent::Close);
        let mut input = Input::new(Some(Context::Branches));
        press(&mut input, KeyCode::Backspace);
        press(&mut input, KeyCode::Left);
        assert_eq!(press(&mut input, code), NamespaceEvent::Close);
    }
}

#[test]
fn paste_retains_unselected_segments_and_an_explicit_slash_replaces_the_whole_command() {
    for (text, expected, context) in [
        ("log", "/repo log", Some(Context::Repository)),
        ("log more", "/repo log more", Some(Context::Repository)),
        ("分支", "/repo 分支", Some(Context::Repository)),
        (
            "branch create \"中文 名称\"",
            "/repo branch create \"中文 名称\"",
            Some(Context::Repository),
        ),
        ("/settings", "/settings", None),
        ("//settings", "/settings", None),
    ] {
        let mut input = Input::new(Some(Context::Branches));
        press(&mut input, KeyCode::Backspace);
        for empty in ["", "\n\r\t"] {
            input.paste(empty);
            assert_eq!(selected(&input), "branch");
        }
        input.paste(text);
        assert_eq!(input.text(), expected);
        assert_eq!(input.context(), context);
        assert_eq!(input.editor().cursor, input.editor().text.len());
    }
    let mut input = Input::new(Some(Context::Branches));
    press(&mut input, KeyCode::Backspace);
    press(&mut input, KeyCode::Left);
    input.paste("settings");
    assert_eq!(input.text(), "/settings");
    assert_eq!(input.context(), None);
}

#[test]
fn sibling_candidates_support_tab_and_pointer_activation() {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    for pointer in [false, true] {
        let f = support::Fixture::create();
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        branches(&mut s);
        let index = s
            .search_commands()
            .unwrap()
            .iter()
            .position(|c| c.name == "repo log")
            .unwrap();
        for _ in 0..index {
            key(&mut s, KeyCode::Down);
        }
        assert_eq!(selected(s.bar.command_input().unwrap()), "branch");
        if pointer {
            render(&mut s, 120, 32);
            let (area, _) = s
                .command_items
                .iter()
                .find(|(_, candidate)| *candidate == index)
                .copied()
                .unwrap();
            s.mouse(
                MouseEvent {
                    kind: MouseEventKind::Down(MouseButton::Left),
                    column: area.x,
                    row: area.y,
                    modifiers: KeyModifiers::NONE,
                },
                &Cancel::default(),
            )
            .unwrap();
        } else {
            key(&mut s, KeyCode::Tab);
            assert_eq!(s.bar.text(), "/repo log");
            assert_eq!(
                s.bar.command_input().unwrap().context(),
                Some(Context::Repository)
            );
            assert!(matches!(s.mode, Mode::Branches(_)));
            key(&mut s, KeyCode::Enter);
        }
        wait(&mut s);
        assert!(matches!(s.mode, Mode::History(_)));
        assert!(s.check_task.is_none());
        s.shutdown();
    }
}

#[test]
fn partial_highlight_has_distinct_spans_and_survives_narrow_layouts() {
    use ratatui::style::Color;
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        for (width, height) in [(12, 8), (20, 12), (60, 18), (120, 34)] {
            let (_home, mut s) = fixture(vec![]);
            s.bar = bar::InputBar::Command {
                input: Input::new(Some(Context::Branches)),
                selected: 0,
                menu: Default::default(),
                path: None,
                selection: Default::default(),
            };
            key(&mut s, KeyCode::Backspace);
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal.draw(|frame| view::render(frame, &mut s)).unwrap();
            if width >= 60 {
                let inner = view::input::content_area(s.input_area.unwrap());
                let slash = &terminal.backend().buffer()[(inner.x, inner.y)];
                let leaf = &terminal.backend().buffer()[(inner.x + 6, inner.y)];
                assert_eq!(slash.fg, Color::LightCyan);
                assert_eq!(slash.bg, Color::Rgb(34, 52, 65));
                assert_eq!(leaf.symbol(), "b");
                assert_eq!(leaf.fg, Color::Black);
                assert_eq!(leaf.bg, Color::Rgb(180, 145, 220));
                assert_eq!(
                    terminal.backend().buffer()[(inner.x + "/repo branch".len() as u16, inner.y)]
                        .bg,
                    Color::Reset
                );
                assert_eq!(
                    terminal.backend().buffer()[(inner.x + "/repo branch ".len() as u16, inner.y)]
                        .symbol(),
                    "›"
                );
                assert!(!terminal.backend().cursor_visible());
            }
            key(&mut s, KeyCode::Left);
            terminal.draw(|frame| view::render(frame, &mut s)).unwrap();
            key(&mut s, KeyCode::Right);
            terminal.draw(|frame| view::render(frame, &mut s)).unwrap();
            assert_eq!(selected(s.bar.command_input().unwrap()), "branch");
            key(&mut s, KeyCode::Esc);
            terminal.draw(|frame| view::render(frame, &mut s)).unwrap();
            assert!(terminal.backend().cursor_visible());
            s.shutdown();
        }
    }
}

#[test]
fn leaf_replacement_keeps_the_repository_captured_before_selection_changes() {
    let f = support::Fixture::create();
    let (_home, mut s) = fixture(vec![f.repo.clone(), "/example/other".into()]);
    s.focus(Some(f.repo.clone()));
    branches(&mut s);
    let (tx, rx) = mpsc::channel();
    s.clipboard = crate::clipboard::Writer::with_sink(move |text| {
        tx.send(text)?;
        Ok(())
    });
    let expected = match &s.mode {
        Mode::Branches(view) => view.session().selected().unwrap().name.clone(),
        _ => panic!("branch"),
    };
    s.focus(Some("/example/other".into()));
    type_text(&mut s, "/copy");
    assert_eq!(s.bar.text(), "/copy");
    assert!(
        matches!(&s.bar, crate::bar::InputBar::Command { path: Some(path), .. } if path == &f.repo)
    );
    key(&mut s, KeyCode::Enter);
    assert_eq!(rx.recv_timeout(Duration::from_secs(5)).unwrap(), expected);
    assert_eq!(s.selected_path(), Some("/example/other".into()));
    assert!(matches!(s.mode, Mode::Branches(_)));
    assert!(s.repository_tasks.is_empty() && s.check_task.is_none());
    s.shutdown();
}
