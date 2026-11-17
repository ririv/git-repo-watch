use super::*;
use crate::{
    bar::InputBar,
    command::{
        completion::Menu,
        options::{Candidates, Value},
    },
    diff::Preference,
};

fn options(s: &Screen) -> &Candidates {
    let InputBar::Command {
        menu: Menu::Options(values),
        ..
    } = &s.bar
    else {
        panic!("layout parameter candidates");
    };
    values
}

#[test]
fn layout_parameters_apply_on_enter_and_cancel_to_the_underlying_view() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let f = support::Fixture::create();
        support::write(f.repo.join("draft.rs"), "fn draft() {}\n");
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        let configuration = std::fs::read(s.service.store.config_path()).unwrap();
        let index = std::fs::read(f.repo.join(".git/index")).unwrap();
        command(&mut s, "repo diff");
        wait(&mut s);
        s.paste("draft");
        wait(&mut s);
        command(&mut s, "repo diff layout unified");
        let query = s.mode.file_session().unwrap().query.clone();
        s.paste("//repo diff lay");
        key(&mut s, KeyCode::Enter);
        assert_eq!(
            options(&s).options[options(&s).selected].value,
            Value::DiffLayout(Preference::Unified)
        );
        key(&mut s, KeyCode::Up);
        assert_eq!(s.diff_preference, Preference::Unified);
        key(&mut s, KeyCode::Tab);
        assert_eq!(s.bar.text(), "/repo diff layout split");
        key(&mut s, KeyCode::Esc);
        assert_eq!(s.bar.text(), "/repo diff layout split");
        key(&mut s, KeyCode::Esc);
        assert_eq!(s.bar.text(), query);
        assert_eq!(s.diff_preference, Preference::Unified);
        command(&mut s, "repo diff layout");
        key(&mut s, KeyCode::Up);
        key(&mut s, KeyCode::Enter);
        assert_eq!(s.diff_preference, Preference::Split);
        assert_eq!(s.bar.text(), query);
        command(&mut s, "repo diff layout auto");
        wait(&mut s);
        assert_eq!(
            std::fs::read(s.service.store.config_path()).unwrap(),
            configuration
        );
        assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
        assert!(s.service.store.report().unwrap().is_none());
        s.shutdown();
    }
}

#[test]
fn layout_parameters_share_candidate_rendering_and_single_click_applies_the_value() {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec![]);
        command(&mut s, "repo diff layout");
        for (width, height) in [(60, 18), (100, 28), (180, 36)] {
            let text = render(&mut s, width, height);
            assert_eq!(s.command_items.len(), 3);
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
        }
        let (area, _) = s
            .command_items
            .iter()
            .find(|(_, index)| *index == 1)
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
        assert_eq!(s.diff_preference, Preference::Split);
        assert!(s.bar.is_plain());
        s.shutdown();
    }
}

#[test]
fn shift_tab_keeps_the_parameter_draft_and_left_right_edit_the_input() {
    let (_home, mut s) = fixture(vec![]);
    s.paste("//repo diff layout sp");
    let draft = s.bar.text();
    key(&mut s, KeyCode::BackTab);
    assert_eq!(s.bar.text(), draft);
    assert_eq!(options(&s).options.len(), 1);
    let cursor = s.bar.editor().cursor;
    key(&mut s, KeyCode::Left);
    assert_eq!(s.bar.editor().cursor, cursor - 1);
    assert_eq!(s.diff_preference, Preference::Auto);
    key(&mut s, KeyCode::Right);
    key(&mut s, KeyCode::Tab);
    assert_eq!(s.bar.text(), "/repo diff layout split");
    key(&mut s, KeyCode::Enter);
    assert_eq!(s.diff_preference, Preference::Split);
    s.shutdown();
}

#[test]
fn command_annotations_distinguish_auto_from_the_effective_layout_and_track_resizing() {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let f = support::Fixture::create();
        f.commit("source.rs", "fn old() {}\n");
        support::write(f.repo.join("source.rs"), "fn new() {}\n");
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        command(&mut s, "repo diff");
        wait(&mut s);
        s.paste("//repo diff ");
        for (width, actual) in [
            (180, Preference::Unified),
            (240, Preference::Split),
            (100, Preference::Unified),
        ] {
            let text = render(&mut s, width, 32);
            let expected = format!(
                "{} ({} {})",
                Preference::Auto.label(),
                i18n::tr!(LayoutVisible),
                actual.label()
            );
            assert!(
                text.lines()
                    .any(|line| line.contains("layout") && line.contains(&expected)),
                "{text}"
            );
        }
        let commands = s.search_commands().unwrap();
        let index = commands
            .iter()
            .position(|command| command.action == Action::DiffLayoutMenu)
            .unwrap();
        let area = s
            .command_items
            .iter()
            .find(|(_, candidate)| *candidate == index)
            .unwrap()
            .0;
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
        assert!(matches!(
            s.bar,
            InputBar::Command {
                menu: command::completion::Menu::Options(_),
                ..
            }
        ));
        key(&mut s, KeyCode::Esc);
        assert_eq!(s.bar.text(), "/repo diff layout ");
        key(&mut s, KeyCode::Enter);
        key(&mut s, KeyCode::Down);
        key(&mut s, KeyCode::Enter);
        assert_eq!(s.diff_preference, Preference::Split);
        s.paste("//repo diff layout ");
        let text = render(&mut s, 100, 28);
        for (index, token) in [(1, "split"), (2, "unified")] {
            let (area, _) = s
                .command_items
                .iter()
                .find(|(_, item)| *item == index)
                .unwrap();
            let line = text.lines().nth(usize::from(area.y)).unwrap();
            assert!(line.contains(token));
            assert_eq!(
                line.trim_end_matches(['│', ' ']).ends_with('✓'),
                token == "split"
            );
        }
        assert!(s.repository_tasks.is_empty());
        s.shutdown();
    }
}

#[test]
fn layout_state_is_available_without_a_diff_and_historical_previews_resolve_their_own_width() {
    let (_home, mut s) = fixture(vec![]);
    s.paste("//repo diff ");
    let text = render(&mut s, 100, 28);
    assert!(
        text.lines()
            .any(|line| line.contains("layout") && line.contains(Preference::Auto.label())),
        "{text}"
    );
    assert!(!text.contains(&format!("({} ", i18n::tr!(LayoutVisible))));
    let f = support::Fixture::create();
    f.commit("source.rs", "fn initial() {}\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo log");
    wait(&mut s);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    s.paste("//repo diff ");
    for (width, actual) in [(180, Preference::Split), (100, Preference::Unified)] {
        let text = render(&mut s, width, 32);
        assert!(
            text.contains(&format!(
                "{} ({} {})",
                Preference::Auto.label(),
                i18n::tr!(LayoutVisible),
                actual.label()
            )),
            "{text}"
        );
    }
}
