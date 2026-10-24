use super::*;
use crate::{bar::InputBar, diff::Preference};

#[test]
fn horizontal_layout_picker_applies_on_enter_cancels_to_the_draft_and_retains_the_view() {
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
        assert!(matches!(
            s.bar,
            InputBar::Layout {
                selected: Preference::Unified,
                ..
            }
        ));
        key(&mut s, KeyCode::Left);
        assert!(matches!(
            s.bar,
            InputBar::Layout {
                selected: Preference::Split,
                ..
            }
        ));
        assert_eq!(s.diff_preference, Preference::Unified);
        s.paste("ignored paste");
        assert_eq!(s.bar.text(), "/repo diff lay");
        key(&mut s, KeyCode::Esc);
        assert_eq!(s.bar.text(), "/repo diff lay");
        assert!(matches!(s.bar, InputBar::Command { .. }));
        assert!(matches!(&s.mode,Mode::Changes(session) if session.files.query==query));
        key(&mut s, KeyCode::Enter);
        key(&mut s, KeyCode::BackTab);
        key(&mut s, KeyCode::Enter);
        assert_eq!(s.diff_preference, Preference::Split);
        assert_eq!(s.bar.editor().text, query);
        assert!(matches!(&s.mode,Mode::Changes(session) if session.files.query==query));
        command(&mut s, "repo diff layout");
        key(&mut s, KeyCode::Tab);
        key(&mut s, KeyCode::Right);
        assert!(matches!(
            s.bar,
            InputBar::Layout {
                selected: Preference::Auto,
                ..
            }
        ));
        key(&mut s, KeyCode::Enter);
        assert_eq!(s.diff_preference, Preference::Auto);
        assert!(s.repository_tasks.is_empty());
        assert_eq!(
            std::fs::read(s.service.store.config_path()).unwrap(),
            configuration
        );
        assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
        assert!(s.service.store.report().unwrap().is_none());
    }
}

#[test]
fn chooser_and_current_mark_are_horizontal_at_all_supported_sizes_and_accept_mouse_selection() {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec![]);
        command(&mut s, "repo diff layout");
        for (width, height) in [(60, 18), (100, 28), (180, 36)] {
            let text = render(&mut s, width, height);
            assert!(text.contains(i18n::tr!(LayoutPickerTitle)));
            assert!(text.contains(i18n::tr!(LayoutPickerHint)));
            let inner = view::input::content_area(s.input_area.unwrap());
            let line = text.lines().nth(usize::from(inner.y)).unwrap();
            for preference in Preference::ALL {
                assert!(line.contains(preference.label()), "{text}");
            }
            assert!(
                line.contains(&format!("✓ {}", Preference::Auto.label())),
                "{text}"
            );
        }
        let inner = view::input::content_area(s.input_area.unwrap());
        let (area, _, _) = view::layout_picker::options(inner, Preference::Auto)
            .into_iter()
            .find(|(_, p, _)| *p == Preference::Split)
            .unwrap();
        assert!(
            s.mouse(
                MouseEvent {
                    kind: MouseEventKind::Down(MouseButton::Left),
                    column: area.x,
                    row: area.y,
                    modifiers: KeyModifiers::NONE
                },
                &Cancel::default()
            )
            .unwrap()
        );
        assert_eq!(s.diff_preference, Preference::Auto);
        assert!(matches!(
            s.bar,
            InputBar::Layout {
                selected: Preference::Split,
                ..
            }
        ));
        key(&mut s, KeyCode::Enter);
        assert_eq!(s.diff_preference, Preference::Split);
    }
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
                    .any(|line| line.contains("/repo diff layout") && line.contains(&expected)),
                "{text}"
            );
        }
        let (_, index) = s
            .command_items
            .iter()
            .find(|(_, index)| {
                command::matches("repo diff ")[*index].action == Action::DiffLayoutMenu
            })
            .unwrap();
        let area = s
            .command_items
            .iter()
            .find(|(_, candidate)| candidate == index)
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
        assert!(matches!(s.bar, InputBar::Layout { .. }));
        key(&mut s, KeyCode::Esc);
        assert!(matches!(s.bar, InputBar::Command { selected: 2, .. }));
        key(&mut s, KeyCode::Enter);

        key(&mut s, KeyCode::Right);
        key(&mut s, KeyCode::Enter);
        assert_eq!(s.diff_preference, Preference::Split);
        s.paste("//repo diff layout ");
        let text = render(&mut s, 100, 28);
        let current = "✓";
        assert!(
            text.lines()
                .any(|line| line.contains("/repo diff layout split") && line.contains(current)),
            "{text}"
        );
        assert!(
            !text
                .lines()
                .any(|line| line.contains("/repo diff layout unified") && line.contains(current))
        );
        assert!(s.repository_tasks.is_empty());
    }
}

#[test]
fn layout_state_is_available_without_a_diff_and_historical_previews_resolve_their_own_width() {
    let (_home, mut s) = fixture(vec![]);
    s.paste("//repo diff ");
    let text = render(&mut s, 100, 28);
    assert!(
        text.lines()
            .any(|line| line.contains("/repo diff layout")
                && line.contains(Preference::Auto.label())),
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
