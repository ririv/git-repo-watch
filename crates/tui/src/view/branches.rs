//! Local/remote branch list and explicit branch metadata share one snapshot.
use super::*;
use crate::branches::{Session, View};
use engine::local::branches::{
    catalog::{Head, Kind},
    upstream::{Comparison, Tracking},
};

pub(super) fn render(
    frame: &mut Frame,
    area: Rect,
    view: &mut View,
    hits: &mut crate::pointer::Map,
) {
    match view {
        View::List(session) => list(frame, area, session, hits),
        View::Detail {
            session,
            entry,
            scroll,
        } => {
            let mut lines = vec![
                field(i18n::tr!(Branch), &entry.name),
                field(i18n::tr!(Path), &session.snapshot.path.to_string_lossy()),
                field(i18n::tr!(BranchReference), &entry.reference),
                field(i18n::tr!(HistoryHash), &entry.oid.to_string()),
                field(i18n::tr!(HistoryAuthor), &entry.author),
            ];
            let time = chrono::DateTime::from_timestamp(entry.time, 0)
                .map(|time| {
                    time.with_timezone(&chrono::Local)
                        .format("%Y-%m-%d %H:%M:%S")
                        .to_string()
                })
                .unwrap_or_else(|| format!("{}: {}", i18n::tr!(HistoryInvalidTime), entry.time));
            lines.push(field(i18n::tr!(HistoryTime), &time));
            if let Kind::Local(tracking) = &entry.kind {
                lines.push(field(i18n::tr!(OverviewUpstream), &upstream(tracking)));
                lines.push(Line::styled(
                    i18n::tr!(BranchLocalRefs),
                    Style::default().fg(MUTED),
                ));
            }
            lines.push(Line::default());
            lines.push(Line::raw(safe(&entry.subject)));
            review::styled_text_view(
                frame,
                area,
                Text::from(lines),
                scroll,
                &safe(&entry.name),
                hits,
                crate::pointer::ScrollTarget::Text,
            );
        }
    }
}
fn field(label: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            i18n::tf!(FieldLabel, label = label),
            Style::default().fg(MUTED),
        ),
        Span::raw(safe(value)),
    ])
}
pub(super) fn list(
    frame: &mut Frame,
    area: Rect,
    session: &mut Session,
    hits: &mut crate::pointer::Map,
) {
    let panel = block(i18n::tr!(BranchTitle));
    let inner = panel.inner(area);
    frame.render_widget(panel, area);
    let parts = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).split(inner);
    let head = match &session.snapshot.head {
        Head::Branch(_) => String::new(),
        Head::Detached(oid) => format!(
            " · {} {}",
            i18n::tr!(OverviewDetached),
            &oid.to_string()[..8]
        ),
        Head::Unborn(name) => format!(" · {} · {}", safe(name), i18n::tr!(OverviewUnborn)),
    };
    let rows = session.rows();
    frame.render_widget(
        Paragraph::new(format!("{} · {}{head}", session.filter.title(), rows.len()))
            .style(Style::default().fg(MUTED)),
        parts[0],
    );
    let items = rows
        .iter()
        .map(|entry| {
            let current = session.snapshot.current(entry);
            let (kind, color, suffix) = match &entry.kind {
                Kind::Local(tracking) => (i18n::tr!(BranchLocal), GOOD, compact(tracking)),
                Kind::Remote => (i18n::tr!(BranchRemote), ACCENT, String::new()),
            };
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{} ", if current { "*" } else { " " }),
                    Style::default().fg(GOOD).bold(),
                ),
                Span::styled(
                    format!("{} ", safe(&entry.name)),
                    Style::default().fg(color),
                ),
                Span::styled(format!("{kind}{suffix}"), Style::default().fg(MUTED)),
            ]))
        })
        .collect::<Vec<_>>();
    let count = items.len();
    let items = if items.is_empty() {
        vec![ListItem::new(i18n::tr!(BranchEmpty))]
    } else {
        items
    };
    frame.render_stateful_widget(
        List::new(items)
            .highlight_style(selected_style())
            .highlight_symbol("› "),
        parts[1],
        &mut session.state,
    );
    crate::scrollbar::vertical(frame, area, parts[1], count, session.state.offset());
    hits.list(
        area,
        parts[1],
        &(0..count).collect::<Vec<_>>(),
        session.state.offset(),
        crate::pointer::Target::Branches,
    );
}
fn compact(tracking: &Result<Tracking, String>) -> String {
    match tracking {
        Ok(Tracking::Absent) => String::new(),
        Ok(Tracking::Local { comparison, .. } | Tracking::Remote { comparison, .. }) => {
            match comparison {
                Comparison::Known { ahead, behind } => format!(" ↑{ahead} ↓{behind}"),
                Comparison::Missing => format!(" · {}", i18n::tr!(OverviewGone)),
            }
        }
        Err(_) => " !".into(),
    }
}
fn upstream(tracking: &Result<Tracking, String>) -> String {
    match tracking {
        Ok(Tracking::Absent) => i18n::tr!(NoUpstream).into(),
        Ok(Tracking::Local { name, comparison } | Tracking::Remote { name, comparison }) => {
            match comparison {
                Comparison::Known { ahead, behind } => format!("{name} · ↑{ahead} ↓{behind}"),
                Comparison::Missing => format!("{name} · {}", i18n::tr!(OverviewGone)),
            }
        }
        Err(error) => multiline(error),
    }
}
