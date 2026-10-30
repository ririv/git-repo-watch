use super::browser::{ready, select, session};
use super::*;
use crate::browser::{Focus, location::Location};
use crate::pointer::{Action as PointerAction, Hit};
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Position;

fn point(s: &Screen, depth: usize, width: u16, height: u16) -> Position {
    (0..height)
        .find_map(|y| {
            (0..width).find_map(|x| {
                let point = Position::new(x, y);
                (s.pointer.hit(point) == Some(Hit::Button(PointerAction::BrowserDirectory(depth))))
                    .then_some(point)
            })
        })
        .expect("visible directory breadcrumb")
}
fn click(s: &mut Screen, point: Position) {
    for kind in [
        MouseEventKind::Down(MouseButton::Left),
        MouseEventKind::Up(MouseButton::Left),
    ] {
        s.mouse(
            MouseEvent {
                kind,
                column: point.x,
                row: point.y,
                modifiers: KeyModifiers::NONE,
            },
            &Cancel::default(),
        )
        .unwrap();
    }
}

#[test]
fn file_names_and_clickable_ancestors_return_search_results_to_the_tree_without_losing_content_drafts()
 {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let f = support::Fixture::create();
        for directory in ["src/中文 空格/deep", "other"] {
            std::fs::create_dir_all(f.repo.join(directory)).unwrap();
        }
        f.commit("src/中文 空格/deep/code.rs", "let needle = 1;\n");
        f.commit("src/info.txt", "info\n");
        f.commit("other/kept.txt", "kept\n");
        let original_index = std::fs::read(f.repo.join(".git/index")).unwrap();
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        command(&mut s, "repo files");
        ready(&mut s);
        select(&mut s, "other");
        key(&mut s, KeyCode::Enter);
        ready(&mut s);
        s.paste("code.rs");
        ready(&mut s);
        key(&mut s, KeyCode::Enter);
        s.paste("needle");
        let rendered = render(&mut s, 160, 30);
        let body = s.body_area.unwrap();
        let preview = view::browser::areas(body, session(&mut s))
            .into_iter()
            .find(|(_, focus)| *focus == Focus::Preview)
            .unwrap()
            .0;
        let title = rendered.lines().nth(preview.y as usize).unwrap();
        assert!(!title.contains("code.rs"));
        assert!(!title.contains("src/"));
        assert!(
            rendered
                .lines()
                .nth(body.y as usize)
                .unwrap()
                .contains("src › 中文 空格 › deep › code.rs")
        );
        let parent = point(&s, 2, 160, 30);
        click(&mut s, parent);
        ready(&mut s);
        assert_eq!(session(&mut s).focus, Focus::Tree);
        assert_eq!(
            session(&mut s).selected().unwrap().path,
            PathBuf::from("src/中文 空格")
        );
        assert!(session(&mut s).expanded(std::path::Path::new("other")));
        assert!(s.bar.text().is_empty());
        select(&mut s, "src/中文 空格/deep");
        select(&mut s, "src/中文 空格/deep/code.rs");
        key(&mut s, KeyCode::Enter);
        assert_eq!(s.bar.text(), "needle");
        assert!(s.search_title().contains("1/1"));
        assert_eq!(
            std::fs::read(f.repo.join(".git/index")).unwrap(),
            original_index
        );
        assert_eq!(
            std::fs::read_to_string(f.repo.join("src/中文 空格/deep/code.rs")).unwrap(),
            "let needle = 1;\n"
        );
        let next = session(&mut s)
            .rows()
            .iter()
            .position(|row| row.entry.path == std::path::Path::new("other/kept.txt"))
            .unwrap();
        session(&mut s).select(next);
        let retained = render(&mut s, 160, 30);
        assert!(
            !retained
                .lines()
                .nth(preview.y as usize)
                .unwrap()
                .contains("code.rs")
        );
        assert!(
            retained
                .lines()
                .nth(body.y as usize)
                .unwrap()
                .contains("src › 中文 空格 › deep › code.rs")
        );
        let parent = point(&s, 2, 160, 30);
        click(&mut s, parent);
        ready(&mut s);
        assert_eq!(
            session(&mut s).selected().unwrap().path,
            PathBuf::from("src/中文 空格")
        );
        s.shutdown();
    }
}

