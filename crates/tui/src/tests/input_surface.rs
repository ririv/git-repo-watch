use super::*;
use crate::{bar::InputBar, command::Context, search::Support, workspace::Section};

fn commands_only(s: &mut Screen) {
    assert_eq!(s.search_support(), Support::Unavailable);
    assert!(matches!(s.bar, InputBar::CommandsOnly), "{:?}", s.bar);
    let repository = s.scope_path();
    let query = s.query.clone();
    let page = s.page;
    for width in [60, 160] {
        let text = render(s, width, 32);
        let input = s.input_area.unwrap();
        let lines = text.lines().collect::<Vec<_>>();
        assert!(
            !lines[input.y as usize].contains(i18n::tr!(CommandsAlt).trim()),
            "{text}"
        );
        assert!(
            lines[input.y as usize + 1].contains(i18n::tr!(SearchCommandOnly)),
            "{text}"
        );
        assert!(lines[input.y as usize + 1].contains('›'), "{text}");
        if matches!(s.mode, Mode::Panel(Section::Overview))
            || (matches!(s.mode, Mode::Browsing) && s.page == Page::Detail)
        {
            assert!(
                lines[input.bottom() as usize].contains(i18n::tr!(OverviewInputHint)),
                "{text}"
            );
        }
    }
    s.paste("ignored text 中文");
    for c in "ignored".chars() {
        key(s, KeyCode::Char(c));
    }
    assert!(matches!(s.bar, InputBar::CommandsOnly));
    assert!(s.bar.text().is_empty());
    assert_eq!(s.query, query);
    assert_eq!(s.page, page);
    assert_eq!(s.scope_path(), repository);
    let context = s.command_context();
    key(s, KeyCode::Char('/'));
    assert_eq!(s.bar.command_input().unwrap().context(), context);
    key(s, KeyCode::Esc);
    assert!(matches!(s.bar, InputBar::CommandsOnly));
}

#[test]
fn repository_overview_only_accepts_commands_and_restores_the_home_search_on_return() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let f = support::Fixture::create();
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        s.paste("repo");
        let query = s.query.clone();
        key(&mut s, KeyCode::Enter);
        wait_workspace(&mut s);
        super::overview::wait_overview(&mut s);
        assert_eq!(s.command_context(), Some(Context::Repository));
        commands_only(&mut s);
        s.switch_repository_section(Section::Overview);
        commands_only(&mut s);
        if language == i18n::Language::Chinese {
            key(&mut s, KeyCode::Char('、'));
            assert_eq!(
                s.bar.command_input().unwrap().context(),
                Some(Context::Repository)
            );
            key(&mut s, KeyCode::Esc);
        }
        command(&mut s, "repos");
        assert!(matches!(s.bar, InputBar::Search(_)));
        assert_eq!(s.bar.text(), query);
        assert_eq!(s.selected_path(), Some(f.repo));
        s.shutdown();
    }
}

#[test]
fn static_pages_and_read_only_details_use_the_same_command_entry() {
    let f = support::Fixture::create();
    f.commit("tracked.txt", "initial\n");
    support::write(f.repo.join("tracked.txt"), "changed\n");
    support::git(&f.repo, &["stash", "push", "-m", "saved"]);
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        for entry in ["settings", "schedule", "repos manage", "help"] {
            command(&mut s, entry);
            commands_only(&mut s);
        }
        for entry in ["repo branch", "repo stash"] {
            command(&mut s, entry);
            wait(&mut s);
            key(&mut s, KeyCode::Enter);
            commands_only(&mut s);
            key(&mut s, KeyCode::Esc);
            assert!(matches!(s.bar, InputBar::Search(_)));
        }
        s.shutdown();
    }
}

#[test]
fn preview_focus_uses_command_entry_and_keeps_the_list_query_for_keyboard_and_pointer() {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    let f = support::Fixture::create();
    f.commit("tracked.txt", "initial\n");
    support::write(f.repo.join("tracked.txt"), "changed\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo diff");
    wait(&mut s);
    s.paste("tracked");
    key(&mut s, KeyCode::Tab);
    commands_only(&mut s);
    assert_eq!(s.mode.file_session().unwrap().query, "tracked");
    key(&mut s, KeyCode::Tab);
    assert_eq!(s.bar.text(), "tracked");
    render(&mut s, 160, 32);
    let Mode::Changes(session) = &s.mode else {
        panic!()
    };
    let areas = view::changes::areas(s.body_area.unwrap(), session);
    let preview = areas
        .iter()
        .find(|(_, focus)| *focus == crate::changes::Focus::Preview)
        .unwrap()
        .0;
    let files = areas
        .iter()
        .find(|(_, focus)| *focus == crate::changes::Focus::Files)
        .unwrap()
        .0;
    s.mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: preview.x + 3,
            row: preview.y + 3,
            modifiers: KeyModifiers::NONE,
        },
        &Cancel::default(),
    )
    .unwrap();
    commands_only(&mut s);
    render(&mut s, 160, 32);
    s.mouse(
        MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: files.x + 3,
            row: files.y + 3,
            modifiers: KeyModifiers::NONE,
        },
        &Cancel::default(),
    )
    .unwrap();
    assert_eq!(s.bar.text(), "tracked");
    command(&mut s, "repo files");
    super::browser::ready(&mut s);
    s.paste("tracked");
    super::browser::ready(&mut s);
    key(&mut s, KeyCode::Enter);
    commands_only(&mut s);
    key(&mut s, KeyCode::Esc);
    assert_eq!(s.bar.text(), "tracked");
    assert_eq!(super::browser::session(&mut s).query, "tracked");
    s.shutdown();
}

