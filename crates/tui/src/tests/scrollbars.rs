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
fn file_groups_share_one_track_and_scroll_headers_with_files() {
    use ratatui::layout::{Position, Rect};
    let f = support::Fixture::create();
    support::write(f.repo.join("staged.rs"), "staged\n");
    support::git(&f.repo, &["add", "."]);
    for index in 0..47 {
        support::write(f.repo.join(format!("working-{index:02}.rs")), "working\n");
    }
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    for in_workspace in [false, true] {
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        if in_workspace {
            key(&mut s, KeyCode::Enter);
            wait_workspace(&mut s);
            s.switch_repository_section(crate::workspace::Section::Worktree);
        } else {
            open_files(&mut s);
        }
        s.file_layout = crate::files::Layout::Flat;
        for (width, height) in [(60, 24), (160, 32)] {
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            key(&mut s, KeyCode::Home);
            terminal.draw(|frame| view::render(frame, &mut s)).unwrap();
            let area = if in_workspace {
                s.workspace_areas
                    .iter()
                    .find(|(_, section)| *section == crate::workspace::Section::Worktree)
                    .unwrap()
                    .0
            } else {
                s.body_area.unwrap()
            };
            let buffer = terminal.backend().buffer();
            let text = (area.y + 1..area.bottom() - 1)
                .map(|y| {
                    (area.x + 1..area.right() - 1)
                        .map(|x| buffer[(x, y)].symbol())
                        .collect::<String>()
                })
                .collect::<Vec<_>>();
            assert!(text[0].contains(i18n::tr!(ChangesStaged)));
            assert!(text[1].contains("staged.rs"));
            assert!(text[2].contains(i18n::tr!(ChangesUnstaged)));
            assert_eq!(
                text.iter().filter(|line| line.contains("working-")).count(),
                text.len() - 3
            );
            assert_eq!(s.pointer.hit(Position::new(area.x + 5, area.y + 1)), None);
            let track = crate::scrollbar::vertical_area(
                area,
                Rect::new(area.x + 1, area.y + 1, area.width - 2, area.height - 2),
            );
            for y in track.y..track.bottom() {
                assert!(matches!(
                    s.pointer.hit(Position::new(track.x, y)),
                    Some(crate::pointer::Hit::Scrollbar(
                        crate::pointer::Action::Scroll(crate::pointer::ScrollTarget::Files, _)
                    ))
                ));
            }
            key(&mut s, KeyCode::Down);
            assert_eq!(
                s.mode.file_session().unwrap().current().unwrap().area,
                Area::Untracked
            );
            key(&mut s, KeyCode::Up);
            assert_eq!(
                s.mode.file_session().unwrap().current().unwrap().area,
                Area::Staged
            );
            key(&mut s, KeyCode::End);
            let end = render(&mut s, width, height);
            assert!(end.contains("working-46.rs"));
            assert!(!end.contains(i18n::tr!(ChangesStaged)));
            let offset = s.mode.file_session().unwrap().offset;
            assert!(offset > 0);
            key(&mut s, KeyCode::Up);
            assert!(render(&mut s, width, height).contains("working-45.rs"));
            key(&mut s, KeyCode::Home);
            let top = render(&mut s, width, height);
            assert!(top.contains(i18n::tr!(ChangesStaged)));
            assert_eq!(s.mode.file_session().unwrap().offset, 0);
        }
        s.shutdown();
    }
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
}

