use super::*;
use ratatui::layout::Rect;

#[test]
fn compact_input_keeps_context_drafts_and_layout_picker_clicks_at_the_drawn_position() {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec![]);
        command(&mut s, "settings");
        s.paste("/interaction");
        let draft = s.bar.text();
        for (width, height) in [(48, 12), (24, 8), (8, 4), (1, 1), (0, 0), (100, 28)] {
            draw(&mut s, width, height);
            assert_eq!(s.bar.text(), draft);
        }
        for (width, height) in [(48, 12), (48, 8), (48, 5)] {
            command(&mut s, "repo diff layout");
            draw(&mut s, width, height);
            let inner = view::input::content_area(s.input_area.unwrap());
            let (area, preference, _) = view::layout_picker::options(inner, s.diff_preference)
                .into_iter()
                .find(|(_, preference, _)| *preference == crate::diff::Preference::Split)
                .unwrap();
            assert!(!area.is_empty());
            assert!(
                s.mouse(
                    MouseEvent {
                        kind: MouseEventKind::Down(MouseButton::Left),
                        column: area.x,
                        row: area.y,
                        modifiers: KeyModifiers::NONE,
                    },
                    &Cancel::default(),
                )
                .unwrap()
            );
            assert!(
                matches!(s.bar, bar::InputBar::Layout { selected, .. } if selected == preference)
            );
            key(&mut s, KeyCode::Enter);
            assert_eq!(s.diff_preference, preference);
        }
        s.shutdown();
    }
}

fn draw(s: &mut Screen, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| view::render(frame, s)).unwrap();
    s.graphics.settle();
    terminal.draw(|frame| view::render(frame, s)).unwrap();
    s.graphics.finish_frame(&mut Vec::new()).unwrap();
    let bounds = Rect::new(0, 0, width, height);
    if terminal.backend().cursor_visible() {
        assert!(bounds.contains(terminal.backend().cursor_position()));
    }
    for area in s
        .body_area
        .iter()
        .chain(s.input_area.iter())
        .chain(s.workspace_areas.iter().map(|(area, _)| area))
        .chain(s.command_items.iter().map(|(area, _)| area))
        .chain(s.breadcrumb_items.iter().map(|(area, _)| area))
        .filter(|area| !area.is_empty())
    {
        assert_eq!(area.intersection(bounds), *area);
    }
    if width == 0 {
        return String::new();
    }
    terminal
        .backend()
        .buffer()
        .content
        .chunks(usize::from(width))
        .map(|row| {
            let mut text = String::new();
            let mut x = 0;
            while x < row.len() {
                let symbol = row[x].symbol();
                text.push_str(symbol);
                x += unicode_width::UnicodeWidthStr::width(symbol).max(1);
            }
            text
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn small_windows_keep_repository_navigation_commands_and_selection_after_resize() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(
            (0..10)
                .map(|i| PathBuf::from(format!("/example/repo{i:02}")))
                .collect(),
        );
        for (width, height) in [
            (59, 17),
            (48, 15),
            (40, 12),
            (32, 9),
            (24, 8),
            (12, 6),
            (4, 4),
            (1, 1),
            (0, 0),
            (120, 32),
        ] {
            key(&mut s, KeyCode::PageDown);
            let text = draw(&mut s, width, height);
            if width >= 24 && height >= 8 {
                assert!(text.contains("repo09"), "{width}×{height}\n{text}");
            }
            s.paste("/settings");
            let text = draw(&mut s, width, height);
            if width >= 24 && height >= 8 {
                assert!(text.contains("/settings"), "{width}×{height}\n{text}");
                assert!(!s.command_items.is_empty());
            }
            key(&mut s, KeyCode::Enter);
            assert_eq!(s.page, Page::Configuration(Configuration::General));
            draw(&mut s, width, height);
            key(&mut s, KeyCode::Esc);
            assert_eq!(s.page, Page::Repositories);
            assert_eq!(s.selected_path(), Some("/example/repo09".into()));
        }
        s.shutdown();
    }
}

