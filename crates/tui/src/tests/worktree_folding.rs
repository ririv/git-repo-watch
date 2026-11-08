use super::file_tree::{checkbox_point, click, directory_fixture, directory_id};
use super::*;
use crate::files::{Cursor, Group, Layout};
use ratatui::layout::Position;

fn files_mut(s: &mut Screen) -> &mut FileSession {
    match &mut s.mode {
        Mode::Files(files) => files,
        Mode::Changes(session) => &mut session.files,
        _ => panic!("file panel"),
    }
}
fn expansion(files: &FileSession, group: Group) -> crate::files::Expansion {
    files.entries(Layout::Tree)[group.index()]
        .iter()
        .find_map(|row| match row {
            crate::files::tree::Row::Directory {
                path, expansion, ..
            } if path == std::path::Path::new("src/中文 目录") => Some(*expansion),
            _ => None,
        })
        .expect("directory row")
}

#[test]
fn worktree_folding_keyboard_preserves_hidden_checks_and_group_navigation() {
    use crate::files::Expansion::{Collapsed, Expanded};
    let f = directory_fixture();
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    for combined in [false, true] {
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        if combined {
            command(&mut s, "repo diff");
            wait(&mut s);
        } else {
            open_files(&mut s);
        }
        let id = directory_id(&s, Group::Unstaged);
        assert!(files_mut(&mut s).select_row(id, Layout::Tree));
        key(&mut s, KeyCode::Insert);
        let selected = s.mode.file_session().unwrap().selection();
        let counts = s
            .mode
            .file_session()
            .unwrap()
            .entries(Layout::Tree)
            .map(|rows| rows.len());
        key(&mut s, KeyCode::Enter);
        let files = s.mode.file_session().unwrap();
        assert_eq!(expansion(files, Group::Unstaged), Collapsed);
        assert_eq!(expansion(files, Group::Staged), Expanded);
        assert_eq!(
            files.entries(Layout::Tree).map(|rows| rows.len()),
            [counts[0], counts[1] - 2]
        );
        assert_eq!(files.focused_changes().len(), 2);
        assert_eq!(files.selection(), selected);
        let steps = files.entries(Layout::Tree)[Group::Unstaged.index()]
            .iter()
            .position(|row| row.cursor(Group::Unstaged) == files.cursor)
            .unwrap()
            + 1;
        for _ in 0..steps {
            key(&mut s, KeyCode::Up);
        }
        assert_eq!(s.mode.file_session().unwrap().group(), Some(Group::Staged));
        for _ in 0..steps {
            key(&mut s, KeyCode::Down);
        }
        assert_eq!(
            s.mode.file_session().unwrap().cursor,
            Cursor::Directory {
                group: Group::Unstaged,
                path: "src/中文 目录".into()
            }
        );
        key(&mut s, KeyCode::Enter);
        key(&mut s, KeyCode::Down);
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
        assert_eq!(s.mode.file_session().unwrap().selection(), selected);
        s.shutdown();
    }
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    assert_eq!(
        std::fs::read_to_string(f.repo.join("src/中文 目录/a.ts")).unwrap(),
        "working a\n"
    );
}

#[test]
fn worktree_directory_click_folds_once_per_pair_and_checkbox_only_marks() {
    use crate::files::Expansion::{Collapsed, Expanded};
    let f = directory_fixture();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo diff");
    wait(&mut s);
    render(&mut s, 160, 40);
    let point = checkbox_point(&s, directory_id(&s, Group::Unstaged));
    let name = Position::new(point.x + 5, point.y);
    for state in [Collapsed, Collapsed, Expanded, Expanded] {
        click(&mut s, name);
        assert_eq!(
            expansion(s.mode.file_session().unwrap(), Group::Unstaged),
            state
        );
        assert!(s.mode.file_session().unwrap().selected.is_empty());
        render(&mut s, 160, 40);
    }
    for count in [2, 0] {
        click(&mut s, point);
        assert_eq!(s.mode.file_session().unwrap().selected.len(), count);
        assert_eq!(
            expansion(s.mode.file_session().unwrap(), Group::Unstaged),
            Expanded
        );
        render(&mut s, 160, 40);
    }
    s.config.file_icons = false;
    assert!(render(&mut s, 160, 40).contains("▼ src/中文 目录/"));
    key(&mut s, KeyCode::Enter);
    assert!(render(&mut s, 160, 40).contains("▶ src/中文 目录/"));
    assert!(
        matches!(&s.mode, Mode::Changes(session) if session.focus == crate::changes::Focus::Files)
    );
    s.shutdown();
}

