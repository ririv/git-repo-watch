use super::*;
use crate::command::{
    Context, Input,
    syntax::{self, Role},
};
use ratatui::{buffer::Buffer, style::Color};

fn parts(input: &Input) -> Vec<(String, Role)> {
    syntax::segments(input)
        .into_iter()
        .filter(|part| !part.range.is_empty())
        .map(|part| (input.editor().text[part.range].to_owned(), part.role))
        .collect()
}
fn draw(s: &mut Screen, width: u16) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(width, 28)).unwrap();
    terminal.draw(|frame| view::render(frame, s)).unwrap();
    terminal.backend().buffer().clone()
}
fn color(s: &Screen, buffer: &Buffer, word: &str) -> Color {
    let area = view::input::content_area(s.input_area.unwrap());
    let mut line = String::new();
    let mut x = area.x;
    while x < area.right() {
        let symbol = buffer[(x, area.y)].symbol();
        line.push_str(symbol);
        x += unicode_width::UnicodeWidthStr::width(symbol).max(1) as u16;
    }
    let index = line
        .find(word)
        .unwrap_or_else(|| panic!("{word} not in {line}"));
    buffer[(
        area.x + unicode_width::UnicodeWidthStr::width(&line[..index]) as u16,
        area.y,
    )]
        .fg
}

#[test]
fn command_roles_follow_declared_parameters_instead_of_word_positions() {
    for (text, expected) in [
        (
            "/repo branch create 功能/中文",
            vec![
                ("/repo branch create ", Role::Command),
                ("功能/中文", Role::Argument),
            ],
        ),
        (
            "/repos filter problems",
            vec![
                ("/repos filter ", Role::Command),
                ("problems", Role::Argument),
            ],
        ),
        (
            "/settings editor code --reuse-window",
            vec![
                ("/settings editor ", Role::Command),
                ("code --reuse-window", Role::Argument),
            ],
        ),
        (
            "/repo diff conflict base-result",
            vec![
                ("/repo diff conflict ", Role::Command),
                ("base-result", Role::Argument),
            ],
        ),
        ("/repo branch cr", vec![("/repo branch cr", Role::Command)]),
        (
            "/repos scan \"/中文 --exclude 路径\" --exclude /备份 /归档",
            vec![
                ("/repos scan ", Role::Command),
                ("\"/中文 --exclude 路径\" ", Role::Argument),
                ("--exclude", Role::Option),
                (" /备份 /归档", Role::Argument),
            ],
        ),
        (
            "/repos scan /根 --ex",
            vec![
                ("/repos scan ", Role::Command),
                ("/根 ", Role::Argument),
                ("--ex", Role::Option),
            ],
        ),
    ] {
        let input = Input::global(Editor::new(text.into()));
        assert_eq!(
            parts(&input),
            expected
                .into_iter()
                .map(|(text, role)| (text.to_owned(), role))
                .collect::<Vec<_>>(),
            "{text}"
        );
    }
    let mut scoped = Input::new(Some(Context::Branches));
    scoped.paste("create 功能/中文");
    assert_eq!(
        parts(&scoped),
        [
            ("create ".into(), Role::Command),
            ("功能/中文".into(), Role::Argument)
        ]
    );
}

#[test]
fn editable_commands_and_values_use_distinct_adaptive_colors_while_search_stays_plain() {
    let (_home, mut s) = fixture(vec![]);
    s.paste("/repos scan /中文 --exclude /备份");
    for (background, argument_color) in [
        (crate::appearance::Background::Dark, Color::LightCyan),
        (
            crate::appearance::Background::Light,
            Color::Rgb(0, 112, 190),
        ),
    ] {
        s.background = background;
        let buffer = draw(&mut s, 100);
        assert_eq!(color(&s, &buffer, "/repos"), view::ACCENT);
        assert_eq!(color(&s, &buffer, "/中文"), argument_color);
        assert_eq!(color(&s, &buffer, "--exclude"), Color::Yellow);
        assert_eq!(color(&s, &buffer, "/备份"), argument_color);
    }
    key(&mut s, KeyCode::Esc);
    s.paste("repos scan /中文");
    let buffer = draw(&mut s, 100);
    assert_eq!(color(&s, &buffer, "repos"), Color::Reset);
    s.shutdown();
}

#[test]
fn scoped_and_scrolled_input_keeps_parameter_roles_and_the_original_editing_geometry() {
    let (_home, mut s) = fixture(vec![]);
    s.paste("/");
    let bar::InputBar::Command { input, .. } = &mut s.bar else {
        panic!();
    };
    *input = Input::new(Some(Context::Branches));
    input.paste("create 功能/中文");
    for background in [
        crate::appearance::Background::Dark,
        crate::appearance::Background::Light,
    ] {
        s.background = background;
        let buffer = draw(&mut s, 100);
        assert_eq!(color(&s, &buffer, "/repo branch"), view::ACCENT);
        assert_eq!(color(&s, &buffer, "create"), view::ACCENT);
        assert_eq!(
            color(&s, &buffer, "功能/中文"),
            match background {
                crate::appearance::Background::Light => Color::Rgb(0, 112, 190),
                _ => Color::LightCyan,
            }
        );
        let area = view::input::content_area(s.input_area.unwrap());
        assert_eq!(
            buffer[(area.x, area.y)].bg,
            match background {
                crate::appearance::Background::Light => Color::Reset,
                _ => Color::Rgb(34, 52, 65),
            }
        );
    }
    s.background = crate::appearance::Background::Dark;
    key(&mut s, KeyCode::Esc);
    s.paste(&format!(
        "//repo branch create {}repo branch create",
        "很长的路径/".repeat(8)
    ));
    let buffer = draw(&mut s, 28);
    assert_eq!(color(&s, &buffer, "repo branch create"), Color::LightCyan);
    for width in [0, 1, 8, 28, 80] {
        draw(&mut s, width);
        key(&mut s, KeyCode::Left);
        key(&mut s, KeyCode::Home);
        draw(&mut s, width);
        key(&mut s, KeyCode::End);
    }
    assert!(s.bar.text().contains("很长的路径/"));
    s.shutdown();
}
