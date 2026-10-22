//! Overview, files and the existing pixel history graph remain visible together.
use super::*;
use crate::workspace::{Arrangement, Panel, Section};

pub(super) fn render(frame: &mut Frame, screen: &mut Screen, area: Rect) {
    let focused = Section::of(&screen.mode);
    screen.workspace_areas = match screen
        .workspace
        .session()
        .expect("open repository")
        .arrangement
    {
        Arrangement::Tiled => tiled_areas(area),
        Arrangement::Maximized => vec![(area, focused)],
    };
    for (pane_area, section) in screen.workspace_areas.clone() {
        match section {
            Section::Overview => overview::panel(frame, screen, pane_area),
            Section::Worktree => files(frame, screen, pane_area),
            Section::History => graph(frame, screen, pane_area),
        }
        if section == focused {
            highlight_border(frame, pane_area);
        }
    }
}
fn tiled_areas(area: Rect) -> Vec<(Rect, Section)> {
    let (overview_area, files_area, history_area) = if area.width >= 110 {
        let columns = Layout::horizontal([
            Constraint::Length((area.width / 3).clamp(36, 52)),
            Constraint::Min(1),
        ])
        .split(area);
        let left = Layout::vertical([
            Constraint::Length(area.height.saturating_sub(7).clamp(5, 12)),
            Constraint::Min(1),
        ])
        .split(columns[0]);
        (left[0], left[1], columns[1])
    } else {
        let rows = Layout::vertical([
            Constraint::Length((area.height / 3).clamp(5, 9)),
            Constraint::Min(1),
        ])
        .split(area);
        let lower = Layout::horizontal([Constraint::Percentage(42), Constraint::Percentage(58)])
            .split(rows[1]);
        (rows[0], lower[0], lower[1])
    };
    vec![
        (overview_area, Section::Overview),
        (files_area, Section::Worktree),
        (history_area, Section::History),
    ]
}
fn files(frame: &mut Frame, screen: &Screen, files_area: Rect) {
    if let Some(files) = active_files(&screen.mode) {
        super::files::render(frame, files_area, files, i18n::tr!(Worktree));
    } else if let Some(session) = screen.workspace.session() {
        match &session.files {
            Panel::Ready(files) => {
                super::files::render(frame, files_area, files, i18n::tr!(Worktree))
            }
            Panel::Loading => placeholder(
                frame,
                files_area,
                Section::Worktree,
                &format!(
                    "{} {}",
                    screen.workspace.spinner(),
                    i18n::tr!(ReadRepositoryFiles)
                ),
            ),
            Panel::Failed(error) => {
                placeholder(frame, files_area, Section::Worktree, &multiline(error))
            }
        }
    }
}
fn graph(frame: &mut Frame, screen: &mut Screen, history_area: Rect) {
    if let Some(session) = active_history(&mut screen.mode) {
        history::panel(frame, history_area, session, &mut screen.graphics);
    } else if let Some(session) = screen.workspace.session_mut() {
        match &mut session.history {
            Panel::Ready(graph) => history::panel(frame, history_area, graph, &mut screen.graphics),
            Panel::Loading => placeholder(
                frame,
                history_area,
                Section::History,
                &format!("{} {}", screen.workspace.spinner(), i18n::tr!(HistoryRead)),
            ),
            Panel::Failed(error) => {
                placeholder(frame, history_area, Section::History, &multiline(error))
            }
        }
    }
}
fn active_files(mode: &Mode) -> Option<&FileSession> {
    match mode {
        Mode::Files(files) => Some(files),
        Mode::Busy(pending) => active_files(&pending.origin),
        _ => None,
    }
}
fn active_history(mode: &mut Mode) -> Option<&mut crate::history::Session> {
    match mode {
        Mode::History(view) => match view.as_mut() {
            crate::history::View::Graph(session) => Some(session),
            _ => None,
        },
        Mode::Busy(pending) => active_history(&mut pending.origin),
        _ => None,
    }
}
fn placeholder(frame: &mut Frame, area: Rect, section: Section, text: &str) {
    frame.render_widget(
        Paragraph::new(text.to_owned())
            .wrap(Wrap { trim: false })
            .block(block(section.title())),
        area,
    );
}
fn highlight_border(frame: &mut Frame, area: Rect) {
    for y in area.y..area.bottom() {
        for x in area.x..area.right() {
            if x == area.x || x + 1 == area.right() || y == area.y || y + 1 == area.bottom() {
                frame.buffer_mut()[(x, y)].set_fg(ACCENT);
            }
        }
    }
}
