use super::*;
use crate::{files::Layout, pointer::Hit};
use ratatui::layout::Position;

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
        // 同一路径的暂存与未暂存身份分别点击勾选，目录行不产生文件命中。
        let mut points = BTreeMap::new();
        for y in 0..40 {
            for x in 0..45 {
                if let Some(Hit::FileCheckbox(index)) = s.pointer.hit(Position::new(x, y)) {
                    points.entry(index).or_insert(Position::new(x, y));
                }
            }
        }
        assert_eq!(points.len(), 3);
        for index in [0, 2] {
            let point = points[&index];
            click(&mut s, point);
        }
        let files = s.mode.file_session().unwrap();
        let selected = files.selection();
        assert_eq!(selected.len(), 2);
        assert_ne!(selected[0].area, selected[1].area);
        let current = files.rows()[files.cursor].clone();
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
        assert_eq!(*files.rows()[files.cursor], current);
        command(&mut s, "repo diff view tree");
        render(&mut s, 160, 40);
        assert_eq!(s.mode.file_session().unwrap().selection(), selected);
        // 目录不能作为点击目标；文件名命中仍指向文件的可见索引。
        assert!(s.pointer.hit(Position::new(12, 3)).is_none());
        key(&mut s, KeyCode::Home);
        key(&mut s, KeyCode::Down);
        wait(&mut s);
        assert_eq!(
            s.mode.file_session().unwrap().rows()[1].location.name(),
            std::path::Path::new("src/中文 目录/b.ts")
        );
        assert_eq!(
            std::fs::read(f.repo.join(".git/index")).unwrap(),
            index_before
        );
        s.shutdown();
    }
}

fn click(s: &mut Screen, point: Position) {
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
fn tree_scrollbar_drag_uses_projected_rows_and_preserves_both_group_offsets() {
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
    let initial = s.mode.file_session().unwrap().cursor;
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
            assert!(files.offsets[0] > 30);
        } else {
            assert_eq!(files.offsets[0], 0);
        }
        assert_eq!(files.offsets[1], 0);
    }
    s.shutdown();
}
