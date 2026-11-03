use super::*;
use crate::command::completion::Menu;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

fn candidate(s: &Screen) -> usize {
    match &s.bar {
        bar::InputBar::Command { selected, menu, .. } => match menu {
            Menu::Commands => *selected,
            Menu::Options(values) => values.selected,
            Menu::Directories(values) => values.selected(),
            Menu::Values(values) => values.selected.expect("selected repository value"),
            Menu::Loading(_) | Menu::Failed(_) => panic!("interactive candidates expected"),
        },
        _ => panic!("candidate menu expected"),
    }
}

#[test]
fn candidate_menus_page_by_visible_rows_and_stop_at_both_ends() {
    for language in [i18n::Language::Chinese, i18n::Language::English] {
        let _language = i18n::using(language);
        for height in [12, 28] {
            for entry in ["repo", "repos filter ", "repos add ~/目"] {
                let (home, mut s) = fixture(vec!["/example/alpha".into()]);
                for index in 0..15 {
                    std::fs::create_dir(home.path().join(format!("目录 {index:02}"))).unwrap();
                }
                s.paste(&format!("//{entry}"));
                if entry.starts_with("repos add") {
                    complete_path(&mut s);
                }
                render(&mut s, 100, height);
                let count = match &s.bar {
                    bar::InputBar::Command {
                        menu: Menu::Directories(values),
                        ..
                    } => values.names().len(),
                    bar::InputBar::Command {
                        menu: Menu::Options(values),
                        ..
                    } => values.options.len(),
                    _ => s.search_commands().unwrap().len(),
                };
                let draft = s.bar.text();
                let before = std::fs::read(s.service.store.config_path()).unwrap();
                key(&mut s, KeyCode::Down);
                render(&mut s, 100, height);
                let start = candidate(&s);
                let visible = s.command_items.len();
                assert!(visible > 0);
                key(&mut s, KeyCode::PageDown);
                assert_eq!(candidate(&s), (start + visible).min(count - 1));
                render(&mut s, 100, height);
                assert!(
                    s.command_items
                        .iter()
                        .any(|(_, index)| *index == candidate(&s))
                );
                let next = candidate(&s);
                key(&mut s, KeyCode::PageUp);
                assert_eq!(candidate(&s), next.saturating_sub(visible));
                for _ in 0..count {
                    key(&mut s, KeyCode::PageDown);
                    render(&mut s, 100, height);
                }
                assert_eq!(candidate(&s), count - 1);
                for _ in 0..count {
                    key(&mut s, KeyCode::PageUp);
                    render(&mut s, 100, height);
                }
                assert_eq!(candidate(&s), 0);
                assert_eq!(s.bar.text(), draft);
                assert_eq!(
                    std::fs::read(s.service.store.config_path()).unwrap(),
                    before
                );
                assert!(s.check_task.is_none() && s.repository_tasks.is_empty());
                s.shutdown();
            }
        }
    }
}

#[test]
fn paging_keeps_prefix_selection_and_accepts_an_empty_candidate_list() {
    let (_home, mut s) = fixture(vec![]);
    command(&mut s, "settings");
    key(&mut s, KeyCode::Char('/'));
    key(&mut s, KeyCode::Backspace);
    assert!(s.bar.command_input().unwrap().prefix_selected());
    let draft = s.bar.text();
    render(&mut s, 100, 28);
    for code in [KeyCode::PageDown, KeyCode::PageUp] {
        key(&mut s, code);
        render(&mut s, 100, 28);
        assert!(s.bar.command_input().unwrap().prefix_selected());
        assert_eq!(s.bar.text(), draft);
    }
    s.paste("//no-matching-command");
    render(&mut s, 100, 28);
    assert!(s.command_items.is_empty());
    let draft = s.bar.text();
    for code in [KeyCode::PageDown, KeyCode::PageUp] {
        key(&mut s, code);
        assert_eq!(candidate(&s), 0);
        assert_eq!(s.bar.text(), draft);
        assert!(s.message.is_empty());
    }
    s.shutdown();
}

