use super::*;
use crate::pointer::{Hit, Target};
use ratatui::{layout::Position, style::Color};

#[test]
fn settings_values_align_and_have_distinct_semantic_colors() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec![]);
        command(&mut s, "settings");
        s.config.mouse.scroll = false;
        let mut terminal = Terminal::new(TestBackend::new(120, 34)).unwrap();
        terminal.draw(|frame| view::render(frame, &mut s)).unwrap();
        let buffer = terminal.backend().buffer();
        let mut columns = std::collections::BTreeSet::new();
        let mut states = std::collections::BTreeSet::new();
        for y in 0..34 {
            for x in 0..120 {
                let cell = &buffer[(x, y)];
                if cell.symbol() == "●" || cell.symbol() == "○" {
                    columns.insert(x);
                    states.insert(cell.symbol().to_owned());
                    assert_eq!(
                        cell.fg,
                        if cell.symbol() == "●" {
                            Color::LightGreen
                        } else {
                            Color::Gray
                        }
                    );
                }
            }
        }
        assert_eq!(columns.len(), 1);
        assert_eq!(states.len(), 2);
        assert!(
            buffer
                .content
                .iter()
                .any(|cell| cell.fg == Color::LightMagenta)
        );
        assert!(
            buffer
                .content
                .iter()
                .any(|cell| cell.fg == Color::LightCyan)
        );
    }
}

#[test]
fn settings_description_rows_select_their_item_and_manual_scroll_is_preserved() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec![]);
        command(&mut s, "settings");
        for width in [60, 80, 140] {
            s.setting_selected = Configuration::General
                .items(&s.snapshot, &s.config.ai)
                .len()
                - 1;
            render(&mut s, width, 18);
            let selected = s.setting_selected;
            let positions = (0..18)
                .flat_map(|y| (0..width).map(move |x| Position::new(x, y)))
                .filter(|position| {
                    s.pointer.hit(*position) == Some(Hit::Item(Target::Settings, selected))
                })
                .collect::<Vec<_>>();
            assert!(
                positions
                    .iter()
                    .map(|p| p.y)
                    .collect::<std::collections::BTreeSet<_>>()
                    .len()
                    >= 2,
                "description must belong to the item"
            );
            s.scroll = 0;
            let top = render(&mut s, width, 18);
            assert_eq!(s.scroll, 0);
            assert!(top.contains(i18n::tr!(SettingsChecks)));
            key(&mut s, KeyCode::Up);
            render(&mut s, width, 18);
            assert!(s.scroll > 0);
        }
    }
}