#[test]
fn file_groups_hide_the_track_when_the_combined_list_fits() {
    let f = support::Fixture::create();
    support::write(f.repo.join("staged.txt"), "staged\n");
    support::git(&f.repo, &["add", "."]);
    support::write(f.repo.join("working.txt"), "working\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    open_files(&mut s);
    let mut terminal = Terminal::new(TestBackend::new(100, 32)).unwrap();
    terminal.draw(|frame| view::render(frame, &mut s)).unwrap();
    let area = s.body_area.unwrap();
    assert!(
        (area.y + 1..area.bottom() - 1)
            .all(|y| terminal.backend().buffer()[(area.right() - 1, y)].fg
                != crate::scrollbar::THUMB_COLOR)
    );
    s.shutdown();
}

#[test]
fn first_file_reveals_its_group_heading_from_a_scrolled_viewport_in_every_scene() {
    use crate::files::{Group, Layout};
    use ratatui::layout::Position;
    enum Scene {
        Files,
        Workspace,
    }
    let f = support::Fixture::create();
    support::write(f.repo.join("staged.txt"), "staged\n");
    support::git(&f.repo, &["add", "."]);
    for index in 0..47 {
        support::write(f.repo.join(format!("working-{index:02}.txt")), "working\n");
    }
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    for scene in [Scene::Files, Scene::Workspace] {
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        match scene {
            Scene::Files => open_files(&mut s),
            Scene::Workspace => {
                key(&mut s, KeyCode::Enter);
                wait_workspace(&mut s);
                key(&mut s, KeyCode::Down);
                key(&mut s, KeyCode::Enter);
                wait(&mut s);
            }
        }
        for layout in [Layout::Flat, Layout::Tree] {
            s.file_layout = layout;
            for group in [Group::Staged, Group::Unstaged] {
                let files = match &mut s.mode {
                    Mode::Files(files) => files,
                    _ => panic!("file panel"),
                };
                let first = files.entries(layout)[group.index()][0].cursor(group);
                files.cursor = first;
                // 复现滚动后第一项恰好位于视口顶端、标题在上一行的情况。
                files.offset = files
                    .list(layout)
                    .iter()
                    .position(|row| row.cursor().as_ref() == Some(&files.cursor))
                    .unwrap();
                let text = render(&mut s, 160, 40);
                let area = match scene {
                    Scene::Files => s.body_area.unwrap(),
                    Scene::Workspace => {
                        s.workspace_areas
                            .iter()
                            .find(|(_, section)| *section == crate::workspace::Section::Worktree)
                            .unwrap()
                            .0
                    }
                };
                let title = match group {
                    Group::Staged => i18n::tr!(ChangesStaged),
                    Group::Unstaged => i18n::tr!(ChangesUnstaged),
                };
                let line = text.lines().nth(usize::from(area.y + 1)).unwrap();
                assert!(line.contains(title), "{text}");
                assert_eq!(s.pointer.hit(Position::new(area.x + 5, area.y + 1)), None);
                assert!(matches!(
                    s.pointer.hit(Position::new(area.x + 5, area.y + 2)),
                    Some(crate::pointer::Hit::Item(crate::pointer::Target::Files, _))
                ));
            }
        }
        s.shutdown();
    }
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
}

#[test]
fn empty_file_groups_hide_after_filtering_staging_unstaging_and_refresh() {
    use crate::{
        files::{Cursor, Layout},
        pointer::{Hit, Target},
    };
    use ratatui::layout::Position;
    fn refresh(s: &mut Screen, path: &std::path::Path) {
        let snapshot = s.service.files(path, &Cancel::default()).unwrap();
        let Mode::Files(files) = &mut s.mode else {
            panic!("files")
        };
        *files = files.refreshed(snapshot);
    }
    fn search(s: &mut Screen, query: &str) {
        let Mode::Files(files) = &mut s.mode else {
            panic!("files")
        };
        files.search(query.into());
    }
    for language in [i18n::Language::Chinese, i18n::Language::English] {
        let _language = i18n::using(language);
        for layout in [Layout::Flat, Layout::Tree] {
            let f = support::Fixture::create();
            let (_home, mut s) = fixture(vec![f.repo.clone()]);
            open_files(&mut s);
            s.file_layout = layout;
            let clean = render(&mut s, 100, 32);
            assert!(clean.contains(i18n::tr!(FileListClean)));
            assert!(!clean.contains(i18n::tr!(ChangesStaged)));
            assert!(!clean.contains(i18n::tr!(ChangesUnstaged)));
            support::write(f.repo.join("indexed.txt"), "index\n");
            support::git(&f.repo, &["add", "indexed.txt"]);
            support::write(f.repo.join("working.txt"), "working\n");
            refresh(&mut s, &f.repo);
            let initial_index = std::fs::read(f.repo.join(".git/index")).unwrap();
            for (query, shown, hidden) in [
                (
                    "indexed",
                    i18n::tr!(ChangesStaged),
                    i18n::tr!(ChangesUnstaged),
                ),
                (
                    "working",
                    i18n::tr!(ChangesUnstaged),
                    i18n::tr!(ChangesStaged),
                ),
            ] {
                search(&mut s, query);
                let text = render(&mut s, 100, 32);
                assert!(text.contains(shown), "{text}");
                assert!(!text.contains(hidden), "{text}");
                let area = s.body_area.unwrap();
                assert_eq!(
                    s.pointer.hit(Position::new(area.x + 1, area.y + 2)),
                    Some(Hit::FileCheckbox(0))
                );
                assert_eq!(
                    s.pointer.hit(Position::new(area.x + 5, area.y + 2)),
                    Some(Hit::Item(Target::Files, 0))
                );
            }
            search(&mut s, "no-matching-file");
            let empty = render(&mut s, 100, 32);
            assert!(empty.contains(i18n::tr!(FileListNoMatches)));
            assert!(!empty.contains(i18n::tr!(ChangesStaged)));
            assert!(!empty.contains(i18n::tr!(ChangesUnstaged)));
            assert_eq!(s.mode.file_session().unwrap().cursor, Cursor::Empty);
            assert_eq!(s.mode.file_session().unwrap().offset, 0);
            assert_eq!(
                std::fs::read(f.repo.join(".git/index")).unwrap(),
                initial_index
            );
            let Mode::Files(files) = &mut s.mode else {
                panic!("files")
            };
            files.set_search_mode(crate::search::Mode::Locate);
            let locate = render(&mut s, 100, 32);
            assert!(locate.contains(i18n::tr!(ChangesStaged)));
            assert!(locate.contains(i18n::tr!(ChangesUnstaged)));
            let Mode::Files(files) = &mut s.mode else {
                panic!("files")
            };
            files.set_search_mode(crate::search::Mode::Filter);
            search(&mut s, "");
            support::git(&f.repo, &["add", "working.txt"]);
            refresh(&mut s, &f.repo);
            let staged = render(&mut s, 100, 32);
            assert!(staged.contains(&format!("{} · 2", i18n::tr!(ChangesStaged))));
            assert!(!staged.contains(i18n::tr!(ChangesUnstaged)));
            support::git(
                &f.repo,
                &["reset", "HEAD", "--", "indexed.txt", "working.txt"],
            );
            refresh(&mut s, &f.repo);
            let unstaged = render(&mut s, 100, 32);
            assert!(unstaged.contains(&format!("{} · 2", i18n::tr!(ChangesUnstaged))));
            assert!(!unstaged.contains(i18n::tr!(ChangesStaged)));
            std::fs::remove_file(f.repo.join("indexed.txt")).unwrap();
            std::fs::remove_file(f.repo.join("working.txt")).unwrap();
            refresh(&mut s, &f.repo);
            assert!(render(&mut s, 100, 32).contains(i18n::tr!(FileListClean)));
            s.shutdown();
        }
    }
}
