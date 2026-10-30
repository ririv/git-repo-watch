use super::browser::{ready, select, session};
use super::*;
use crate::{
    browser::Focus,
    pointer::{Action as PointerAction, Hit, ScrollTarget},
};
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect},
};

fn frame(s: &mut Screen, width: u16, height: u16) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| view::render(frame, s)).unwrap();
    terminal.backend().buffer().clone()
}
fn tree(s: &mut Screen) -> Rect {
    view::browser::areas(s.body_area.unwrap(), session(s))
        .into_iter()
        .find(|(_, focus)| *focus == Focus::Tree)
        .unwrap()
        .0
}
fn event(s: &mut Screen, kind: MouseEventKind, point: Position, modifiers: KeyModifiers) -> bool {
    s.mouse(
        MouseEvent {
            kind,
            column: point.x,
            row: point.y,
            modifiers,
        },
        &Cancel::default(),
    )
    .unwrap()
}
fn track(s: &Screen, area: Rect) -> Vec<Position> {
    (area.x..area.right())
        .filter_map(|x| {
            let point = Position::new(x, area.bottom() - 1);
            matches!(
                s.pointer.hit(point),
                Some(Hit::Scrollbar(PointerAction::Scroll(
                    ScrollTarget::BrowserTreeHorizontal,
                    _
                )))
            )
            .then_some(point)
        })
        .collect()
}
fn click(s: &mut Screen, point: Position) {
    event(
        s,
        MouseEventKind::Down(MouseButton::Left),
        point,
        KeyModifiers::NONE,
    );
    event(
        s,
        MouseEventKind::Up(MouseButton::Left),
        point,
        KeyModifiers::NONE,
    );
}

#[test]
fn long_unicode_names_scroll_with_pinned_status_and_draggable_tracks_then_clamp_when_widened() {
    let f = support::Fixture::create();
    let name = format!("a_{}TAIL.txt", "中文e\u{301}👩‍💻 ".repeat(6));
    f.commit(&name, "old\n");
    std::fs::write(f.repo.join(&name), "new\n").unwrap();
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo files");
    ready(&mut s);
    select(&mut s, &name);
    let before = frame(&mut s, 120, 28);
    let area = tree(&mut s);
    let row = area.y + 1;
    assert!(session(&mut s).horizontal.overflowing());
    key(&mut s, KeyCode::Right);
    assert_eq!(session(&mut s).horizontal.offset(), 4);
    assert_eq!(
        session(&mut s).selected().unwrap().path,
        PathBuf::from(&name)
    );
    key(&mut s, KeyCode::Left);
    assert_eq!(session(&mut s).horizontal.offset(), 0);
    key(&mut s, KeyCode::Tab);
    assert_eq!(session(&mut s).focus, Focus::Preview);
    key(&mut s, KeyCode::Right);
    assert_eq!(session(&mut s).pane().position.horizontal.offset(), 0);
    key(&mut s, KeyCode::BackTab);
    assert_eq!(session(&mut s).focus, Focus::Tree);
    let end = *track(&s, area).last().unwrap();
    assert_eq!(end.x, area.right() - 5);
    assert_eq!(before[(area.right() - 2, row)].symbol(), "M");
    assert_eq!(
        before[(area.right() - 2, row)].fg,
        ratatui::style::Color::Yellow
    );
    click(&mut s, end);
    let after = frame(&mut s, 120, 28);
    let body = (area.x + 1..area.right() - 1)
        .map(|x| after[(x, row)].symbol())
        .collect::<String>();
    assert!(body.contains("TAIL.txt"), "{body}");
    assert!(session(&mut s).horizontal.offset() > 0);
    assert_eq!(after[(area.right() - 2, row)].symbol(), "M");
    assert_eq!(
        after[(area.right() - 2, row)].fg,
        ratatui::style::Color::Yellow
    );
    event(
        &mut s,
        MouseEventKind::Down(MouseButton::Left),
        end,
        KeyModifiers::NONE,
    );
    assert!(s.drag.is_some());
    event(
        &mut s,
        MouseEventKind::Drag(MouseButton::Left),
        Position::new(0, end.y),
        KeyModifiers::NONE,
    );
    event(
        &mut s,
        MouseEventKind::Up(MouseButton::Left),
        Position::new(0, end.y),
        KeyModifiers::NONE,
    );
    assert_eq!(session(&mut s).horizontal.offset(), 0);
    frame(&mut s, 120, 28);
    click(&mut s, end);
    assert!(session(&mut s).horizontal.offset() > 0);
    session(&mut s).arrangement = crate::workspace::Arrangement::Maximized;
    frame(&mut s, 500, 28);
    assert_eq!(session(&mut s).horizontal.offset(), 0);
    assert!(!session(&mut s).horizontal.overflowing());
    let wide = tree(&mut s);
    assert!(track(&s, wide).is_empty());
    assert_eq!(
        session(&mut s).selected().unwrap().path,
        PathBuf::from(&name)
    );
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    assert_eq!(std::fs::read_to_string(f.repo.join(name)).unwrap(), "new\n");
    s.shutdown();
}

