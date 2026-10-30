//! Lazy file trees and immutable source previews share the repository viewport.
mod content;
mod layout;
mod path;
use super::*;
use crate::{
    browser::{Focus, Session},
    pointer::Target,
    workspace::Arrangement,
};

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
    graphics: &mut crate::graphics::Display,
    hits: &mut crate::pointer::Map,
) {
    path::render(frame, path::areas(area).0, session, hits);
    for (area, focus) in areas(area, session) {
        match focus {
            Focus::Tree => tree(frame, area, session, hits),
            Focus::Preview => preview(frame, area, session, wrap, graphics, hits),
        }
        if focus == session.focus {
            highlight_border(frame, area);
        }
    }
}
fn tree(frame: &mut Frame, area: Rect, session: &mut Session, hits: &mut crate::pointer::Map) {
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
    let rows = session.rows();
    let width = parts[1].width as usize;
    let name_width = if width >= 3 { width - 3 } else { width };
    let labels = rows
        .iter()
        .map(|row| {
            let entry = row.entry;
            let status = entry.status();
            let color = if status.is_some_and(|status| status.changed()) {
                WARN
            } else {
                Color::Reset
            };
            let path = if session.query.is_empty() {
                entry
                    .path
                    .strip_prefix(row.head.parent().expect("nonempty browser path"))
                    .expect("compact row stays within its parent")
                    .to_string_lossy()
            } else {
                entry.path.to_string_lossy()
            };
            let label = if entry.directory() {
                let mut label = format!(
                    "{}{path}/",
                    if session.expanded(row.head) {
                        "▼  "
                    } else {
                        "▶  "
                    }
                );
                let indicator = if session.directory_loading(&entry.path) {
                    Some(session.spinner())
                } else if session.directory_notice(&entry.path).is_some() {
                    Some("!")
                } else {
                    None
                };
                if let Some(indicator) = indicator {
                    label.push(' ');
                    label.push_str(indicator);
                }
                label
            } else {
                format!("   {path}")
            };
            let label = Line::styled(
                format!("{}{}", "  ".repeat(row.depth), safe(&label)),
                Style::default().fg(if entry.directory() { ACCENT } else { color }),
            );
            (label, status.map(|status| status.label()), color)
        })
        .collect::<Vec<_>>();
    let count = rows.len();
    drop(rows);
    session.horizontal.constrain(
        labels
            .iter()
            .map(|(label, _, _)| label.width())
            .max()
            .unwrap_or(0)
            .saturating_sub(name_width),
    );
    let items = labels
        .into_iter()
        .map(|(label, status, color)| {
            let mut line = crate::horizontal::clip(&label, session.horizontal.offset(), name_width);
            if width >= 3 {
                line.spans
                    .push(Span::raw(" ".repeat(width - 2 - line.width())));
                line.spans.push(Span::styled(
                    status.unwrap_or_else(|| "  ".into()),
                    Style::default().fg(color),
                ));
            }
            ListItem::new(line)
        })
        .collect::<Vec<_>>();
    frame.render_stateful_widget(
        List::new(items).highlight_style(selected_style()),
        parts[1],
        &mut session.state,
    );
    if count == 0 && session.notice().is_none() {
        frame.render_widget(
            Paragraph::new(i18n::tr!(BrowserEmpty)).style(Style::default().fg(MUTED)),
            parts[1],
        );
    }
    crate::scrollbar::vertical(frame, area, parts[1], count, session.state.offset());
    hits.list(
        area,
        parts[1],
        &(0..count).collect::<Vec<_>>(),
        session.state.offset(),
        Target::Browser,
    );
    session.horizontal.render_on_border(
        frame,
        area,
        parts[1].x..parts[1].x + name_width as u16,
        hits,
        crate::pointer::ScrollTarget::BrowserTreeHorizontal,
    );
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
