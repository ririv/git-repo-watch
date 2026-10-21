use super::*;
use ratatui::style::Color;

#[test]
fn highlighted_diff_keeps_styles_scrolling_and_navigation_after_resizing() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let f = support::Fixture::create();
        f.commit("中文 source.rs", "fn old() { let s = \"旧内容\"; }\n");
        let mut source = "fn changed() { let s = \"新内容\"; }\n".to_owned();
        source.push_str(&"// many lines\n".repeat(80));
        source.push_str("fn last_line() {}\n");
        support::write(f.repo.join("中文 source.rs"), &source);
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        command(&mut s, "stage");
        wait(&mut s);
        key(&mut s, KeyCode::Enter);
        wait(&mut s);
        assert!(matches!(s.mode, Mode::Diff { .. }));
        for (width, height) in [(100, 28), (60, 18), (150, 32)] {
            let Mode::Diff { scroll, .. } = &mut s.mode else {
                unreachable!()
            };
            *scroll = 0;
            let mut text = render(&mut s, width, height);
            assert!(text.contains("Rust"), "{text}");
            for _ in 0..12 {
                if text.contains("+fn changed()") {
                    break;
                }
                key(&mut s, KeyCode::Down);
                text = render(&mut s, width, height);
            }
            assert!(text.contains("+fn changed()"), "{text}");
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal.draw(|frame| view::render(frame, &mut s)).unwrap();
            let cells = &terminal.backend().buffer().content;
            assert!(cells.iter().any(|cell| cell.bg == crate::diff::ADDED));
            assert!(cells.iter().any(|cell| cell.bg == crate::diff::REMOVED));
            let source_colors = cells
                .iter()
                .filter(|cell| cell.bg == crate::diff::ADDED && matches!(cell.fg, Color::Rgb(..)))
                .map(|cell| cell.fg)
                .collect::<std::collections::HashSet<_>>();
            assert!(source_colors.len() >= 3);
            let Mode::Diff { scroll, .. } = &mut s.mode else {
                unreachable!()
            };
            *scroll = usize::MAX;
            assert!(render(&mut s, width, height).contains("last_line"));
            key(&mut s, KeyCode::PageUp);
            assert!(!render(&mut s, width, height).contains("last_line"));
        }
        key(&mut s, KeyCode::Left);
        assert!(matches!(s.mode, Mode::Files(_)));
        assert!(support::git(&f.repo, &["diff", "--cached"]).is_empty());
        assert!(s.service.store.report().unwrap().is_none());
    }
}
