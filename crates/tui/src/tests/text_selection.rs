use super::*;
use crossterm::event::{KeyEventKind, MouseButton, MouseEvent, MouseEventKind};
use engine::state::InteractionMode;

fn toggle(s: &mut Screen) {
    s.key(
        KeyEvent::new(KeyCode::Char('g'), KeyModifiers::CONTROL),
        &Cancel::default(),
    )
    .unwrap();
}
#[test]
fn text_selection_preserves_drafts_and_keyboard_operation_but_releases_pointer() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec!["/example/alpha".into(), "/example/beta".into()]);
        s.config.mouse.scroll = false;
        s.config.mouse.select_pane = false;
        s.service.store.save_config(&s.config).unwrap();
        let preferences = s.config.mouse.clone();
        s.paste("alp");
        toggle(&mut s);
        wait(&mut s);
        assert_eq!(s.config.interaction_mode, InteractionMode::TextSelection);
        assert_eq!(s.bar.text(), "alp");
        let rendered = render(&mut s, 100, 24);
        assert!(!rendered.contains(i18n::tr!(InteractionText)));
        let mut terminal = Terminal::new(TestBackend::new(100, 24)).unwrap();
        terminal.draw(|frame| view::render(frame, &mut s)).unwrap();
        assert_eq!(
            terminal.backend().buffer()[(0, 20)].fg,
            ratatui::style::Color::LightGreen
        );
        let mut repeated = KeyEvent::new(KeyCode::Char('g'), KeyModifiers::CONTROL);
        repeated.kind = KeyEventKind::Repeat;
        s.key(repeated, &Cancel::default()).unwrap();
        assert!(!s.interaction_save.pending());
        for kind in [
            MouseEventKind::Down(MouseButton::Left),
            MouseEventKind::Down(MouseButton::Right),
            MouseEventKind::Drag(MouseButton::Left),
            MouseEventKind::ScrollDown,
            MouseEventKind::ScrollLeft,
        ] {
            assert!(
                !s.mouse(
                    MouseEvent {
                        kind,
                        column: 4,
                        row: 6,
                        modifiers: KeyModifiers::NONE
                    },
                    &Cancel::default()
                )
                .unwrap()
            );
        }
        assert_eq!(s.bar.text(), "alp");
        key(&mut s, KeyCode::Char('h'));
        s.paste("a");
        assert_eq!(s.bar.text(), "alpha");
        key(&mut s, KeyCode::Esc);
        assert_eq!(s.config.interaction_mode, InteractionMode::TextSelection);
        assert_eq!(s.bar.text(), "");
        key(&mut s, KeyCode::Down);
        assert_eq!(s.selected(), 1);
        s.paste("/repo diff");
        let draft = s.bar.text();
        toggle(&mut s);
        wait(&mut s);
        assert_eq!(s.bar.text(), draft);
        assert_eq!(s.config.interaction_mode, InteractionMode::Application);
        assert_eq!(s.config.mouse, preferences);
        command(&mut s, "settings interaction text");
        wait(&mut s);
        command(&mut s, "settings");
        assert_eq!(s.page, Page::Configuration(Configuration::General));
        assert_eq!(s.config.interaction_mode, InteractionMode::TextSelection);
        assert!(
            s.key(
                KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
                &Cancel::default()
            )
            .unwrap()
        );
        s.shutdown();
    }
}
#[test]
fn preference_persists_and_settings_and_commands_select_current_mode() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec![]);
        command(&mut s, "settings interaction text");
        wait(&mut s);
        assert_eq!(
            s.service.store.config().unwrap().interaction_mode,
            InteractionMode::TextSelection
        );
        let mut fresh = screen(Service::new(
            s.service.store.clone(),
            s.service.scheduler.clone(),
        ));
        assert_eq!(
            fresh.config.interaction_mode,
            InteractionMode::TextSelection
        );
        fresh.shutdown();
        command(&mut s, "settings interaction");
        let crate::bar::InputBar::Command {
            input, selected, ..
        } = &s.bar
        else {
            panic!("mode choices")
        };
        assert_eq!(
            input.matches()[*selected].action,
            Action::Interaction(InteractionMode::TextSelection)
        );
        key(&mut s, KeyCode::Esc);
        command(&mut s, "settings");
        s.activate_setting(crate::settings::Item::Interaction, &Cancel::default())
            .unwrap();
        let rendered = render(&mut s, 120, 34);
        assert!(rendered.contains("interaction interactive"));
        assert!(rendered.contains("interaction text"));
        assert_eq!(
            s.bar.command_input().unwrap().context(),
            Some(command::Context::Settings)
        );
        assert_eq!(s.bar.text(), "/settings interaction ");
        key(&mut s, KeyCode::Up);
        key(&mut s, KeyCode::Enter);
        wait(&mut s);
        assert_eq!(s.config.interaction_mode, InteractionMode::Application);
        assert_eq!(s.page, Page::Configuration(Configuration::General));
        s.shutdown();
    }
}
#[test]
fn config_lock_never_blocks_keyboard_and_rapid_changes_save_last_requested_mode() {
    let (_home, mut s) = fixture(vec!["/example/alpha".into(), "/example/beta".into()]);
    let lock = s.service.store.lock().unwrap();
    toggle(&mut s);
    assert!(s.interaction_save.pending());
    key(&mut s, KeyCode::Down);
    assert_eq!(s.selected(), 1);
    toggle(&mut s);
    toggle(&mut s);
    toggle(&mut s);
    drop(lock);
    wait(&mut s);
    assert_eq!(s.config.interaction_mode, InteractionMode::Application);
    assert_eq!(
        s.service.store.config().unwrap().interaction_mode,
        InteractionMode::Application
    );
    s.shutdown();
}
#[test]
fn failed_save_keeps_applied_mode_and_reports_error() {
    let (_home, mut s) = fixture(vec![]);
    let path = s.service.store.config_path();
    let previous = std::fs::read(&path).unwrap();
    std::fs::write(&path, b"broken config [").unwrap();
    toggle(&mut s);
    wait(&mut s);
    assert_eq!(s.config.interaction_mode, InteractionMode::Application);
    assert!(s.message.contains(i18n::tr!(InteractionSaveFailed)));
    assert_eq!(std::fs::read(&path).unwrap(), b"broken config [");
    std::fs::write(path, previous).unwrap();
    s.shutdown();
}
#[test]
fn switching_back_to_interactive_restores_real_item_clicks_and_wheel_navigation() {
    for (width, height) in [(60, 18), (160, 32)] {
        let paths = (0..30)
            .map(|i| PathBuf::from(format!("/example/repository-{i:02}")))
            .collect();
        let (_home, mut s) = fixture(paths);
        command(&mut s, "settings interaction text");
        wait(&mut s);
        render(&mut s, width, height);
        let area = s.repository_area.unwrap();
        let pointer = |kind| MouseEvent {
            kind,
            column: area.x + 3,
            row: area.y + 3,
            modifiers: KeyModifiers::NONE,
        };
        assert!(
            !s.mouse(
                pointer(MouseEventKind::Down(MouseButton::Left)),
                &Cancel::default()
            )
            .unwrap()
        );
        assert!(
            !s.mouse(pointer(MouseEventKind::ScrollDown), &Cancel::default())
                .unwrap()
        );
        assert_eq!(s.selected(), 0);
        command(&mut s, "settings interaction interactive");
        wait(&mut s);
        render(&mut s, width, height);
        assert!(
            s.mouse(
                pointer(MouseEventKind::Down(MouseButton::Left)),
                &Cancel::default()
            )
            .unwrap()
        );
        assert_eq!(s.selected(), 1);
        assert!(
            s.mouse(pointer(MouseEventKind::ScrollDown), &Cancel::default())
                .unwrap()
        );
        assert_eq!(s.selected(), 4);
        s.shutdown();
    }
}

#[test]
fn capture_transitions_disable_and_restore_actual_terminal_reporting() {
    let mut output = Vec::new();
    crate::interaction::apply(InteractionMode::TextSelection, &mut output).unwrap();
    let released = String::from_utf8(output.clone()).unwrap();
    for mode in [1000, 1002, 1003, 1006] {
        assert!(released.contains(&format!("?{mode}l")));
    }
    output.clear();
    crate::interaction::apply(InteractionMode::Application, &mut output).unwrap();
    let captured = String::from_utf8(output).unwrap();
    for mode in [1000, 1002, 1003, 1006] {
        assert!(captured.contains(&format!("?{mode}h")));
    }
}
