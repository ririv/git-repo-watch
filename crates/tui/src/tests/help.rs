use super::*;
use ratatui::style::Color;

#[test]
fn help_resources_wrap_without_markup_and_reach_the_last_section_after_resizing() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec![]);
        command(&mut s, "help");
        let title = match language {
            i18n::Language::English => "Quick navigation",
            i18n::Language::Chinese => "快速导航",
        };
        for (width, height) in [(60, 18), (100, 28), (150, 32)] {
            s.scroll = 0;
            let top = render(&mut s, width, height);
            assert!(top.contains(title), "{top}");
            assert!(top.contains("Enter"));
            assert!(!top.contains('`'));
            assert!(!top.contains("## "));
            s.scroll = usize::MAX;
            let bottom = render(&mut s, width, height);
            assert!(bottom.contains("APP_LANG=en"), "{bottom}");
            assert!(bottom.contains("APP_LANG=zh"), "{bottom}");
            assert!(s.scroll > 0 && s.scroll < usize::MAX);
            key(&mut s, KeyCode::PageUp);
            assert!(!render(&mut s, width, height).contains("APP_LANG=zh"));
        }
        s.scroll = usize::MAX;
        render(&mut s, 60, 18);
        let narrow_scroll = s.scroll;
        assert!(render(&mut s, 150, 32).contains("APP_LANG=zh"));
        assert!(s.scroll < narrow_scroll);
        key(&mut s, KeyCode::Left);
        assert_eq!(s.page, Page::Repositories);
        assert!(s.service.store.report().unwrap().is_none());
    }
}

#[test]
fn help_scrollbar_tracks_the_first_and_last_visible_pages_in_both_languages() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec![]);
        command(&mut s, "help");
        for (width, height) in [(60, 18), (100, 28)] {
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            s.scroll = 0;
            terminal.draw(|frame| view::render(frame, &mut s)).unwrap();
            let area = s.body_area.unwrap();
            let first = terminal.backend().buffer()[(area.right() - 1, area.y + 1)].clone();
            assert_eq!(first.fg, Color::LightBlue);
            s.scroll = usize::MAX;
            terminal.draw(|frame| view::render(frame, &mut s)).unwrap();
            let last = &terminal.backend().buffer()[(area.right() - 1, area.bottom() - 2)];
            assert_eq!(last.symbol(), first.symbol());
            assert_eq!(last.fg, Color::LightBlue);
        }
    }
}
