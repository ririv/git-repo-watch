//! Responsive commit graph, metadata preview and per-parent historical file views.
mod detail;
mod pixels;
use super::*;
use crate::history::{Session, View};
use engine::local::history::{Continuation, Entry, Head, Label, References};

pub(super) fn render(
    frame: &mut Frame,
    area: Rect,
    view: &mut View,
    preference: crate::diff::Preference,
    graphics: &mut crate::graphics::Display,
) {
    match view {
        View::Graph(session) => graph(
            frame,
            area,
            session,
            graphics,
            name(&session.references.path),
        ),
        View::Commit(commit) => detail::render(frame, area, commit),
        View::Diff {
            content, scroll, ..
        } => super::diff::render(frame, area, content, scroll, preference),
    }
}

fn time(seconds: i64) -> String {
    match chrono::DateTime::from_timestamp(seconds, 0) {
        Some(time) => time
            .with_timezone(&chrono::Local)
            .format("%Y-%m-%d %H:%M:%S")
            .to_string(),
        None => format!("{}: {seconds}", i18n::tr!(HistoryInvalidTime)),
    }
}

fn field(key: &str, value: impl Into<String>) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{key}: "), Style::default().fg(MUTED)),
        Span::raw(safe(&value.into())),
    ])
}

fn labels(references: &References, entry: &Entry) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    match &references.head {
        Head::Attached { branch, oid } if *oid == entry.oid => spans.push(Span::styled(
            format!("HEAD→{} ", safe(branch)),
            Style::default().fg(ACCENT).bold(),
        )),
        Head::Detached(oid) if *oid == entry.oid => {
            spans.push(Span::styled("HEAD ", Style::default().fg(ACCENT).bold()))
        }
        _ => {}
    }
    if let Some(labels) = references.labels.get(&entry.oid) {
        for label in labels {
            if matches!((&references.head, label), (Head::Attached {branch,oid}, Label::Local(name)) if *oid==entry.oid && branch==name)
            {
                continue;
            }
            let (text, color) = match label {
                Label::Local(name) => (name.clone(), GOOD),
                Label::Remote(name) => (name.clone(), Color::LightCyan),
                Label::Tag(name) => (format!("tag:{name}"), WARN),
            };
            spans.push(Span::styled(
                format!("{} ", safe(&text)),
                Style::default().fg(color),
            ));
        }
    }
    spans
}

pub(super) fn panel(
    frame: &mut Frame,
    area: Rect,
    session: &mut Session,
    graphics: &mut crate::graphics::Display,
) {
    graph(
        frame,
        area,
        session,
        graphics,
        i18n::tr!(WorkspaceHistory).into(),
    );
}

fn graph(
    frame: &mut Frame,
    area: Rect,
    session: &mut Session,
    graphics: &mut crate::graphics::Display,
    title: String,
) {
    let parts = if area.width >= 120 && !session.records.is_empty() {
        Layout::horizontal([Constraint::Min(65), Constraint::Length(42)]).split(area)
    } else {
        Layout::horizontal([Constraint::Percentage(100), Constraint::Length(0)]).split(area)
    };
    let panel = block(title);
    let inner = panel.inner(parts[0]);
    frame.render_widget(panel, parts[0]);
    let query_missing = !session.query.is_empty() && session.state.selected().is_none();
    let shallow = session.references.depth == engine::local::history::Depth::Shallow;
    let header_rows = 2 + u16::from(query_missing) + u16::from(shallow);
    let body = Layout::vertical([Constraint::Length(header_rows), Constraint::Min(1)]).split(inner);
    let mut status = vec![
        Line::styled(i18n::tr!(HistoryScope), Style::default().fg(MUTED)),
        Line::from(format!(
            "{}: {} · {}",
            i18n::tr!(HistoryLoaded),
            session.records.len(),
            if matches!(session.continuation, Continuation::End) {
                i18n::tr!(HistoryEnd)
            } else {
                i18n::tr!(HistoryMore)
            }
        )),
    ];
    if query_missing {
        status.push(Line::styled(
            i18n::tr!(HistoryNoMatch),
            Style::default().fg(WARN),
        ));
    }
    if shallow {
        status.push(Line::styled(
            i18n::tr!(HistoryShallow),
            Style::default().fg(WARN),
        ));
    }
    frame.render_widget(Paragraph::new(status), body[0]);
    let metrics = graphics.metrics();
    let graph_width = if metrics.is_ok() {
        session.width.min((body[1].width as usize / 3).max(1)) as u16
    } else {
        0
    };
    let items: Vec<_> = session
        .records
        .iter()
        .map(|record| {
            let mut spans = vec![
                Span::raw(" ".repeat(usize::from(graph_width))),
                Span::styled(
                    format!("  {} ", &record.entry.oid.to_string()[..8]),
                    Style::default().fg(WARN),
                ),
            ];
            spans.extend(labels(&session.references, &record.entry));
            spans.push(Span::raw(safe(&record.entry.subject)));
            ListItem::new(Line::from(spans))
        })
        .collect();
    if items.is_empty() {
        frame.render_widget(
            Paragraph::new(i18n::tr!(HistoryEmpty)).style(Style::default().fg(MUTED)),
            body[1],
        );
    } else {
        frame.render_stateful_widget(
            List::new(items)
                .highlight_symbol("› ")
                .highlight_spacing(ratatui::widgets::HighlightSpacing::Always)
                .highlight_style(selected_style()),
            body[1],
            &mut session.state,
        );
        match metrics {
            Ok(metrics) => pixels::render(
                frame,
                body[1],
                session,
                graphics,
                metrics,
                graph_width,
                body[0],
            ),
            Err(error) => {
                frame.render_widget(
                    Paragraph::new(error).style(Style::default().fg(WARN)),
                    Rect {
                        height: 1,
                        ..body[0]
                    },
                );
            }
        }
    }
    if parts[1].width > 0 {
        let text = if let Some(entry) = session.selected() {
            let mut lines = vec![
                field(i18n::tr!(HistoryHash), entry.oid.to_string()),
                field(i18n::tr!(HistoryAuthor), &entry.author),
                field(i18n::tr!(HistoryTime), time(entry.seconds)),
                field(
                    i18n::tr!(HistoryParents),
                    if entry.parents().is_empty() {
                        i18n::tr!(HistoryRoot).into()
                    } else {
                        entry
                            .parents()
                            .iter()
                            .map(|oid| oid.to_string()[..12].to_owned())
                            .collect::<Vec<_>>()
                            .join(" · ")
                    },
                ),
            ];
            lines.push(Line::default());
            lines.push(Line::from(labels(&session.references, entry)));
            lines.push(Line::default());
            lines.push(Line::raw(safe(&entry.subject)));
            Text::from(lines)
        } else {
            Text::default()
        };
        frame.render_widget(
            Paragraph::new(text).wrap(Wrap { trim: false }).block(block(
                session
                    .selected()
                    .map(|entry| entry.oid.to_string()[..12].to_owned())
                    .unwrap_or_else(|| i18n::tr!(HistoryTitle).into()),
            )),
            parts[1],
        );
    }
}
