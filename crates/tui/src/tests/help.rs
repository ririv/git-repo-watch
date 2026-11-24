use super::*;

#[test]
fn help_resources_wrap_without_markup_and_reach_the_last_section_after_resizing() {
    let text_without_wrap = |text: &str| {
        text.chars()
            .filter(|character| !character.is_whitespace() && !matches!(character, '│' | '┃'))
            .collect::<String>()
    };
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec![]);
        command(&mut s, "help");
        let title = match language {
            i18n::Language::English => "Quick navigation",
            i18n::Language::Chinese => "快速导航",
        };
        #[cfg(feature = "developer-scheduler")]
        let last = match language {
            i18n::Language::English => "Manual checks still fetch and read local status only.",
            i18n::Language::Chinese => "本地读取。",
        };
        #[cfg(not(feature = "developer-scheduler"))]
        let last = "APP_LANG=zh";
        let last = text_without_wrap(last);
        for (width, height) in [(60, 18), (100, 28), (150, 32)] {
            s.scroll = 0;
            let top = render(&mut s, width, height);
            assert!(top.contains(title), "{top}");
            assert!(top.contains("Enter"));
            assert!(!top.contains('`'));
            assert!(!top.contains("## "));
            s.scroll = usize::MAX;
            let bottom = render(&mut s, width, height);
            assert!(text_without_wrap(&bottom).contains(&last), "{bottom}");
            #[cfg(not(feature = "developer-scheduler"))]
            assert!(bottom.contains("APP_LANG=en"), "{bottom}");
            assert!(s.scroll > 0 && s.scroll < usize::MAX);
            key(&mut s, KeyCode::PageUp);
            assert!(!text_without_wrap(&render(&mut s, width, height)).contains(&last));
        }
        s.scroll = usize::MAX;
        render(&mut s, 60, 18);
        let narrow_scroll = s.scroll;
        assert!(text_without_wrap(&render(&mut s, 150, 32)).contains(&last));
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
            assert_eq!(first.fg, crate::scrollbar::THUMB_COLOR);
            assert_eq!(first.symbol(), "┃");
            s.scroll = usize::MAX;
            terminal.draw(|frame| view::render(frame, &mut s)).unwrap();
            let last = &terminal.backend().buffer()[(area.right() - 1, area.bottom() - 2)];
            assert_eq!(last.symbol(), first.symbol());
            assert_eq!(last.fg, crate::scrollbar::THUMB_COLOR);
        }
    }
}