#[test]
fn command_entry_keeps_cursor_and_prompt_while_the_title_tracks_command_editing() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec![]);
        command(&mut s, "help");
        let mut terminal = Terminal::new(TestBackend::new(100, 32)).unwrap();
        terminal.draw(|frame| view::render(frame, &mut s)).unwrap();
        assert!(terminal.backend().cursor_visible());
        let area = s.input_area.unwrap();
        assert_eq!(
            terminal.backend().buffer()[(area.x, area.y)].fg,
            ratatui::style::Color::Rgb(180, 145, 220)
        );
        key(&mut s, KeyCode::Char('/'));
        terminal.draw(|frame| view::render(frame, &mut s)).unwrap();
        assert!(terminal.backend().cursor_visible());
        let text = render(&mut s, 100, 32);
        let area = s.input_area.unwrap();
        assert!(
            text.lines()
                .nth(area.y as usize)
                .unwrap()
                .contains(i18n::tr!(CommandsAlt).trim())
        );
        key(&mut s, KeyCode::Esc);
        terminal.draw(|frame| view::render(frame, &mut s)).unwrap();
        assert!(terminal.backend().cursor_visible());
        s.shutdown();
    }
}

#[test]
fn removing_or_replacing_context_prefix_clears_every_old_background_cell() {
    use ratatui::{backend::Backend, style::Color};
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        for width in [76, 100, 180] {
            let (_home, mut s) = fixture(vec![]);
            let mut terminal = Terminal::new(TestBackend::new(width, 40)).unwrap();
            for (keys, expected) in [
                (vec![KeyCode::Char('/')], "/"),
                (
                    vec![KeyCode::Backspace, KeyCode::Left, KeyCode::Char('/')],
                    "/",
                ),
                (
                    vec![
                        KeyCode::Backspace,
                        KeyCode::Backspace,
                        KeyCode::Backspace,
                        KeyCode::Backspace,
                    ],
                    "",
                ),
                (
                    vec![KeyCode::Backspace, KeyCode::Left, KeyCode::Backspace],
                    "",
                ),
                (
                    vec![KeyCode::Backspace, KeyCode::Left, KeyCode::Char('s')],
                    "/s",
                ),
            ] {
                s.bar = InputBar::Command {
                    input: crate::command::Input::new(Some(Context::Diff(
                        crate::command::context::DiffContext::Worktree,
                    ))),
                    selected: 0,
                    menu: Default::default(),
                    path: None,
                    selection: Default::default(),
                };
                terminal.draw(|frame| view::render(frame, &mut s)).unwrap();
                let old_inner = view::input::content_area(s.input_area.unwrap());
                let gap = (old_inner.x + "/repo diff".len() as u16, old_inner.y);
                // 终端保留标签尾部的色块，但应用缓存里该分隔空格仍是默认背景。
                let mut stale = terminal.backend().buffer()[gap].clone();
                stale.set_bg(Color::Rgb(34, 52, 65));
                terminal
                    .backend_mut()
                    .draw(std::iter::once((gap.0, gap.1, &stale)))
                    .unwrap();
                for code in keys {
                    key(&mut s, code);
                    terminal.draw(|frame| view::render(frame, &mut s)).unwrap();
                }
                assert_eq!(s.bar.text(), expected);
                let inner = view::input::content_area(s.input_area.unwrap());
                let mut x = inner.x;
                while x < inner.right() {
                    let cell = &terminal.backend().buffer()[(x, inner.y)];
                    assert_eq!(
                        cell.bg,
                        Color::Reset,
                        "orphaned background at {x}, language={language:?}, width={width}"
                    );
                    // TestBackend 仅写入字形起始格；终端会把宽字形的样式铺满其占用的格子。
                    x += unicode_width::UnicodeWidthStr::width(cell.symbol()).max(1) as u16;
                }
                assert!(terminal.backend().cursor_visible());
            }
            s.shutdown();
        }
    }
}
