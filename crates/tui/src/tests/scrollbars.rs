use super::*;

#[test]
fn repository_scrollbar_only_appears_when_rows_overflow_after_resize() {
    let (_home, mut s) = fixture(
        (0..26)
            .map(|i| PathBuf::from(format!("/example/repo{i:02}")))
            .collect(),
    );
    for height in [40, 18, 40] {
        let mut terminal = Terminal::new(TestBackend::new(86, height)).unwrap();
        terminal.draw(|frame| view::render(frame, &mut s)).unwrap();
        let area = s.repository_area.unwrap();
        let buffer = terminal.backend().buffer();
        let scrollable = (area.y + 1..area.bottom() - 1)
            .any(|y| buffer[(area.right() - 1, y)].fg == crate::scrollbar::THUMB_COLOR);
        assert_eq!(scrollable, height == 18);
    }
    s.shutdown();
}

#[test]
fn file_groups_keep_independent_tracks_and_headers_when_navigating() {
    let f = support::Fixture::create();
    for index in 0..20 {
        support::write(
            f.repo.join(format!("staged-{index:02}.rs")),
            "fn staged() {}\n",
        );
    }
    support::git(&f.repo, &["add", "."]);
    for index in 0..20 {
        support::write(
            f.repo.join(format!("unstaged-{index:02}.rs")),
            "fn unstaged() {}\n",
        );
    }
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo diff");
    wait(&mut s);
    for (width, height) in [(60, 24), (160, 32)] {
        if let Mode::Changes(session) = &mut s.mode {
            session.files.offsets = [0; 2];
        }
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|frame| view::render(frame, &mut s)).unwrap();
        let Mode::Changes(session) = &s.mode else {
            panic!("changes")
        };
        let (area, _) = view::changes::areas(s.body_area.unwrap(), session)
            .into_iter()
            .find(|(_, focus)| *focus == crate::changes::Focus::Files)
            .unwrap();
        let buffer = terminal.backend().buffer();
        let mut groups = 0;
        for y in area.y + 1..area.bottom() - 1 {
            let line: String = (area.x..area.right())
                .map(|x| buffer[(x, y)].symbol())
                .collect();
            if line.contains(i18n::tr!(ChangesStaged)) || line.contains(i18n::tr!(ChangesUnstaged))
            {
                groups += 1;
                assert_eq!(buffer[(area.right() - 1, y)].symbol(), "│");
                assert_eq!(buffer[(area.right() - 1, y + 1)].symbol(), "┃");
                assert_eq!(
                    buffer[(area.right() - 1, y + 1)].fg,
                    crate::scrollbar::THUMB_COLOR
                );
            }
        }
        assert_eq!(groups, 2);
        key(&mut s, KeyCode::End);
        terminal.draw(|frame| view::render(frame, &mut s)).unwrap();
        assert_eq!(
            terminal.backend().buffer()[(area.right() - 1, area.bottom() - 2)].symbol(),
            "┃"
        );
        key(&mut s, KeyCode::Home);
    }
    s.shutdown();
}
