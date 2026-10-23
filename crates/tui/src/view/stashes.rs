//! Responsive stash selection and complete immutable entry details.
use super::*;
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
pub(super) fn render(frame: &mut Frame, area: Rect, view: &mut View) {
    match view {
        View::Detail {
            session,
            entry,
            scroll,
        } => review::text_view(
            frame,
            area,
            &details(session, entry),
            scroll,
            i18n::tr!(StashTitle),
        ),
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
            let items = rows
                .iter()
                .map(|entry| {
                    let index = entry.index;
                    ListItem::new(Line::from(vec![
                        Span::styled(format!("stash@{{{index}}} "), Style::default().fg(ACCENT)),
                        Span::styled(
                            format!("{} ", &entry.oid.to_string()[..8]),
                            Style::default().fg(WARN),
                        ),
                        Span::raw(safe(&entry.message)),
                    ]))
                })
                .collect::<Vec<_>>();
            let count = items.len();
            let items = if items.is_empty() {
                vec![ListItem::new(i18n::tr!(StashEmpty))]
            } else {
                items
            };
            let preview = session.selected().map(|entry| details(session, entry));
            frame.render_stateful_widget(
                List::new(items)
                    .block(block(i18n::tr!(StashTitle)))
                    .highlight_style(selected_style())
                    .highlight_symbol("› "),
                areas[0],
                &mut session.state,
            );
            crate::scrollbar::vertical(
                frame,
                areas[0],
                block("").inner(areas[0]),
                count,
                session.state.offset(),
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
