//! Lazy file trees and immutable source previews share the repository viewport.
mod content;
mod layout;
mod path;
use super::*;
use crate::search::highlight::{self, Emphasis};
use crate::{
    browser::{Focus, Session},
    pointer::Target,
    workspace::Arrangement,
};
use unicode_width::UnicodeWidthStr;

pub(crate) fn areas(area: Rect, session: &Session) -> Vec<(Rect, Focus)> {
    let (_, area) = path::areas(area);
    if area.width < 75 || area.height < 8 || session.arrangement == Arrangement::Maximized {
        return vec![(area, session.focus)];
    }
    let parts = Layout::horizontal([
        Constraint::Length((area.width * 2 / 5).clamp(24, 42)),
        Constraint::Min(1),
    ])
    .split(area);
    vec![(parts[0], Focus::Tree), (parts[1], Focus::Preview)]
}
pub(crate) fn preview_mode_at(
    area: Rect,
    session: &Session,
    point: ratatui::layout::Position,
) -> Option<crate::browser::opening::TextMode> {
    if point.y == area.y || session.panes.feedback.previous().is_some() {
        return None;
    }
    layout::panes(area, session)
        .into_iter()
        .find(|pane| pane.frame.contains(point))
        .map(|pane| pane.mode)
}
pub(super) fn render(
    frame: &mut Frame,
    area: Rect,
    session: &mut Session,
    wrap: bool,
    icons: crate::file_icons::Display,
    graphics: &mut crate::graphics::Display,
    hits: &mut crate::pointer::Map,
) {
    path::render(frame, path::areas(area).0, session, hits);
    for (area, focus) in areas(area, session) {
        match focus {
            Focus::Tree => tree(frame, area, session, icons, hits),
            Focus::Preview => preview(frame, area, session, wrap, graphics, hits),
        }
        if focus == session.focus {
            highlight_border(frame, area);
        } else {
            inactive_selection(frame, area);
        }
    }
}
fn tree(
    frame: &mut Frame,
    area: Rect,
    session: &mut Session,
    icons: crate::file_icons::Display,
    hits: &mut crate::pointer::Map,
) {
    let panel = block(i18n::tr!(BrowserTitle));
    let inner = panel.inner(area);
    frame.render_widget(panel, area);
    let notice = session.notice().map(str::to_owned);
    let parts = Layout::vertical([
        Constraint::Length(u16::from(notice.is_some())),
        Constraint::Min(0),
    ])
    .split(inner);
    if let Some(notice) = notice {
        frame.render_widget(
            Paragraph::new(safe(&notice)).style(Style::default().fg(MUTED)),
            parts[0],
        );
    }
    let mut horizontal = std::mem::take(&mut session.horizontal);
    let rows = session.rows();
    let count = rows.len();
    let mut state = session.state;
    let visible = list_window::visible(parts[1], count, &mut state);
    let selection = row_selection(state.selected(), count);
    let width = usize::from(parts[1].width);
    let status_width = if width >= 3 { 3 } else { 0 };
    let path = |row: crate::browser::Row<'_>| -> String {
        let entry = row.entry;
        let path = if session.query.is_empty() {
            entry
                .path
                .strip_prefix(row.head.parent().expect("nonempty browser path"))
                .expect("compact row stays within its parent")
                .to_string_lossy()
        } else {
            entry.path.to_string_lossy()
        };
        let mut label = safe(&path);
        if entry.directory() {
            label.push('/');
            if let Some(indicator) = if session.directory_loading(&entry.path) {
                Some(session.spinner())
            } else if session.directory_notice(&entry.path).is_some() {
                Some("!")
            } else {
                None
            } {
                label.push(' ');
                label.push_str(indicator);
            }
        }
        label
    };
    // 文件浏览的图标栏始终占两列，隐藏图标时也保留宽度。
    let widest = rows
        .iter()
        .copied()
        .map(|row| row.depth * 2 + 2 + path(row).width())
        .max()
        .unwrap_or(0);
    let paths = crate::horizontal::Columns::new(parts[1], 0, status_width).scroll(
        frame,
        area,
        hits,
        crate::horizontal::Pane::Browser,
        &mut horizontal,
        widest,
    );
    list_window::render(frame, parts[1], visible.clone(), |index| {
        let row = rows[index];
        let entry = row.entry;
        let status = entry.status();
        let color = if status.is_some_and(|status| status.changed()) {
            WARN
        } else {
            Color::Reset
        };
        let mut spans = vec![Span::raw("  ".repeat(row.depth))];
        if entry.directory() {
            let folder = if session.expanded(row.head) {
                crate::file_icons::Folder::Expanded
            } else {
                crate::file_icons::Folder::Collapsed
            };
            spans.extend(icons.directory_slot(folder));
        } else {
            spans.extend(icons.slot(&entry.path));
        }
        spans.extend(
            highlight::line(
                Line::styled(
                    path(row),
                    Style::default().fg(if entry.directory() { ACCENT } else { color }),
                ),
                &session.query,
                Emphasis::at(index, selection),
            )
            .spans,
        );
        let status = (status_width > 0).then(|| {
            Span::styled(
                status.map_or_else(|| "  ".into(), |status| status.label()),
                Style::default().fg(color),
            )
        });
        ListItem::new(paths.line(&Line::from(spans), status))
            .style(row_style(selection == Some(index)))
    });
    session.state = state;
    session.horizontal = horizontal;
    if count == 0 && session.notice().is_none() {
        frame.render_widget(
            Paragraph::new(i18n::tr!(BrowserEmpty)).style(Style::default().fg(MUTED)),
            parts[1],
        );
    }
    crate::scrollbar::vertical(frame, area, parts[1], count, session.state.offset());
    hits.sequential_list(area, parts[1], count, visible, Target::Browser);
}
fn preview(
    frame: &mut Frame,
    area: Rect,
    session: &mut Session,
    wrap: bool,
    graphics: &mut crate::graphics::Display,
    hits: &mut crate::pointer::Map,
) {
    let current = layout::panes(area, session);
    for pane in &current {
        session.preview_width(pane.mode, pane.content.width);
    }
    let retained = session.panes.feedback.previous();
    let active = retained.map_or_else(
        || session.presentation().active(),
        |snapshot| snapshot.target.presentation.active(),
    );
    let panes = retained.map_or(current, |snapshot| layout::retained(area, snapshot));
    let frozen = retained.is_some();
    let loading_visible = frozen || session.panes.feedback.visible();
    let panel = block("");
    frame.render_widget(panel, area);
    let spinner = session.spinner().to_owned();
    let notice = session
        .selected()
        .and_then(|entry| session.directory_notice(&entry.path).map(str::to_owned))
        .unwrap_or_else(|| i18n::tr!(BrowserSelectFile).into());
    let mut retained_hits = crate::pointer::Map::default();
    for pane in panes {
        layout::decorate(
            frame,
            &pane,
            session.focus == Focus::Preview && active == pane.mode,
        );
        let view = match session.panes.feedback.previous_mut() {
            Some(snapshot) => {
                let view = snapshot.get_mut(pane.mode);
                content::View {
                    preview: &view.preview,
                    position: &mut view.position,
                    layout: &mut view.layout,
                    find: &view.find,
                }
            }
            None => {
                let view = session.panes.get_mut(pane.mode);
                content::View {
                    preview: &view.preview,
                    position: &mut view.position,
                    layout: &mut view.layout,
                    find: &view.find,
                }
            }
        };
        content::render(
            frame,
            &pane,
            view,
            content::Options {
                spinner: &spinner,
                notice: &notice,
                visible: loading_visible,
                wrap,
            },
            graphics,
            if frozen {
                &mut retained_hits
            } else {
                &mut *hits
            },
        );
    }
}