#[test]
fn worktree_search_reveals_matches_and_restores_browsing_folds() {
    use crate::files::Expansion::{Collapsed, Expanded};
    let f = directory_fixture();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo diff");
    wait(&mut s);
    let id = directory_id(&s, Group::Unstaged);
    files_mut(&mut s).select_row(id, Layout::Tree);
    key(&mut s, KeyCode::Enter);
    s.paste("a.ts");
    assert_eq!(
        expansion(s.mode.file_session().unwrap(), Group::Unstaged),
        Expanded
    );
    assert_eq!(s.mode.file_session().unwrap().rows().len(), 2);
    key(&mut s, KeyCode::Esc);
    render(&mut s, 160, 40);
    assert_eq!(
        expansion(s.mode.file_session().unwrap(), Group::Unstaged),
        Collapsed
    );
    files_mut(&mut s).set_search_mode(crate::search::Mode::Locate);
    s.paste(".ts");
    let id = directory_id(&s, Group::Unstaged);
    files_mut(&mut s).select_row(id, Layout::Tree);
    key(&mut s, KeyCode::Enter);
    assert_eq!(
        expansion(s.mode.file_session().unwrap(), Group::Unstaged),
        Collapsed
    );
    key(&mut s, KeyCode::Down);
    assert_eq!(
        expansion(s.mode.file_session().unwrap(), Group::Unstaged),
        Expanded
    );
    assert!(s.mode.file_session().unwrap().current().is_some());
    key(&mut s, KeyCode::Esc);
    render(&mut s, 160, 40);
    assert_eq!(
        expansion(s.mode.file_session().unwrap(), Group::Unstaged),
        Collapsed
    );
    command(&mut s, "repo diff view flat");
    assert!(
        s.mode
            .file_session()
            .unwrap()
            .entries(Layout::Flat)
            .iter()
            .flatten()
            .all(|row| matches!(row, crate::files::tree::Row::File { .. }))
    );
    command(&mut s, "repo diff view tree");
    assert_eq!(
        expansion(s.mode.file_session().unwrap(), Group::Unstaged),
        Collapsed
    );
    assert!(matches!(
        s.mode.file_session().unwrap().cursor,
        Cursor::Directory { .. }
    ));
    s.shutdown();
}

#[test]
fn worktree_refresh_keeps_folds_across_compact_chain_changes_and_removed_directories() {
    use crate::files::Expansion::{Collapsed, Expanded};
    let f = support::Fixture::create();
    std::fs::create_dir_all(f.repo.join("src/中文 目录")).unwrap();
    support::write(f.repo.join("src/中文 目录/a.ts"), "a\n");
    support::write(f.repo.join("src/other.txt"), "other\n");
    let (_home, s) = fixture(vec![f.repo.clone()]);
    let snapshot = s.service.files(&f.repo, &Cancel::default()).unwrap();
    let mut files = FileSession::new(snapshot);
    let index = files.entries(Layout::Tree)[Group::Unstaged.index()]
        .iter()
        .position(|row| {
            row.cursor(Group::Unstaged)
                == Cursor::Directory {
                    group: Group::Unstaged,
                    path: "src/中文 目录".into(),
                }
        })
        .unwrap();
    assert!(files.select_row(index, Layout::Tree));
    assert!(files.toggle_directory(Layout::Tree));
    assert_eq!(expansion(&files, Group::Unstaged), Collapsed);
    files.toggle_current();
    std::fs::remove_file(f.repo.join("src/other.txt")).unwrap();
    let mut next = files.refreshed(s.service.files(&f.repo, &Cancel::default()).unwrap());
    assert_eq!(next.cursor, files.cursor);
    assert_eq!(next.selection(), files.selection());
    assert_eq!(next.entries(Layout::Tree)[Group::Unstaged.index()].len(), 1);
    assert_eq!(expansion(&next, Group::Unstaged), Collapsed);
    assert!(next.toggle_directory(Layout::Tree));
    assert_eq!(expansion(&next, Group::Unstaged), Expanded);
    support::write(f.repo.join("src/new.txt"), "new\n");
    next = next.refreshed(s.service.files(&f.repo, &Cancel::default()).unwrap());
    assert_eq!(
        next.cursor,
        Cursor::Directory {
            group: Group::Unstaged,
            path: "src".into()
        }
    );
    std::fs::remove_dir_all(f.repo.join("src")).unwrap();
    next = next.refreshed(s.service.files(&f.repo, &Cancel::default()).unwrap());
    assert_eq!(next.cursor, Cursor::Empty);
    assert!(next.selected.is_empty());
}
