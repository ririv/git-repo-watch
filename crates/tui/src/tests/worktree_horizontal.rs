use super::*;
use crate::files::Layout;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::{Position, Rect};

const BEFORE: &str = "old/中文 目录/long-original-package/old-assistant-with-a-long-filename.ts";
const AFTER: &str = "new/迁移 目录/new-assistant-with-a-long-filename.ts";

fn renamed() -> support::Fixture {
    let f = support::Fixture::create();
    std::fs::create_dir_all(f.repo.join(std::path::Path::new(BEFORE).parent().unwrap())).unwrap();
    f.commit(BEFORE, "export const value = 1;\n");
    std::fs::create_dir_all(f.repo.join("new/迁移 目录")).unwrap();
    support::git(&f.repo, &["mv", BEFORE, AFTER]);
    f
}
fn panel(s: &Screen) -> Rect {
    if s.page == Page::Detail {
        s.workspace_areas
            .iter()
            .find(|(_, section)| *section == crate::workspace::Section::Worktree)
            .unwrap()
            .0
    } else if let Mode::Changes(session) = &s.mode {
        crate::view::changes::areas(s.body_area.unwrap(), session)
            .into_iter()
            .find(|(_, focus)| *focus == crate::changes::Focus::Files)
            .unwrap()
            .0
    } else {
        s.body_area.unwrap()
    }
}
fn has_track(s: &mut Screen, width: u16, height: u16) -> bool {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| view::render(frame, s)).unwrap();
    let area = panel(s);
    (area.x + 1..area.right() - 1).any(|x| {
        terminal.backend().buffer()[(x, area.bottom() - 1)].fg == crate::scrollbar::THUMB_COLOR
    })
}

#[test]
fn worktree_paths_scroll_in_both_layouts_and_repository_panes_without_moving_selection() {
    let f = renamed();
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    for workspace in [false, true] {
        for layout in [Layout::Flat, Layout::Tree] {
            let (_home, mut s) = fixture(vec![f.repo.clone()]);
            s.file_layout = layout;
            if workspace {
                key(&mut s, KeyCode::Enter);
                wait_workspace(&mut s);
                s.switch_repository_section(crate::workspace::Section::Worktree);
            } else {
                open_files(&mut s);
            }
            assert!(has_track(&mut s, 120, 32));
            let area = panel(&s);
            let y = area.bottom() - 1;
            let before = s.mode.file_session().unwrap().cursor.clone();
            key(&mut s, KeyCode::Right);
            assert!(matches!(s.mode, Mode::Files(_)));
            assert_eq!(s.mode.file_session().unwrap().cursor, before);
            assert_eq!(s.mode.file_session().unwrap().horizontal.offset(), 4);
            key(&mut s, KeyCode::Insert);
            assert!(has_track(&mut s, 120, 32));
            for _ in 0..80 {
                key(&mut s, KeyCode::Right);
            }
            let mut terminal = Terminal::new(TestBackend::new(120, 32)).unwrap();
            terminal.draw(|frame| view::render(frame, &mut s)).unwrap();
            let b = terminal.backend().buffer();
            let row = (area.y + 1..y)
                .find(|row| {
                    b[(area.x + 2, *row)].symbol() == "✓"
                        && b[(area.right() - 2, *row)].symbol() == "R"
                })
                .unwrap();
            assert_eq!(b[(area.x + 1, row)].symbol(), "[");
            assert_eq!(b[(area.x + 3, row)].symbol(), "]");
            assert_eq!(b[(area.right() - 2, row)].symbol(), "R");
            let text = (area.x + 1..area.right() - 1)
                .map(|x| b[(x, row)].symbol())
                .collect::<String>();
            assert!(
                text.contains("new-assistant-with-a-long-filename.ts"),
                "{text}"
            );
            let end = s.mode.file_session().unwrap().horizontal.offset();
            assert!(end > 4);
            key(&mut s, KeyCode::Left);
            assert_eq!(s.mode.file_session().unwrap().horizontal.offset(), end - 4);
            assert!(!has_track(&mut s, 360, 32));
            assert_eq!(s.mode.file_session().unwrap().horizontal.offset(), 0);
            assert!(has_track(&mut s, 120, 32));
            let area = panel(&s);
            let point = Position::new(area.right() - 4, area.bottom() - 1);
            assert!(matches!(
                s.pointer.hit(point),
                Some(crate::pointer::Hit::Scrollbar(
                    crate::pointer::Action::Scroll(
                        crate::pointer::ScrollTarget::Horizontal(crate::horizontal::Pane::Worktree),
                        _
                    )
                ))
            ));
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
            s.mouse(
                MouseEvent {
                    kind: MouseEventKind::Up(MouseButton::Left),
                    column: point.x,
                    row: point.y,
                    modifiers: KeyModifiers::NONE,
                },
                &Cancel::default(),
            )
            .unwrap();
            assert!(s.mode.file_session().unwrap().horizontal.offset() > 0);
            assert_eq!(s.mode.file_session().unwrap().cursor, before);
            key(&mut s, KeyCode::Enter);
            wait(&mut s);
            assert!(matches!(s.mode, Mode::Changes(_)));
        }
    }
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
}

#[test]
fn change_file_list_scrolls_independently_and_preserves_its_position_after_refresh() {
    let f = renamed();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    s.file_layout = Layout::Flat;
    command(&mut s, "repo diff staged");
    wait(&mut s);
    assert!(has_track(&mut s, 120, 30));
    key(&mut s, KeyCode::Right);
    assert!(
        matches!(&s.mode,Mode::Changes(session) if session.focus==crate::changes::Focus::Files)
    );
    let offset = s.mode.file_session().unwrap().horizontal.offset();
    assert_eq!(offset, 4);
    command(&mut s, "refresh");
    wait(&mut s);
    render(&mut s, 120, 30);
    assert_eq!(s.mode.file_session().unwrap().horizontal.offset(), offset);
    s.paste("not-present");
    assert!(!has_track(&mut s, 120, 30));
    assert_eq!(s.mode.file_session().unwrap().horizontal.offset(), 0);
    key(&mut s, KeyCode::Esc);
    assert!(has_track(&mut s, 120, 30));
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert!(
        matches!(&s.mode,Mode::Changes(session) if session.focus==crate::changes::Focus::Preview)
    );
    key(&mut s, KeyCode::Esc);
    assert!(
        matches!(&s.mode,Mode::Changes(session) if session.focus==crate::changes::Focus::Files)
    );
}