#[test]
fn empty_option_values_reopen_after_escape_without_applying_a_choice() {
    for language in [i18n::Language::Chinese, i18n::Language::English] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec!["/example/alpha".into()]);
        let before = std::fs::read(s.service.store.config_path()).unwrap();
        for name in [
            "repos sort",
            "repos filter",
            "settings startup enabled",
            "schedule autopush",
            "settings ai agent",
        ] {
            for whitespace in [" ", "   "] {
                s.paste(&format!("//{name}{whitespace}"));
                let draft = s.bar.text();
                assert!(matches!(
                    &s.bar,
                    bar::InputBar::Command {
                        menu: Menu::Options(_),
                        ..
                    }
                ));
                key(&mut s, KeyCode::Esc);
                assert!(matches!(
                    &s.bar,
                    bar::InputBar::Command {
                        menu: Menu::Commands,
                        ..
                    }
                ));
                key(&mut s, KeyCode::Enter);
                assert!(
                    matches!(&s.bar, bar::InputBar::Command { menu: Menu::Options(values), .. } if !values.options.is_empty())
                );
                assert_eq!(s.bar.text(), draft);
                assert!(s.message.is_empty());
                assert!(matches!(s.mode, Mode::Browsing));
                assert_eq!(
                    std::fs::read(s.service.store.config_path()).unwrap(),
                    before
                );
                key(&mut s, KeyCode::Esc);
                key(&mut s, KeyCode::Esc);
            }
        }
        s.paste("//repos sort invalid");
        key(&mut s, KeyCode::Esc);
        key(&mut s, KeyCode::Enter);
        assert!(!s.message.is_empty());
        assert_eq!(s.order, Order::Priority);
        assert_eq!(
            std::fs::read(s.service.store.config_path()).unwrap(),
            before
        );
        s.shutdown();
    }
}

#[test]
fn directory_click_fills_the_visible_candidate_without_running_the_command() {
    for language in [i18n::Language::Chinese, i18n::Language::English] {
        let _language = i18n::using(language);
        for quote in ["", "\"", "'"] {
            let (home, mut s) = fixture(vec![]);
            for index in 0..15 {
                std::fs::create_dir(home.path().join(format!("目录 空格 {index:02}"))).unwrap();
            }
            let before = std::fs::read(s.service.store.config_path()).unwrap();
            s.paste(&format!("//repos add {quote}~/目{quote}"));
            complete_path(&mut s);
            render(&mut s, 100, 28);
            key(&mut s, KeyCode::PageDown);
            key(&mut s, KeyCode::PageDown);
            render(&mut s, 100, 28);
            let (area, _) = s
                .command_items
                .iter()
                .find(|(_, index)| *index == 10)
                .copied()
                .unwrap();
            let click = MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: area.x + 1,
                row: area.y,
                modifiers: KeyModifiers::NONE,
            };
            let draft = s.bar.text();
            s.config.mouse.precise_click = false;
            assert!(!s.mouse(click, &Cancel::default()).unwrap());
            assert_eq!(s.bar.text(), draft);
            s.config.mouse.precise_click = true;
            assert!(s.mouse(click, &Cancel::default()).unwrap());
            assert_eq!(
                s.bar.text(),
                format!("/repos add {quote}~/目录 空格 10/{quote}")
            );
            assert_eq!(
                s.bar.editor().cursor,
                s.bar.editor().text.len() - quote.len()
            );
            assert!(matches!(s.mode, Mode::Browsing));
            assert!(matches!(
                s.bar,
                bar::InputBar::Command {
                    menu: Menu::Commands,
                    ..
                }
            ));
            assert_eq!(
                std::fs::read(s.service.store.config_path()).unwrap(),
                before
            );
            key(&mut s, KeyCode::Enter);
            assert!(
                matches!(&s.mode, Mode::Confirm { operation: Confirmation::Configure { value, .. }, .. }
                if std::path::Path::new(value) == home.path().join("目录 空格 10"))
            );
            assert_eq!(
                std::fs::read(s.service.store.config_path()).unwrap(),
                before
            );
            assert!(s.check_task.is_none() && s.repository_tasks.is_empty());
            s.shutdown();
        }
    }
}

#[test]
fn path_punctuation_in_repository_search_agrees_for_typing_and_chunked_paste() {
    for language in [i18n::Language::Chinese, i18n::Language::English] {
        let _language = i18n::using(language);
        for enabled in [false, true] {
            for paste in [false, true] {
                let (_home, mut s) = fixture(vec!["/example/目录/中文、名字".into()]);
                s.config.chinese_command_trigger = enabled;
                for chunk in ["example", "/目录", "/中文", "、名字"] {
                    if paste {
                        s.paste(chunk);
                    } else {
                        for character in chunk.chars() {
                            key(&mut s, KeyCode::Char(character));
                        }
                    }
                }
                assert_eq!(s.query, "example/目录/中文、名字");
                assert_eq!(s.bar.text(), s.query);
                assert!(matches!(s.bar, bar::InputBar::Search(_)));
                let query = s.query.clone();
                key(&mut s, KeyCode::Home);
                key(&mut s, KeyCode::Char('/'));
                assert!(matches!(s.bar, bar::InputBar::Command { .. }));
                key(&mut s, KeyCode::Esc);
                assert_eq!(s.bar.text(), query);
                s.paste("//settings");
                assert_eq!(s.bar.text(), "/settings");
                assert_eq!(s.query, query);
                s.shutdown();
            }
        }
    }
}
