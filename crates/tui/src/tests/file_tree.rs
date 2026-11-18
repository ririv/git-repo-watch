use super::*;
use crate::files::{CheckState, Cursor, Group};
use crate::{files::Layout, pointer::Hit};
use ratatui::layout::Position;

pub(super) fn directory_fixture() -> support::Fixture {
    let f = support::Fixture::create();
    std::fs::create_dir_all(f.repo.join("src/中文 目录")).unwrap();
    std::fs::create_dir_all(f.repo.join("outside")).unwrap();
    f.commit("src/中文 目录/a.ts", "old a\n");
    f.commit("src/中文 目录/b.ts", "old b\n");
    f.commit("outside/other.txt", "old outside\n");
    for file in ["a", "b"] {
        support::write(
            f.repo.join(format!("src/中文 目录/{file}.ts")),
            &format!("staged {file}\n"),
        );
    }
    support::git(&f.repo, &["add", "."]);
    for file in ["a", "b"] {
        support::write(
            f.repo.join(format!("src/中文 目录/{file}.ts")),
            &format!("working {file}\n"),
        );
    }
    support::write(f.repo.join("outside/other.txt"), "working outside\n");
    f
}

pub(super) fn directory_id(s: &Screen, group: Group) -> usize {
    let mut index = 0;
    for (section, entries) in s
        .mode
        .file_session()
        .unwrap()
        .entries(Layout::Tree)
        .iter()
        .enumerate()
    {
        for entry in entries {
            if entry.cursor(Group::from_index(section))
                == (Cursor::Directory {
                    group,
                    path: "src/中文 目录".into(),
                })
            {
                return index;
            }
            index += 1;
        }
    }
    panic!("directory row is visible")
}

pub(super) fn checkbox_point(s: &Screen, index: usize) -> Position {
    (0..40).find_map(|y| (0..45).find_map(|x| {
        let point = Position::new(x, y);
        matches!(s.pointer.hit(point), Some(Hit::FileCheckbox(candidate)) if candidate == index).then_some(point)
    })).expect("visible checkbox")
}

#[test]
fn directory_keyboard_and_pointer_selection_keeps_groups_separate_and_shows_partial_checks() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let f = directory_fixture();
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        command(&mut s, "repo diff");
        wait(&mut s);
        render(&mut s, 160, 40);
        let id = directory_id(&s, Group::Unstaged);
        let checkbox = checkbox_point(&s, id);
        click(&mut s, Position::new(checkbox.x + 5, checkbox.y));
        let files = s.mode.file_session().unwrap();
        assert!(files.current().is_none());
        assert!(files.selected.is_empty());
        assert_eq!(files.selection().len(), 2);
        assert!(
            files
                .selection()
                .iter()
                .all(|change| change.area == Area::Unstaged)
        );
        assert!(
            matches!(&s.mode, Mode::Changes(session) if matches!(session.preview, crate::changes::Preview::Directory { count: 2, .. }))
        );
        key(&mut s, KeyCode::Enter);
        key(&mut s, KeyCode::Insert);
        assert_eq!(s.mode.file_session().unwrap().selected.len(), 2);
        key(&mut s, KeyCode::Down);
        key(&mut s, KeyCode::Insert);
        key(&mut s, KeyCode::Up);
        let files = s.mode.file_session().unwrap();
        let entries = files.entries(Layout::Tree);
        let directory = entries[Group::Unstaged.index()]
            .iter()
            .find(|row| row.cursor(Group::Unstaged) == files.cursor)
            .unwrap();
        assert_eq!(
            files.check_state(directory, Group::Unstaged),
            CheckState::Mixed
        );
        assert!(render(&mut s, 160, 40).contains("[−]"));
        key(&mut s, KeyCode::Insert);
        let mut terminal = Terminal::new(TestBackend::new(160, 40)).unwrap();
        terminal.draw(|frame| view::render(frame, &mut s)).unwrap();
        let marks = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .filter(|cell| cell.symbol() == "✓")
            .collect::<Vec<_>>();
        assert_eq!(marks.len(), 3);
        assert!(
            marks
                .iter()
                .all(|cell| cell.fg == ratatui::style::Color::Rgb(73, 199, 140)
                    && cell.modifier.contains(ratatui::style::Modifier::BOLD))
        );
        let snapshot = s.service.files(&f.repo, &Cancel::default()).unwrap();
        let current = s.mode.file_session().unwrap();
        let refreshed = current.refreshed(snapshot);
        assert_eq!(refreshed.cursor, current.cursor);
        assert_eq!(refreshed.selection(), current.selection());
        key(&mut s, KeyCode::Insert);
        render(&mut s, 160, 40);
        click(&mut s, checkbox);
        assert_eq!(s.mode.file_session().unwrap().selected.len(), 2);
        click(&mut s, checkbox);
        assert!(s.mode.file_session().unwrap().selected.is_empty());
        render(&mut s, 160, 40);
        command(&mut s, "repo diff staged");
        wait(&mut s);
        render(&mut s, 160, 40);
        let staged = checkbox_point(&s, directory_id(&s, Group::Staged));
        click(&mut s, staged);
        assert!(
            s.mode
                .file_session()
                .unwrap()
                .selection()
                .iter()
                .all(|change| change.area == Area::Staged)
        );
        command(&mut s, "repo diff unstage");
        wait(&mut s);
        let snapshot = s.service.files(&f.repo, &Cancel::default()).unwrap();
        assert!(
            snapshot
                .changes
                .iter()
                .all(|change| change.area != Area::Staged)
        );
        assert_eq!(
            std::fs::read_to_string(f.repo.join("outside/other.txt")).unwrap(),
            "working outside\n"
        );
        s.shutdown();
    }
}