#[test]
fn repository_views_and_confirmation_render_across_tiny_sizes_without_mutating_git() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let f = support::Fixture::create();
        f.commit("tracked.rs", "fn initial() {}\n");
        support::write(f.repo.join("tracked.rs"), "fn changed() {}\n");
        let index = std::fs::read(f.repo.join(".git/index")).unwrap();
        let (home, mut s) = fixture(vec![f.repo.clone()]);
        let config = std::fs::read(s.service.store.config_path()).unwrap();
        key(&mut s, KeyCode::Enter);
        wait_workspace(&mut s);
        super::overview::wait_overview(&mut s);
        for section in crate::workspace::Section::ALL {
            s.switch_repository_section(section);
            for (width, height) in [(48, 15), (32, 9), (8, 4), (1, 1), (0, 0)] {
                draw(&mut s, width, height);
                if height > 1 {
                    assert_eq!(s.workspace_areas, vec![(s.body_area.unwrap(), section)]);
                }
            }
            draw(&mut s, 140, 32);
            assert_eq!(s.workspace_areas.len(), 4);
            assert_eq!(crate::workspace::Section::of(&s.mode), section);
        }
        for entry in [
            "repo overview",
            "repo branch",
            "repo log",
            "repo stash",
            "repo diff",
            "help",
            "settings",
            "schedule",
            "repos remove",
        ] {
            command(&mut s, entry);
            wait(&mut s);
            for (width, height) in [
                (48, 15),
                (32, 9),
                (16, 6),
                (2, 2),
                (1, 1),
                (0, 0),
                (140, 32),
            ] {
                draw(&mut s, width, height);
            }
            if entry == "repo diff" {
                key(&mut s, KeyCode::Enter);
                wait(&mut s);
                for (width, height) in [(48, 15), (32, 9), (16, 6), (1, 1), (0, 0)] {
                    draw(&mut s, width, height);
                }
            }
        }
        assert!(matches!(s.mode, Mode::Confirm { .. }));
        key(&mut s, KeyCode::Esc);
        assert_eq!(s.config.repositories, vec![f.repo.clone()]);
        std::fs::create_dir(home.path().join("中文目录")).unwrap();
        s.paste(&format!("//repos add {}/", home.path().display()));
        key(&mut s, KeyCode::Tab);
        for (width, height) in [(48, 15), (24, 8), (1, 1), (0, 0)] {
            draw(&mut s, width, height);
        }
        assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
        assert_eq!(
            std::fs::read(s.service.store.config_path()).unwrap(),
            config
        );
        s.shutdown();
    }
}

#[test]
fn document_editor_uses_its_own_cursor_and_footer_then_restores_the_command_bar() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec![]);
        command(&mut s, "settings ai prompt");
        let text = draw(&mut s, 100, 28);
        assert!(text.contains("Write a Git commit message"));
        assert!(text.contains("Ctrl+S"));
        assert!(!text.contains('╭'));
        assert!(s.input_area.is_none());
        assert_eq!(s.body_area.unwrap().bottom(), 26);
        let empty_result_body = s.body_area;
        let Mode::PromptEditor(editor) = &s.mode else {
            panic!("prompt editor");
        };
        let draft = editor.prompt();
        for (width, height) in [(48, 15), (24, 8), (8, 4), (1, 1), (0, 0), (100, 28)] {
            draw(&mut s, width, height);
            assert!(s.input_area.is_none());
            assert!(matches!(&s.mode, Mode::PromptEditor(editor) if editor.prompt() == draft));
        }
        s.message = "invalid prompt".into();
        let text = draw(&mut s, 100, 28);
        assert!(text.contains("invalid prompt") && text.contains("Ctrl+S"));
        assert_eq!(s.body_area, empty_result_body);
        s.paste("新");
        assert!(s.message.is_empty());
        key(&mut s, KeyCode::Esc);
        draw(&mut s, 100, 28);
        assert!(s.input_area.is_some());
        s.shutdown();
    }
}
