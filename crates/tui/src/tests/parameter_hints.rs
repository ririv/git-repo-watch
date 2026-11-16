use super::*;
use crate::command::completion::Menu;
use ratatui::style::{Color, Modifier};

#[test]
fn parameter_hints_wait_for_confirmation_and_do_not_enter_execution_or_history() {
    let (_home, mut s) = fixture(vec![]);
    let before = std::fs::read(s.service.store.config_path()).unwrap();
    s.paste("/schedule interval");
    for code in [KeyCode::Down, KeyCode::Up] {
        let text = render(&mut s, 100, 28);
        assert!(!text.contains("<duration>"));
        key(&mut s, code);
        assert_eq!(s.bar.text(), "/schedule interval");
    }
    render(&mut s, 100, 28);
    let geometry = (s.body_area, s.input_area);
    key(&mut s, KeyCode::Enter);
    assert_eq!(s.bar.text(), "/schedule interval ");
    let mut terminal = Terminal::new(TestBackend::new(100, 28)).unwrap();
    terminal.draw(|frame| view::render(frame, &mut s)).unwrap();
    let cursor = terminal.backend().cursor_position();
    let cell = &terminal.backend().buffer()[cursor];
    assert_eq!(cell.symbol(), "<");
    assert_eq!(cell.fg, Color::DarkGray);
    assert!(cell.modifier.contains(Modifier::ITALIC));
    assert_eq!((s.body_area, s.input_area), geometry);
    key(&mut s, KeyCode::Enter);
    assert!(matches!(s.mode, Mode::Browsing));
    assert_eq!(s.bar.text(), "/schedule interval ");
    assert!(s.service.store.command_history().unwrap().is_empty());
    assert_eq!(
        std::fs::read(s.service.store.config_path()).unwrap(),
        before
    );
    s.paste("1h");
    assert!(!render(&mut s, 100, 28).contains("<duration>"));
    key(&mut s, KeyCode::Enter);
    assert!(matches!(s.mode, Mode::Confirm { .. }));
    assert_eq!(
        s.service.store.command_history().unwrap(),
        ["/schedule interval 1h"]
    );
    key(&mut s, KeyCode::Esc);
    assert_eq!(s.bar.text(), "/schedule interval 1h");
    assert_eq!(
        std::fs::read(s.service.store.config_path()).unwrap(),
        before
    );
    s.shutdown();
}

#[test]
fn option_candidates_still_fill_real_values_and_remove_the_inline_hint() {
    let (_home, mut s) = fixture(vec![]);
    s.paste("/repos filter");
    key(&mut s, KeyCode::Enter);
    assert_eq!(s.bar.text(), "/repos filter ");
    assert!(matches!(
        s.bar,
        bar::InputBar::Command {
            menu: Menu::Options(_),
            ..
        }
    ));
    assert!(render(&mut s, 100, 28).contains("[all|problems|"));
    key(&mut s, KeyCode::Down);
    assert_eq!(s.bar.text(), "/repos filter ");
    key(&mut s, KeyCode::Tab);
    assert_eq!(s.bar.text(), "/repos filter problems");
    assert!(!render(&mut s, 100, 28).contains("[all|problems|"));
    assert_eq!(s.filter, Filter::All);
    key(&mut s, KeyCode::Enter);
    assert_eq!(s.filter, Filter::Problems);
    assert_eq!(
        s.service.store.command_history().unwrap(),
        ["/repos filter problems"]
    );
    s.shutdown();
}

#[test]
fn scoped_ghost_hints_follow_cursor_edits_and_fit_narrow_windows_without_changing_the_draft() {
    let (_home, mut s) = fixture(vec![]);
    command(&mut s, "settings");
    s.paste("/");
    s.paste("concurrency");
    key(&mut s, KeyCode::Tab);
    assert_eq!(s.bar.text(), "/settings concurrency ");
    assert!(render(&mut s, 100, 28).contains("<count>"));
    for width in [0, 1, 8, 20, 60, 140] {
        render(&mut s, width, 24);
        assert_eq!(s.bar.text(), "/settings concurrency ");
    }
    key(&mut s, KeyCode::Left);
    assert!(!render(&mut s, 100, 28).contains("<count>"));
    key(&mut s, KeyCode::End);
    assert!(render(&mut s, 100, 28).contains("<count>"));
    s.paste("4");
    assert!(!render(&mut s, 100, 28).contains("<count>"));
    key(&mut s, KeyCode::Backspace);
    assert!(render(&mut s, 100, 28).contains("<count>"));
    assert_eq!(s.bar.text(), "/settings concurrency ");
    s.shutdown();
}

#[test]
fn layout_parameter_candidates_open_immediately_from_scoped_commands_and_apply_with_up_down() {
    use crate::{
        command::{
            Context, Input,
            options::{Kind, Value},
        },
        diff::Preference,
    };
    let (_home, mut s) = fixture(vec![]);
    s.paste("/");
    let bar::InputBar::Command { input, .. } = &mut s.bar else {
        panic!();
    };
    *input = Input::new(Some(Context::Repository));
    input.paste("diff layout");
    key(&mut s, KeyCode::Tab);
    assert_eq!(s.bar.text(), "/repo diff layout ");
    let bar::InputBar::Command {
        menu: Menu::Options(values),
        ..
    } = &s.bar
    else {
        panic!("layout parameter candidates");
    };
    assert_eq!(values.options.len(), 3);
    assert_eq!(
        values.options[values.selected].value,
        Value::DiffLayout(Preference::Auto)
    );
    let text = render(&mut s, 100, 28);
    assert_eq!(s.command_items.len(), 3, "{text}");
    for token in ["auto", "split", "unified"] {
        assert!(
            s.command_items.iter().any(|(area, _)| text
                .lines()
                .nth(usize::from(area.y))
                .unwrap()
                .contains(token)),
            "{text}"
        );
    }
    assert!(text.contains("[auto|split|unified]"));
    assert_eq!(s.diff_preference, Preference::Auto);
    key(&mut s, KeyCode::Down);
    key(&mut s, KeyCode::Tab);
    assert_eq!(s.bar.text(), "/repo diff layout split");
    assert_eq!(s.diff_preference, Preference::Auto);
    key(&mut s, KeyCode::Enter);
    assert_eq!(s.diff_preference, Preference::Split);
    wait(&mut s);
    s.activate(
        Action::DiffLayoutMenu,
        None,
        Default::default(),
        &Cancel::default(),
    )
    .unwrap();
    assert!(
        matches!(&s.bar,bar::InputBar::Command { menu: Menu::Options(values), .. } if values.options.len() == Kind::DiffLayout.options().count())
    );
    s.shutdown();
}