#[test]
fn filtered_directory_operations_only_stage_visible_files_and_flat_layout_preserves_checks() {
    let f = directory_fixture();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo diff");
    wait(&mut s);
    s.paste("b.ts");
    render(&mut s, 160, 40);
    let point = checkbox_point(&s, directory_id(&s, Group::Unstaged));
    click(&mut s, Position::new(point.x + 5, point.y));
    let files = s.mode.file_session().unwrap();
    assert!(files.selected.is_empty());
    assert_eq!(files.selection().len(), 1);
    assert_eq!(
        files.selection()[0].location.name(),
        std::path::Path::new("src/中文 目录/b.ts")
    );
    key(&mut s, KeyCode::Insert);
    let selected = s.mode.file_session().unwrap().selection();
    command(&mut s, "repo diff view flat");
    assert!(s.mode.file_session().unwrap().current().is_some());
    assert_eq!(s.mode.file_session().unwrap().selection(), selected);
    command(&mut s, "repo diff view tree");
    render(&mut s, 160, 40);
    let point = checkbox_point(&s, directory_id(&s, Group::Unstaged));
    click(&mut s, point);
    assert!(s.mode.file_session().unwrap().selected.is_empty());
    command(&mut s, "repo diff stage");
    wait(&mut s);
    let snapshot = s.service.files(&f.repo, &Cancel::default()).unwrap();
    assert!(
        snapshot
            .changes
            .iter()
            .any(|change| change.area == Area::Unstaged
                && change.location.name() == std::path::Path::new("src/中文 目录/a.ts"))
    );
    assert!(
        !snapshot
            .changes
            .iter()
            .any(|change| change.area == Area::Unstaged
                && change.location.name() == std::path::Path::new("src/中文 目录/b.ts"))
    );
    assert!(
        snapshot
            .changes
            .iter()
            .any(|change| change.area == Area::Unstaged
                && change.location.name() == std::path::Path::new("outside/other.txt"))
    );
    s.shutdown();
}

