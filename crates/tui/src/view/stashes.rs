//! Responsive stash selection and complete immutable entry details.
use super::*;
use crate::search::highlight::{self, Emphasis};
use crate::stashes::{Session, View};
fn details(session: &Session, entry: &engine::local::stash::Entry) -> String {
    format!(
        "{}: {}\n{}: {}\n{}: {}\n{}: {}\n\n{}",
        i18n::tr!(Path),
        safe(&session.snapshot.path.to_string_lossy()),
        i18n::tr!(HistoryHash),
        entry.oid,
        i18n::tr!(HistoryTime),
        chrono::DateTime::from_timestamp(entry.time, 0)
            .map(|t| t
                .with_timezone(&chrono::Local)
                .format("%Y-%m-%d %H:%M:%S")
                .to_string())
            .unwrap_or_else(|| i18n::tr!(HistoryInvalidTime).into()),
        i18n::tr!(StashPosition),
        format_args!("stash@{{{}}}", entry.index),
        safe(&entry.message)
    )
}
pub(super) fn render(
    frame: &mut Frame,
    area: Rect,
    view: &mut View,
    options: crate::diff::Options,
    icons: crate::file_icons::Display,
    graphics: &mut crate::graphics::Display,
    hits: &mut crate::pointer::Map,
) {
    match view {
        View::Detail { inspect, .. } => {
            super::inspection::render(frame, area, inspect, options, icons, hits)
        }
        View::Diff {
            content, scroll, ..
        } => super::document::render(frame, area, content, scroll, options, graphics, hits),
        View::List(session) => {
            let areas = if area.width >= 100 {
                let parts =
                    Layout::horizontal([Constraint::Percentage(55), Constraint::Percentage(45)])
                        .split(area);
                vec![parts[0], parts[1]]
            } else {
                vec![area]
            };
            let rows = session.rows();
            let selection = row_selection(session.state.selected(), rows.len());
            let lines = rows
                .iter()
                .enumerate()
                .map(|(row, entry)| {
                    let index = entry.index;
                    let mut spans = vec![Span::styled(
                        format!("stash@{{{index}}} "),
                        Style::default().fg(ACCENT),
                    )];
                    spans.extend(
                        highlight::line(
                            Line::from(vec![
                                Span::styled(
                                    format!("{} ", &entry.oid.to_string()[..8]),
                                    Style::default().fg(WARN),
                                ),
                                Span::raw(safe(&entry.message)),
                            ]),
                            &session.query,
                            Emphasis::at(row, selection),
                        )
                        .spans,
                    );
                    Line::from(spans)
                })
                .collect::<Vec<_>>();
            let count = lines.len();
            let preview = session.selected().map(|entry| details(session, entry));
            let panel = block(i18n::tr!(StashTitle));
            let inner = panel.inner(areas[0]);
            frame.render_widget(panel, areas[0]);
            list_window::Scrolling {
                panel: areas[0],
                inner,
                pane: crate::horizontal::Pane::Stashes,
                viewport: &mut session.horizontal,
            }
            .render(
                frame,
                hits,
                &mut session.state,
                &lines,
                Some(i18n::tr!(StashEmpty)),
            );
            crate::scrollbar::vertical(frame, areas[0], inner, count, session.state.offset());
            hits.list(
                areas[0],
                inner,
                &(0..count).collect::<Vec<_>>(),
                session.state.offset(),
                crate::pointer::Target::Stashes,
            );
            if areas.len() > 1 {
                // 静态预览没有独立滚动操作；完整详情才显示滚动条。
                frame.render_widget(
                    Paragraph::new(preview.as_deref().unwrap_or(i18n::tr!(StashEmpty)))
                        .wrap(Wrap { trim: false })
                        .block(block(i18n::tr!(StashDetails))),
                    areas[1],
                );
            }
        }
    }
}
