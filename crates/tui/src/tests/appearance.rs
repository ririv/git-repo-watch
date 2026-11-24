use super::*;
use crate::appearance::Background;
use ratatui::style::{Color, Modifier};

#[test]
fn light_terminal_selection_keeps_menu_and_repository_text_colors() {
    let (_home, mut s) = fixture(vec![fixture_path!("/example/alpha").into()]);
    s.paste("/com");
    let draw = |screen: &mut Screen| {
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal.draw(|frame| view::render(frame, screen)).unwrap();
        terminal.backend().buffer().clone()
    };
    s.background = Background::Dark;
    let dark = draw(&mut s);
    assert!(!s.command_items.is_empty());
    let selected = dark
        .content
        .iter()
        .enumerate()
        .filter_map(|(index, cell)| (cell.bg == view::SELECTION).then_some(index))
        .collect::<Vec<_>>();
    assert!(!selected.is_empty());
    s.background = Background::Light;
    let light = draw(&mut s);
    for index in &selected {
        assert_eq!(
            light.content[*index].symbol(),
            dark.content[*index].symbol()
        );
        assert_eq!(light.content[*index].fg, dark.content[*index].fg);
        assert_eq!(
            light.content[*index].modifier,
            dark.content[*index].modifier
        );
        assert_eq!(light.content[*index].bg, Color::Rgb(196, 219, 239));
    }
    assert!(
        selected
            .iter()
            .any(|index| dark.content[*index].fg == Color::Reset)
    );
    assert!(
        selected
            .iter()
            .any(|index| dark.content[*index].fg == view::ACCENT)
    );
    s.background = Background::Unreported;
    let unreported = draw(&mut s);
    for index in selected {
        assert_eq!(unreported.content[index].fg, dark.content[index].fg);
        assert_eq!(unreported.content[index].bg, Color::Reset);
        assert!(
            unreported.content[index]
                .modifier
                .contains(Modifier::UNDERLINED)
        );
    }
    s.shutdown();
}

#[test]
fn input_parameters_and_option_current_values_deepen_together_on_light_terminals() {
    for query in ["/search filter", "/repos filter all"] {
        let (_home, mut s) = fixture(vec![fixture_path!("/example/alpha").into()]);
        s.paste(query);
        let draw = |screen: &mut Screen| {
            let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
            terminal.draw(|frame| view::render(frame, screen)).unwrap();
            terminal.backend().buffer().clone()
        };
        s.background = Background::Dark;
        let dark = draw(&mut s);
        s.background = Background::Light;
        let light = draw(&mut s);
        for (source, plain, selected) in [
            (
                Color::LightCyan,
                Color::Rgb(0, 112, 190),
                Color::Rgb(0, 94, 180),
            ),
            (
                Color::LightGreen,
                Color::Rgb(0, 130, 94),
                Color::Rgb(0, 108, 78),
            ),
        ] {
            let cells = dark
                .content
                .iter()
                .enumerate()
                .filter(|(_, cell)| {
                    cell.fg == source
                        && cell.symbol() != " "
                        && matches!(cell.bg, Color::Reset | view::SELECTION)
                })
                .collect::<Vec<_>>();
            assert!(!cells.is_empty(), "{query}: {source:?}");
            for (index, cell) in cells {
                assert_eq!(light.content[index].symbol(), cell.symbol());
                assert_eq!(
                    light.content[index].fg,
                    if cell.bg == view::SELECTION {
                        selected
                    } else {
                        plain
                    }
                );
                assert_eq!(light.content[index].modifier, cell.modifier);
            }
        }
        s.shutdown();
    }
}
