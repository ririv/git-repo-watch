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
            Section::Branches => branch_list(frame, screen, pane_area),
            Section::Worktree => files(frame, screen, pane_area),
            Section::History => graph(frame, screen, pane_area),
        }
        if Some(section) == Section::focused(&screen.mode) {
            highlight_border(frame, pane_area);
        }
    }
}
fn tiled_areas(area: Rect) -> Vec<(Rect, Section)> {
    let rows = Layout::vertical([
        Constraint::Length((area.height * 2 / 5).clamp(5, 12)),
        Constraint::Min(1),
    ])
    .split(area);
    let columns = [Constraint::Percentage(45), Constraint::Percentage(55)];
    let upper = Layout::horizontal(columns).split(rows[0]);
    let lower = Layout::horizontal(columns).split(rows[1]);
    vec![
        (upper[0], Section::Overview),
        (upper[1], Section::Branches),
        (lower[0], Section::Worktree),
        (lower[1], Section::History),
    ]
}
fn branch_list(frame: &mut Frame, screen: &mut Screen, area: Rect) {
    if let Some(session) = active_branches(&mut screen.mode) {
        branches::list(frame, area, session);
    } else if let Some(session) = screen.workspace.session_mut() {
        match &mut session.branches {
            Panel::Ready(session) => branches::list(frame, area, session),
            Panel::Loading => placeholder(
                frame,
                area,
                Section::Branches,
                &format!(
                    "{} {}",
                    screen.workspace.spinner(),
                    i18n::tr!(BranchCatalogRead)
                ),
            ),
            Panel::Failed(error) => placeholder(frame, area, Section::Branches, &multiline(error)),
        }
    }
}
fn active_branches(mode: &mut Mode) -> Option<&mut crate::branches::Session> {
    match mode {
        Mode::Branches(view) => match view.as_mut() {
            crate::branches::View::List(session) => Some(session),
            _ => None,
        },
        Mode::Busy(pending) => active_branches(&mut pending.origin),
        _ => None,
    }
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