#[test]
fn compact_chain_ancestors_and_repository_root_have_distinct_navigation_targets() {
    let f = support::Fixture::create();
    std::fs::create_dir_all(f.repo.join("a/b/c")).unwrap();
    f.commit("a/b/c/code.txt", "contents\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo files");
    ready(&mut s);
    s.paste("code.txt");
    ready(&mut s);
    key(&mut s, KeyCode::Enter);
    render(&mut s, 120, 30);
    let parent = point(&s, 1, 120, 30);
    click(&mut s, parent);
    ready(&mut s);
    assert_eq!(session(&mut s).selected().unwrap().path, PathBuf::from("a"));
    assert!(
        matches!(session(&mut s).location(), Location::Directory(path) if path == std::path::Path::new("a"))
    );
    render(&mut s, 120, 30);
    let root = point(&s, 0, 120, 30);
    click(&mut s, root);
    ready(&mut s);
    assert!(matches!(session(&mut s).location(), Location::Root));
    assert!(session(&mut s).state.selected().is_none());
    key(&mut s, KeyCode::Down);
    assert_eq!(session(&mut s).state.selected(), Some(0));
    assert_eq!(
        session(&mut s).selected().unwrap().path,
        PathBuf::from("a/b/c")
    );
    render(&mut s, 120, 30);
    let home = s
        .breadcrumb_items
        .iter()
        .find(|(_, destination)| *destination == crate::navigation::breadcrumbs::Destination::Home)
        .unwrap()
        .0;
    click(&mut s, Position::new(home.x, home.y));
    assert_eq!(s.page, Page::Repositories);
    s.shutdown();
}

#[test]
fn path_hit_regions_follow_unicode_clipping_pointer_preferences_and_small_windows() {
    let f = support::Fixture::create();
    std::fs::create_dir_all(f.repo.join("很长的目录 名称/deep/another")).unwrap();
    f.commit("很长的目录 名称/deep/another/中文 文件.md", "# Heading\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo files");
    ready(&mut s);
    s.paste("中文 文件.md");
    ready(&mut s);
    key(&mut s, KeyCode::Enter);
    let filename = session(&mut s).selected().unwrap().path;
    let mut body_height = None;
    for (width, height) in [(160, 30), (55, 18), (24, 14), (1, 1), (160, 30)] {
        let rendered = render(&mut s, width, height);
        assert_eq!(session(&mut s).selected().unwrap().path, filename);
        if width == 160 {
            assert!(rendered.contains("中文 文件.md"));
            let area = view::browser::areas(s.body_area.unwrap(), session(&mut s))[1].0;
            assert_eq!(*body_height.get_or_insert(area.height), area.height);
        }
        for y in 0..height {
            for x in 0..width {
                if let Some(Hit::Button(PointerAction::BrowserDirectory(depth))) =
                    s.pointer.hit(Position::new(x, y))
                {
                    assert!(depth < 4);
                    assert_eq!(y, s.body_area.unwrap().y);
                }
            }
        }
    }
    let root = point(&s, 0, 160, 30);
    s.config.mouse.precise_click = false;
    click(&mut s, root);
    assert_eq!(session(&mut s).focus, Focus::Preview);
    assert_eq!(session(&mut s).selected().unwrap().path, filename);
    s.config.mouse.precise_click = true;
    s.config.mouse.select_pane = false;
    click(&mut s, root);
    ready(&mut s);
    assert!(matches!(session(&mut s).location(), Location::Root));
    assert_eq!(session(&mut s).focus, Focus::Tree);
    s.shutdown();
}

#[test]
fn removed_ancestor_reports_the_read_error_without_selecting_an_unrelated_file() {
    let f = support::Fixture::create();
    std::fs::create_dir_all(f.repo.join("a/b")).unwrap();
    std::fs::write(f.repo.join("a/b/code.txt"), "contents\n").unwrap();
    f.commit("a/info.txt", "info\n");
    f.commit("other.txt", "other\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo files");
    ready(&mut s);
    s.paste("code.txt");
    ready(&mut s);
    key(&mut s, KeyCode::Enter);
    render(&mut s, 140, 28);
    let target = point(&s, 2, 140, 28);
    std::fs::remove_dir_all(f.repo.join("a")).unwrap();
    click(&mut s, target);
    ready(&mut s);
    assert_eq!(session(&mut s).focus, Focus::Tree);
    assert!(session(&mut s).state.selected().is_none());
    assert!(session(&mut s).notice().is_some());
    assert!(
        matches!(session(&mut s).location(), Location::Directory(path) if path == std::path::Path::new("a/b"))
    );
    s.shutdown();
}