#[test]
fn file_modes_preserve_selection_preview_and_group_identity_in_both_languages() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let f = support::Fixture::create();
        let path = "src/中文 目录/a.ts";
        std::fs::create_dir_all(f.repo.join("src/中文 目录")).unwrap();
        support::write(f.repo.join(path), "export const a = 1;\n");
        support::git(&f.repo, &["add", "."]);
        support::write(f.repo.join(path), "export const a = 2;\n");
        support::write(f.repo.join("src/中文 目录/b.ts"), "export const b = 3;\n");
        let index_before = std::fs::read(f.repo.join(".git/index")).unwrap();
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        command(&mut s, "repo diff");
        wait(&mut s);
        assert_eq!(s.file_layout, Layout::Tree);
        let text = render(&mut s, 160, 40);
        assert!(text.contains("src/") && text.contains("中文 目录/"));
        // 目录与文件各有命中行；同一路径在暂存与未暂存区的文件身份仍然独立。
        let entries = s
            .mode
            .file_session()
            .unwrap()
            .entries(Layout::Tree)
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        let mut points = BTreeMap::new();
        for y in 0..40 {
            for x in 0..45 {
                if let Some(Hit::FileCheckbox(index)) = s.pointer.hit(Position::new(x, y))
                    && let crate::files::tree::Row::File { index, .. } = &entries[index]
                {
                    points.entry(*index).or_insert(Position::new(x, y));
                }
            }
        }
        assert_eq!(points.len(), 2);
        let point = points[&0];
        click(&mut s, point);
        let files = s.mode.file_session().unwrap();
        let selected = files.selection();
        assert_eq!(selected.len(), 1);
        assert_ne!(selected[0].area, Area::Staged);
        let current = files.current().unwrap().clone();
        command(&mut s, "repo diff view");
        let commands = s.search_commands().unwrap();
        assert_eq!(commands.len(), 1);
        assert!(
            matches!(&s.bar, bar::InputBar::Command { menu: command::completion::Menu::Options(values), .. } if values.options.len() == 2)
        );
        key(&mut s, KeyCode::Esc);
        command(&mut s, "repo diff view flat");
        assert_eq!(s.file_layout, Layout::Flat);
        assert!(render(&mut s, 160, 40).contains("src/中文 目录/a.ts"));
        let files = s.mode.file_session().unwrap();
        assert_eq!(files.selection(), selected);
        assert_eq!(*files.current().unwrap(), current);
        command(&mut s, "repo diff view tree");
        render(&mut s, 160, 40);
        assert_eq!(s.mode.file_session().unwrap().selection(), selected);
        let area = s.body_area.unwrap();
        assert!((area.y..area.bottom()).any(|row| matches!(
            s.pointer.hit(Position::new(area.x + 12, row)),
            Some(Hit::Item(crate::pointer::Target::Files, 0))
        )));
        command(&mut s, "repo diff staged");
        wait(&mut s);
        key(&mut s, KeyCode::Home);
        assert!(matches!(
            s.mode.file_session().unwrap().cursor,
            crate::files::Cursor::Directory { .. }
        ));
        key(&mut s, KeyCode::Down);
        wait(&mut s);
        assert_eq!(
            s.mode
                .file_session()
                .unwrap()
                .current()
                .unwrap()
                .location
                .name(),
            std::path::Path::new("src/中文 目录/a.ts")
        );
        assert_eq!(
            std::fs::read(f.repo.join(".git/index")).unwrap(),
            index_before
        );
        s.shutdown();
    }
}

pub(super) fn click(s: &mut Screen, point: Position) {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    s.mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: point.x,
            row: point.y,
            modifiers: KeyModifiers::NONE,
        },
        &Cancel::default(),
    )
    .unwrap();
}

#[test]
fn tree_scrollbar_drag_uses_one_viewport_for_projected_rows() {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    let f = support::Fixture::create();
    for index in 0..25 {
        let dir = f.repo.join(format!("folder-{index:02}"));
        std::fs::create_dir_all(&dir).unwrap();
        support::write(dir.join("a.txt"), "staged\n");
    }
    support::git(&f.repo, &["add", "."]);
    for index in 0..25 {
        support::write(f.repo.join(format!("folder-{index:02}/a.txt")), "working\n");
    }
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo diff");
    wait(&mut s);
    render(&mut s, 160, 32);
    let thumb = (0..32)
        .find_map(|y| {
            (0..42).find_map(|x| {
                let point = Position::new(x, y);
                s.pointer.grab(point).map(|_| point)
            })
        })
        .expect("tree scrollbar thumb");
    let initial = s.mode.file_session().unwrap().cursor.clone();
    click(&mut s, thumb);
    assert_eq!(s.mode.file_session().unwrap().cursor, initial);
    for (row, expect_bottom) in [(31, true), (thumb.y, false)] {
        s.mouse(
            MouseEvent {
                kind: MouseEventKind::Drag(MouseButton::Left),
                column: 90,
                row,
                modifiers: KeyModifiers::NONE,
            },
            &Cancel::default(),
        )
        .unwrap();
        render(&mut s, 160, 32);
        let files = s.mode.file_session().unwrap();
        if expect_bottom {
            assert!(files.offset > 0);
        } else {
            assert_eq!(files.offset, 0);
        }
    }
    s.shutdown();
}
