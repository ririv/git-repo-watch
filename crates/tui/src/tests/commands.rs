use super::*;

#[test]
fn command_and_option_menus_keep_the_viewport_until_selection_crosses_its_edge() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        for (entry, height) in [("repo", 18), ("repos filter", 12)] {
            let (_home, mut s) = fixture(vec!["/example/alpha".into()]);
            if entry == "repo" {
                s.paste("/repo");
            } else {
                command(&mut s, entry);
            }
            render(&mut s, 100, height);
            let count = s.command_items.len();
            assert!(count > 1);
            for _ in 0..count {
                key(&mut s, KeyCode::Down);
                render(&mut s, 100, height);
            }
            let window = s.command_items.clone();
            assert_eq!(window.first().unwrap().1, 1);
            assert_eq!(window.last().unwrap().1, count);
            key(&mut s, KeyCode::Up);
            render(&mut s, 100, height);
            assert_eq!(
                s.command_items, window,
                "{entry}: moving within the window must not scroll"
            );
            for _ in 0..count - 2 {
                key(&mut s, KeyCode::Up);
                render(&mut s, 100, height);
                assert_eq!(s.command_items, window);
            }
            key(&mut s, KeyCode::Up);
            render(&mut s, 100, height);
            assert_eq!(s.command_items.first().unwrap().1, 0);
            assert_eq!(s.command_items.last().unwrap().1, count - 1);
            key(&mut s, KeyCode::Esc);
            if entry == "repos filter" {
                key(&mut s, KeyCode::Esc);
            }
            render(&mut s, 100, height);
            assert!(s.command_items.is_empty());
            s.shutdown();
        }
    }
}

#[test]
fn menu_viewport_keeps_selection_visible_on_resize_and_resets_for_new_candidates() {
    let (_home, mut s) = fixture(vec!["/example/alpha".into()]);
    s.paste("/repo");
    for _ in 0..command::COMMANDS.len() {
        key(&mut s, KeyCode::Down);
    }
    for (width, height) in [(100, 28), (60, 12), (150, 32)] {
        render(&mut s, width, height);
        let bar::InputBar::Command { selected, .. } = &s.bar else {
            panic!()
        };
        assert!(s.command_items.iter().any(|(_, index)| index == selected));
        let window = s.command_items.clone();
        key(&mut s, KeyCode::Up);
        render(&mut s, width, height);
        assert_eq!(s.command_items, window);
    }
    s.paste(" copy");
    render(&mut s, 100, 28);
    assert_eq!(s.command_items.len(), 1);
    assert_eq!(s.command_items[0].1, 0);
    s.paste("-no-matching-command");
    render(&mut s, 100, 28);
    assert!(s.command_items.is_empty());
    key(&mut s, KeyCode::Esc);
    render(&mut s, 100, 28);
    s.paste("/repo");
    render(&mut s, 100, 28);
    assert_eq!(s.command_items.first().unwrap().1, 0);
    s.shutdown();
}

#[test]
fn context_global_hint_only_appears_when_another_slash_switches_scope() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec![]);
        let global_hint = i18n::tr!(ContextGlobalCommandHint)
            .split(" · ")
            .next()
            .unwrap();
        s.paste("/");
        assert!(!render(&mut s, 100, 28).contains(global_hint));
        key(&mut s, KeyCode::Esc);
        command(&mut s, "settings");
        s.paste("/");
        for width in [60, 100, 180] {
            let text = render(&mut s, width, 28);
            let input = s.input_area.unwrap();
            let hint = text.lines().nth(input.bottom().into()).unwrap();
            assert!(hint.contains(global_hint), "{text}");
            assert!(
                text.contains(i18n::tr!(ContextCommandPlaceholder)),
                "{text}"
            );
        }
        key(&mut s, KeyCode::Char('/'));
        assert!(matches!(
            s.bar.command_input(),
            Some(command::Input::Global(_))
        ));
        assert_eq!(s.bar.text(), "/");
        assert!(!render(&mut s, 100, 28).contains(global_hint));
        key(&mut s, KeyCode::Esc);
        s.paste("/");
        s.paste("concurrency");
        assert!(!render(&mut s, 100, 28).contains(global_hint));
        key(&mut s, KeyCode::Char('/'));
        assert!(matches!(
            s.bar.command_input(),
            Some(command::Input::Scoped { .. })
        ));
        assert!(s.bar.text().ends_with("concurrency/"));
        key(&mut s, KeyCode::Esc);
        s.paste("/");
        key(&mut s, KeyCode::Backspace);
        let text = render(&mut s, 100, 28);
        assert!(text.contains(i18n::tr!(ContextPrefixHint)), "{text}");
        assert!(!text.contains(global_hint));
        s.shutdown();
    }
}

#[test]
fn command_menu_keeps_input_title_in_shared_border_and_restores_search_on_close() {
    use ratatui::style::Color;
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec![]);
        for (width, height) in [(60, 18), (100, 28), (180, 36)] {
            s.paste("/");
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal.draw(|frame| view::render(frame, &mut s)).unwrap();
            let input = s.input_area.unwrap();
            let first = s.command_items.first().unwrap().0;
            let last = s.command_items.last().unwrap().0;
            let buffer = terminal.backend().buffer();
            assert_eq!(s.command_items.len(), 8);
            assert_eq!(last.bottom(), input.y);
            assert_eq!(buffer[(0, first.y - 1)].symbol(), "╭");
            assert_eq!(buffer[(width - 1, first.y - 1)].symbol(), "╮");
            assert_eq!(buffer[(0, input.y)].symbol(), "├");
            assert_eq!(buffer[(width - 1, input.y)].symbol(), "┤");
            let overflowing = s.search_commands().unwrap().len() > s.command_items.len();
            assert_eq!(
                (first.y..input.y).any(|y| buffer[(width - 1, y)].symbol() == "┃"),
                overflowing
            );
            for x in 1..width - 1 {
                assert_eq!(buffer[(x, first.y - 1)].symbol(), "─");
                assert_eq!(buffer[(x, first.y - 1)].fg, Color::Rgb(180, 145, 220));
            }
            let text = render(&mut s, width, height);
            let title = text.lines().nth(input.y.into()).unwrap();
            assert!(title.contains(i18n::tr!(CommandsAlt).trim()), "{title}");
            key(&mut s, KeyCode::Esc);
            terminal.draw(|frame| view::render(frame, &mut s)).unwrap();
            assert!(s.command_items.is_empty());
            let buffer = terminal.backend().buffer();
            assert_eq!(buffer[(0, input.y)].symbol(), "╭");
            assert_eq!(buffer[(width - 1, input.y)].symbol(), "╮");
        }
        s.shutdown();
    }
}

