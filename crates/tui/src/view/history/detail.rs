//! A commit message sits above file navigation and one continuous diff viewport.
mod files;
mod message;
mod preview;
use super::*;
use crate::history::{Commit, Focus};
use crate::workspace::Arrangement;

pub(crate) fn areas(area: Rect, commit: &Commit) -> Vec<(Rect, Focus)> {
    if commit.arrangement == Arrangement::Maximized {
        return vec![(area, commit.focus)];
    }
    let wide = area.width >= 100 && area.height >= 10;
    let header = (area.height / 3)
        .clamp(if wide { 6 } else { 4 }, 9)
        .min(message::desired_height(commit, area.width))
        .min(area.height.saturating_sub(3));
    let rows = Layout::vertical([Constraint::Length(header), Constraint::Min(0)]).split(area);
    let mut result = vec![(rows[0], Focus::Message)];
    if wide {
        let columns = Layout::horizontal([
            Constraint::Length((area.width / 3).clamp(24, 42)),
            Constraint::Min(0),
        ])
        .split(rows[1]);
        result.extend([(columns[0], Focus::Files), (columns[1], Focus::Preview)]);
    } else {
        result.push((rows[1], Focus::Files));
    }
    result
        .into_iter()
        .filter(|(area, _)| !area.is_empty())
        .collect()
}
pub(super) fn render(
    frame: &mut Frame,
    area: Rect,
    commit: &mut Commit,
    options: crate::diff::Options,
    icons: crate::file_icons::Display,
    hits: &mut crate::pointer::Map,
) {
    let parts = areas(area, commit);
    if !parts.iter().any(|(_, focus)| *focus == commit.focus) {
        commit.focus = Focus::Files;
    }
    for (panel, focus) in parts {
        match focus {
            Focus::Message => message::render(frame, panel, commit, hits),
            Focus::Files => files::render(frame, panel, commit, icons, hits),
            Focus::Preview => preview::render(frame, panel, commit, options, hits),
        }
        if focus == commit.focus {
            highlight_border(frame, panel);
        }
    }
}