#[test]
fn exact_fit_and_collapsed_directories_do_not_create_phantom_overflow() {
    let f = support::Fixture::create();
    std::fs::create_dir(f.repo.join("pack")).unwrap();
    f.commit(&format!("pack/{}.txt", "long".repeat(16)), "contents\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo files");
    ready(&mut s);
    session(&mut s).arrangement = crate::workspace::Arrangement::Maximized;
    frame(&mut s, 14, 22);
    let area = tree(&mut s);
    assert!(!session(&mut s).horizontal.overflowing());
    assert!(track(&s, area).is_empty());
    frame(&mut s, 13, 22);
    assert!(session(&mut s).horizontal.overflowing());
    let area = tree(&mut s);
    let end = *track(&s, area).last().unwrap();
    click(&mut s, end);
    assert_eq!(session(&mut s).horizontal.offset(), 1);
    frame(&mut s, 14, 22);
    assert_eq!(session(&mut s).horizontal.offset(), 0);
    key(&mut s, KeyCode::Right);
    assert!(!session(&mut s).expanded(std::path::Path::new("pack")));
    assert_eq!(session(&mut s).horizontal.offset(), 0);
    key(&mut s, KeyCode::Enter);
    ready(&mut s);
    frame(&mut s, 14, 22);
    assert!(session(&mut s).expanded(std::path::Path::new("pack")));
    assert!(session(&mut s).horizontal.overflowing());
    key(&mut s, KeyCode::Right);
    assert_eq!(session(&mut s).horizontal.offset(), 4);
    key(&mut s, KeyCode::Left);
    assert_eq!(session(&mut s).horizontal.offset(), 0);
    assert!(session(&mut s).expanded(std::path::Path::new("pack")));
    key(&mut s, KeyCode::Enter);
    frame(&mut s, 14, 22);
    assert!(!session(&mut s).horizontal.overflowing());
    s.paste("absent");
    ready(&mut s);
    frame(&mut s, 14, 22);
    assert!(session(&mut s).rows().is_empty());
    assert!(!session(&mut s).horizontal.overflowing());
    s.shutdown();
}

#[test]
fn filtered_gestures_preserve_queries_and_keep_tree_and_preview_viewports_independent() {
    let f = support::Fixture::create();
    std::fs::create_dir_all(f.repo.join("parent/中文 目录")).unwrap();
    let name = format!("parent/中文 目录/target-{}-END.txt", "long".repeat(18));
    f.commit(&name, &format!("{}TOKEN\n", "body ".repeat(60)));
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo files");
    ready(&mut s);
    s.paste("target");
    ready(&mut s);
    frame(&mut s, 120, 28);
    let area = tree(&mut s);
    let inside = Position::new(area.x + 2, area.y + 2);
    event(
        &mut s,
        MouseEventKind::ScrollRight,
        inside,
        KeyModifiers::NONE,
    );
    assert_eq!(session(&mut s).horizontal.offset(), 4);
    assert_eq!(s.bar.text(), "target");
    key(&mut s, KeyCode::Left);
    assert_eq!(session(&mut s).horizontal.offset(), 4);
    s.config.mouse.scroll = false;
    assert!(!event(
        &mut s,
        MouseEventKind::ScrollRight,
        inside,
        KeyModifiers::NONE
    ));
    assert_eq!(session(&mut s).horizontal.offset(), 4);
    s.config.mouse.scroll = true;
    s.config.mouse.precise_click = false;
    let end = *track(&s, area).last().unwrap();
    click(&mut s, end);
    assert_eq!(session(&mut s).horizontal.offset(), 4);
    event(
        &mut s,
        MouseEventKind::ScrollDown,
        inside,
        KeyModifiers::SHIFT,
    );
    assert_eq!(session(&mut s).horizontal.offset(), 8);
    s.config.interaction_mode = engine::state::InteractionMode::TextSelection;
    assert!(!event(
        &mut s,
        MouseEventKind::ScrollRight,
        inside,
        KeyModifiers::NONE
    ));
    assert_eq!(session(&mut s).horizontal.offset(), 8);
    s.config.interaction_mode = engine::state::InteractionMode::Application;
    s.config.mouse.precise_click = true;
    key(&mut s, KeyCode::Enter);
    ready(&mut s);
    s.paste("TOKEN");
    frame(&mut s, 120, 28);
    let preview = view::browser::areas(s.body_area.unwrap(), session(&mut s))
        .into_iter()
        .find(|(_, focus)| *focus == Focus::Preview)
        .unwrap()
        .0;
    let before = session(&mut s).pane().position.horizontal.offset();
    assert!(before > 4);
    event(
        &mut s,
        MouseEventKind::ScrollLeft,
        Position::new(preview.x + 2, preview.y + 2),
        KeyModifiers::NONE,
    );
    assert_eq!(
        session(&mut s).pane().position.horizontal.offset(),
        before - 4
    );
    assert_eq!(session(&mut s).horizontal.offset(), 8);
    assert_eq!(s.bar.text(), "TOKEN");
    key(&mut s, KeyCode::Left);
    assert_eq!(
        session(&mut s).pane().position.horizontal.offset(),
        before - 4
    );
    key(&mut s, KeyCode::Esc);
    key(&mut s, KeyCode::Esc);
    assert_eq!(session(&mut s).focus, Focus::Tree);
    assert_eq!(s.bar.text(), "target");
    assert_eq!(session(&mut s).horizontal.offset(), 8);
    assert!(session(&mut s).visit_directory(0));
    ready(&mut s);
    assert_eq!(session(&mut s).horizontal.offset(), 0);
    s.shutdown();
}