#[test]
fn description_selection_completes_real_commands_and_returns_parameter_drafts() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec![]);
        let configuration = std::fs::read(s.service.store.config_path()).unwrap();
        s.paste("//检查频率");
        for (width, height) in [(60, 18), (100, 28), (180, 36)] {
            let text = render(&mut s, width, height);
            assert!(text.contains("/schedule interval <duration>"), "{text}");
        }
        key(&mut s, KeyCode::Tab);
        assert_eq!(s.bar.text(), "/schedule interval ");
        s.paste("1w");
        key(&mut s, KeyCode::Enter);
        assert!(matches!(s.mode, Mode::Confirm { .. }));
        key(&mut s, KeyCode::Esc);
        assert_eq!(s.bar.text(), "/schedule interval 1w");
        assert_eq!(
            std::fs::read(s.service.store.config_path()).unwrap(),
            configuration
        );
        assert!(s.service.store.report().unwrap().is_none());
        assert!(s.repository_tasks.is_empty());
    }
}

#[test]
fn clicking_description_results_uses_the_same_selection_and_keeps_frozen_scope() {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    let (_home, mut s) = fixture(vec!["/example/alpha".into(), "/example/beta".into()]);
    let configuration = std::fs::read(s.service.store.config_path()).unwrap();
    let description =
        i18n::Message::UnregisterRepositoryKeepFilesOnDisk.in_language(i18n::Language::Chinese);
    s.paste(&format!("/{description}"));
    s.select(1);
    render(&mut s, 180, 36);
    let (area, index) = s.command_items[0];
    assert_eq!(s.search_commands().unwrap()[index].action, Action::Remove);
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
    assert!(
        matches!(&s.mode, Mode::Confirm { operation: Confirmation::Remove { path, .. }, .. } if path == &PathBuf::from("/example/alpha"))
    );
    key(&mut s, KeyCode::Esc);
    assert_eq!(
        std::fs::read(s.service.store.config_path()).unwrap(),
        configuration
    );
    assert!(s.repository_tasks.is_empty());
}

#[test]
fn concise_commands_show_values_without_scope_or_detail_rows_and_remain_clickable() {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec![]);
        command(&mut s, "settings ai");
        let configuration = std::fs::read(s.service.store.config_path()).unwrap();
        s.paste("/");
        let text = render(&mut s, 140, 32);
        let items = s.command_items.clone();
        assert_eq!(items.len(), 3);
        for (area, index) in &items {
            let command = s.search_commands().unwrap()[*index];
            assert_eq!(area.height, 1);
            let line = text.lines().nth(area.y.into()).unwrap();
            assert!(line.contains(command.summary().text()), "{line}");
            for absent in [
                i18n::tr!(Global),
                i18n::tr!(CurrentRepo),
                i18n::tr!(CurrentView),
                i18n::tr!(LayoutCurrent),
            ] {
                assert!(!line.contains(absent), "{line}");
            }
        }
        assert!(text.contains("agent [codex|claude]"));
        assert!(
            items
                .windows(2)
                .all(|pair| pair[0].0.bottom() == pair[1].0.y)
        );
        assert!(
            text.lines()
                .nth(items[0].0.y.into())
                .unwrap()
                .contains("Codex")
        );
        assert!(
            text.lines()
                .nth(items[1].0.y.into())
                .unwrap()
                .contains("Default")
        );
        assert!(!text.contains(i18n::tr!(AgentNameHint)));
        key(&mut s, KeyCode::Down);
        let text = render(&mut s, 140, 32);
        assert_eq!(s.command_items, items);
        assert!(text.contains(i18n::CommandSummary::AiModel.text()));
        let model = items[1].0;
        assert!(
            s.mouse(
                MouseEvent {
                    kind: MouseEventKind::Down(MouseButton::Left),
                    column: model.x + 2,
                    row: model.y,
                    modifiers: KeyModifiers::NONE,
                },
                &Cancel::default()
            )
            .unwrap()
        );
        assert_eq!(s.bar.text(), "/settings ai model ");
        assert!(matches!(s.mode, Mode::Browsing));
        assert_eq!(
            std::fs::read(s.service.store.config_path()).unwrap(),
            configuration
        );
        s.shutdown();
    }
}

#[test]
fn command_descriptions_remain_searchable_across_interface_languages() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec![]);
        for query in ["填写模型名", "ENTER A MODEL NAME", "设置模型", "SET MODEL"] {
            s.paste(&format!("//{query}"));
            let matches = s.search_commands().unwrap();
            assert_eq!(matches.len(), 1);
            assert_eq!(matches[0].action, Action::AiModel);
            let text = render(&mut s, 140, 28);
            assert!(text.contains(i18n::CommandSummary::AiModel.text()));
            key(&mut s, KeyCode::Esc);
        }
        s.shutdown();
    }
}
