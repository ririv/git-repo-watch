use super::*;
use crate::command::Context;

fn select_prefix(s: &mut Screen) {
    s.page = Page::Configuration(Configuration::Repositories);
    key(s, KeyCode::Home);
    key(s, KeyCode::Char('/'));
    key(s, KeyCode::Backspace);
    assert!(s.bar.command_input().unwrap().prefix_selected());
}
fn type_text(s: &mut Screen, text: &str) {
    for c in text.chars() {
        key(s, KeyCode::Char(c));
    }
}
#[test]
fn selected_prefix_is_replaced_by_typing_including_shift_and_unicode() {
    for (text, modifiers) in [
        ("diff", KeyModifiers::NONE),
        ("Settings", KeyModifiers::SHIFT),
        ("终端", KeyModifiers::NONE),
    ] {
        let (_home, mut s) = fixture(vec!["/example/alpha".into()]);
        select_prefix(&mut s);
        for c in text.chars() {
            s.key(
                KeyEvent::new(KeyCode::Char(c), modifiers),
                &Cancel::default(),
            )
            .unwrap();
        }
        let input = s.bar.command_input().unwrap();
        assert_eq!(input.context(), None);
        assert_eq!(input.text(), format!("/{text}"));
        assert_eq!(input.editor().cursor, input.editor().text.len());
        assert!(!s.bar.commands(s.config.ai.enabled).unwrap().is_empty());
        assert!(s.repository_tasks.is_empty());
        assert_eq!(s.page, Page::Configuration(Configuration::Repositories));
        key(&mut s, KeyCode::Esc);
        key(&mut s, KeyCode::Char('/'));
        assert_eq!(
            s.bar.command_input().unwrap().context(),
            Some(Context::Repositories)
        );
    }
}
#[test]
fn paste_replaces_the_selected_prefix_and_preserves_literal_parameters() {
    for (text, expected) in [
        ("diff", "/diff"),
        ("/diff", "/diff"),
        ("//diff", "/diff"),
        ("repos add \"~/中文 空格\"", "/repos add \"~/中文 空格\""),
        ("\n/终端\r", "/终端"),
    ] {
        let (_home, mut s) = fixture(vec!["/example/alpha".into()]);
        select_prefix(&mut s);
        s.paste(text);
        assert_eq!(s.bar.text(), expected);
        assert_eq!(s.bar.command_input().unwrap().context(), None);
        assert_eq!(s.bar.editor().cursor, expected.len());
        assert!(s.repository_tasks.is_empty());
        assert_eq!(
            s.bar.commands(s.config.ai.enabled).unwrap()[0].name,
            if expected.contains(" add ") {
                "repos add"
            } else if expected == "/终端" {
                "repo terminal"
            } else {
                "repo diff"
            }
        );
    }
    let (_home, mut s) = fixture(vec![]);
    select_prefix(&mut s);
    for text in ["", "\n\r\t"] {
        s.paste(text);
        assert!(s.bar.command_input().unwrap().prefix_selected());
    }
}
#[test]
fn accidental_selection_can_be_restored_or_closed_without_running_a_candidate() {
    for restore in [KeyCode::Enter, KeyCode::Right] {
        let (_home, mut s) = fixture(vec!["/example/alpha".into()]);
        s.paste("alpha");
        select_prefix(&mut s);
        key(&mut s, restore);
        let input = s.bar.command_input().unwrap();
        assert!(!input.prefix_selected());
        assert_eq!(input.context(), Some(Context::Repositories));
        assert_eq!(input.editor().text, "");
        assert_eq!(s.bar.text(), "/repos ");
        assert_eq!(s.filter, Filter::All);
        assert_eq!(s.query, "alpha");
        assert!(s.repository_tasks.is_empty());
        key(&mut s, KeyCode::Backspace);
        key(&mut s, KeyCode::Esc);
        assert_eq!(s.bar.editor().text, "alpha");
        assert!(matches!(s.bar, bar::InputBar::Search(_)));
        assert_eq!(s.selected_path(), Some("/example/alpha".into()));
    }
}
#[test]
fn typing_slash_over_a_selected_prefix_opens_the_global_menu() {
    let (_home, mut s) = fixture(vec![]);
    select_prefix(&mut s);
    key(&mut s, KeyCode::Char('/'));
    assert_eq!(s.bar.text(), "/");
    assert_eq!(s.bar.command_input().unwrap().context(), None);
    assert!(
        s.bar
            .commands(s.config.ai.enabled)
            .unwrap()
            .iter()
            .all(|c| !c.name.contains(' '))
    );
    assert!(s.repository_tasks.is_empty());
    let (_home, mut s) = fixture(vec![]);
    s.page = Page::Configuration(Configuration::Repositories);
    key(&mut s, KeyCode::Char('/'));
    s.paste("scan /example");
    key(&mut s, KeyCode::Home);
    key(&mut s, KeyCode::Backspace);
    assert!(!s.bar.command_input().unwrap().prefix_selected());
    assert_eq!(s.bar.text(), "/repos scan /example");
}
#[test]
fn replacing_the_prefix_keeps_the_original_frozen_repository_target() {
    let (_home, mut s) = fixture(vec!["/example/alpha".into(), "/example/beta".into()]);
    select_prefix(&mut s);
    s.focus(Some("/example/beta".into()));
    type_text(&mut s, "repos remove");
    assert_eq!(s.bar.text(), "/repos remove");
    key(&mut s, KeyCode::Enter);
    assert!(
        matches!(&s.mode,Mode::Confirm{operation:Confirmation::Remove(path),..} if path==std::path::Path::new("/example/alpha"))
    );
    key(&mut s, KeyCode::Esc);
    assert_eq!(s.selected_path(), Some("/example/beta".into()));
}
#[test]
fn selected_prefix_keeps_the_original_style_and_shows_the_same_menu_as_global_slash() {
    use ratatui::style::{Color, Modifier};
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        for (width, height) in [(60, 18), (100, 28), (180, 36)] {
            let (_home, mut s) = fixture(vec!["/example/alpha".into()]);
            s.page = Page::Configuration(Configuration::Repositories);
            key(&mut s, KeyCode::Char('/'));
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal.draw(|f| view::render(f, &mut s)).unwrap();
            let ordinary = terminal.backend().buffer()[(2, height - 4)].clone();
            key(&mut s, KeyCode::Backspace);
            let selected_text = render(&mut s, width, height);
            let selected_names = s
                .bar
                .commands(s.config.ai.enabled)
                .unwrap()
                .iter()
                .map(|c| c.name)
                .collect::<Vec<_>>();
            terminal.draw(|f| view::render(f, &mut s)).unwrap();
            let selected = &terminal.backend().buffer()[(2, height - 4)];
            assert_eq!(selected.symbol(), "/");
            assert_eq!(selected.fg, Color::Black);
            assert_eq!(selected.bg, Color::Rgb(180, 145, 220));
            assert!(selected.modifier.contains(Modifier::BOLD));
            assert!(!selected.modifier.contains(Modifier::UNDERLINED));
            assert!(!terminal.backend().cursor_visible());
            assert!(!selected_text.contains("[ /repos ]"));
            assert!(selected_text.contains(i18n::tr!(CommandsAlt).trim()));
            let selected_input = s.bar.command_input().unwrap().clone();
            if let bar::InputBar::Command { input, .. } = &mut s.bar {
                *input = crate::command::Input::new(None);
            }
            let global_text = render(&mut s, width, height);
            assert_eq!(
                selected_names,
                s.bar
                    .commands(s.config.ai.enabled)
                    .unwrap()
                    .iter()
                    .map(|c| c.name)
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                selected_text
                    .lines()
                    .take((height - 5) as usize)
                    .collect::<Vec<_>>(),
                global_text
                    .lines()
                    .take((height - 5) as usize)
                    .collect::<Vec<_>>()
            );
            if let bar::InputBar::Command { input, .. } = &mut s.bar {
                *input = selected_input;
            }
            key(&mut s, KeyCode::Right);
            terminal.draw(|f| view::render(f, &mut s)).unwrap();
            assert_eq!(terminal.backend().buffer()[(2, height - 4)].bg, ordinary.bg);
            assert!(terminal.backend().cursor_visible());
        }
    }
}
#[test]
fn global_candidates_can_be_navigated_and_completed_while_the_prefix_is_selected() {
    let (_home, mut s) = fixture(vec!["/example/alpha".into()]);
    select_prefix(&mut s);
    let index = s
        .bar
        .commands(s.config.ai.enabled)
        .unwrap()
        .iter()
        .position(|c| c.action == Action::Schedule)
        .unwrap();
    for _ in 0..index {
        key(&mut s, KeyCode::Down);
    }
    assert!(s.bar.command_input().unwrap().prefix_selected());
    key(&mut s, KeyCode::Tab);
    assert_eq!(s.bar.text(), "/schedule");
    assert_eq!(s.bar.command_input().unwrap().context(), None);
    assert_eq!(s.page, Page::Configuration(Configuration::Repositories));
    key(&mut s, KeyCode::Enter);
    assert_eq!(s.page, Page::Configuration(Configuration::Schedule));
    assert!(s.repository_tasks.is_empty());
}
#[test]
fn clicking_a_global_candidate_replaces_the_selected_prefix_before_opening_the_view() {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    let (_home, mut s) = fixture(vec![]);
    select_prefix(&mut s);
    let index = s
        .bar
        .commands(s.config.ai.enabled)
        .unwrap()
        .iter()
        .position(|c| c.action == Action::Schedule)
        .unwrap();
    for _ in 0..index {
        key(&mut s, KeyCode::Down);
    }
    render(&mut s, 100, 28);
    let (area, _) = s
        .command_items
        .iter()
        .find(|(_, i)| *i == index)
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
    assert_eq!(s.page, Page::Configuration(Configuration::Schedule));
    assert!(s.repository_tasks.is_empty());
}

#[test]
fn deleting_a_selected_prefix_closes_commands_and_restores_search() {
    for query in ["", "alpha"] {
        let (_home, mut s) = fixture(vec!["/example/alpha".into()]);
        s.paste(query);
        let path = s.selected_path();
        select_prefix(&mut s);
        key(&mut s, KeyCode::Down);
        key(&mut s, KeyCode::Backspace);
        assert!(matches!(s.bar, bar::InputBar::Search(_)));
        assert_eq!(s.bar.text(), query);
        assert_eq!(s.query, query);
        assert_eq!(s.selected_path(), path);
        assert_eq!(s.page, Page::Configuration(Configuration::Repositories));
        assert!(s.repository_tasks.is_empty());
        assert!(s.bar.commands(s.config.ai.enabled).is_none());
        key(&mut s, KeyCode::Home);
        key(&mut s, KeyCode::Char('/'));
        assert_eq!(s.bar.text(), "/repos ");
        assert!(!s.bar.command_input().unwrap().prefix_selected());
    }
}
