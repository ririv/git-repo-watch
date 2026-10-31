//! Responsive commit graph, metadata preview and per-parent historical file views.
pub(crate) mod detail;
pub(super) mod menu;
mod pixels;
use super::*;
use crate::history::{Session, View};
use crate::search::highlight::{self, Emphasis};
use engine::local::history::{Continuation, Entry, Head, Label, References};

pub(super) fn render(
    frame: &mut Frame,
    area: Rect,
    view: &mut View,
    preference: crate::diff::Options,
    icons: crate::file_icons::Display,
    graphics: &mut crate::graphics::Display,
    hits: &mut crate::pointer::Map,
) {
    match view {
        View::Graph(session) => graph(
            frame,
            area,
            session,
            graphics,
            hits,
            name(&session.references.path),
        ),
        View::Commit(commit) => detail::render(frame, area, commit, preference, icons, hits),
        View::Diff {
            content, scroll, ..
        } => super::document::render(frame, area, content, scroll, preference, graphics, hits),
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
    hits: &mut crate::pointer::Map,
) {
    graph(
        frame,
        area,
        session,
        graphics,
        hits,
        i18n::tr!(WorkspaceHistory).into(),
    );
}

fn graph(
    frame: &mut Frame,
    area: Rect,
    session: &mut Session,
    graphics: &mut crate::graphics::Display,
    hits: &mut crate::pointer::Map,
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
    let query_missing = !session.query.is_empty() && session.matches().is_empty();
    let shallow = session.references.depth == engine::local::history::Depth::Shallow;
    let header_rows = 2 + u16::from(query_missing) + u16::from(shallow);
    let body = Layout::vertical([Constraint::Length(header_rows), Constraint::Min(1)]).split(inner);
    let selection = if session.selection.active() {
        Span::styled(
            format!(
                "{}: {} · ",
                i18n::tr!(HistoryMarked),
                session.selection.count()
            ),
            Style::default().fg(ACCENT),
        )
    } else {
        Span::raw("")
    };
    // 图形提示只覆盖范围说明，始终保留已选数量，不额外增加状态行。
    let selection_width = selection.width().min(usize::from(body[0].width)) as u16;
    let graphics_status = Rect::new(
        body[0].x + selection_width,
        body[0].y,
        body[0].width.saturating_sub(selection_width),
        body[0].height.min(1),
    );
    let mut status = vec![
        Line::from(vec![
            selection,
            Span::styled(i18n::tr!(HistoryScope), Style::default().fg(MUTED)),
        ]),
        Line::from(format!(
            "{}: {} · {}",
            i18n::tr!(HistoryLoaded),
            session.records.len(),
            if matches!(session.continuation, Continuation::End) {
                i18n::tr!(HistoryEnd)
            } else if session.query.is_empty() {
                i18n::tr!(HistoryMore)
            } else {
                i18n::tr!(HistoryMoreSearch)
            },
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
    let content_area = body[1];
    let checkbox_width = if session.selection.active() {
        super::checkbox::WIDTH.min(content_area.width)
    } else {
        0
    };
    // 图统一负责自身左侧的一格留白，不由复选框或列表外边距提供。
    let graph_inset = 1u16.min(content_area.width.saturating_sub(checkbox_width));
    let graph_area = Rect::new(
        content_area.x + checkbox_width + graph_inset,
        content_area.y,
        content_area
            .width
            .saturating_sub(checkbox_width + graph_inset),
        content_area.height,
    );
    let graph_width = if metrics.is_ok() {
        (session.width.min((graph_area.width as usize / 3).max(1)) as u16).min(graph_area.width)
    } else {
        0
    };
    // 图线与文字共用横向位置，各自只滚动溢出的部分，短哈希始终固定。
    let text_width = graph_area.width.saturating_sub(graph_width + 11) as usize;
    let lines: Vec<_> = session
        .records
        .iter()
        .map(|record| {
            let mut spans = labels(&session.references, &record.entry);
            spans.push(Span::raw(safe(&record.entry.subject)));
            Line::from(spans)
        })
        .collect();
    let widest = lines.iter().map(Line::width).max().unwrap_or(0);
    let graph_overflow = if graph_width > 0 {
        session.width.saturating_sub(usize::from(graph_width))
    } else {
        0
    };
    let text_overflow = widest.saturating_sub(text_width);
    session
        .horizontal
        .constrain(graph_overflow.max(text_overflow));
    let text_offset = session.horizontal.offset().min(text_overflow);
    let scroll_start = if graph_overflow > 0 {
        graph_area.x
    } else {
        content_area.right() - text_width as u16
    };
    let scroll_end = if text_overflow > 0 {
        content_area.right()
    } else {
        graph_area.x + graph_width
    };
    session.horizontal.render_on_border(
        frame,
        parts[0],
        scroll_start..scroll_end,
        hits,
        crate::pointer::ScrollTarget::HistoryHorizontal,
    );
    let selection = row_selection(session.state.selected(), session.records.len());
    let items: Vec<_> = session
        .records
        .iter()
        .zip(lines)
        .enumerate()
        .map(|(index, (record, line))| {
            let selected = selection == Some(index);
            let emphasis = Emphasis::at(index, selection);
            let mut spans = if session.selection.active() {
                super::checkbox::spans(if session.selection.contains(&record.entry.oid) {
                    crate::files::CheckState::Checked
                } else {
                    crate::files::CheckState::Unchecked
                })
            } else {
                Vec::new()
            };
            spans.extend([
                Span::raw(" ".repeat(usize::from(graph_inset))),
                Span::raw(" ".repeat(usize::from(graph_width))),
            ]);
            spans.extend(
                highlight::line(
                    Line::styled(
                        format!("  {} ", &record.entry.oid.to_string()[..8]),
                        Style::default().fg(WARN),
                    ),
                    &session.query,
                    emphasis,
                )
                .spans,
            );
            let line = highlight::line(line, &session.query, emphasis);
            spans.extend(crate::horizontal::clip(&line, text_offset, text_width).spans);
            ListItem::new(Line::from(spans)).style(
                if !session.query.is_empty() && session.matches_entry(&record.entry) {
                    row_style(selected).bold()
                } else {
                    row_style(selected)
                },
            )
        })
        .collect();
    if items.is_empty() {
        frame.render_widget(
            Paragraph::new(i18n::tr!(HistoryEmpty)).style(Style::default().fg(MUTED)),
            body[1],
        );
    } else {
        frame.render_stateful_widget(List::new(items), content_area, &mut session.state);
        crate::scrollbar::vertical(
            frame,
            parts[0],
            content_area,
            session.records.len(),
            session.state.offset(),
        );
        hits.list(
            parts[0],
            content_area,
            &(0..session.records.len()).collect::<Vec<_>>(),
            session.state.offset(),
            crate::pointer::Target::History,
        );
        if session.selection.active() {
            for (row, index) in (session.state.offset()..session.records.len())
                .take(content_area.height as usize)
                .enumerate()
            {
                hits.button(
                    Rect::new(
                        content_area.x,
                        content_area.y + row as u16,
                        checkbox_width,
                        1,
                    ),
                    crate::pointer::Action::ToggleCommit(index),
                );
            }
        }
        match metrics {
            Ok(metrics) => pixels::render(
                frame,
                graph_area,
                session,
                graphics,
                metrics,
                graph_width,
                graphics_status,
            ),
            Err(error) => {
                frame.render_widget(Clear, graphics_status);
                frame.render_widget(
                    Paragraph::new(error).style(Style::default().fg(WARN)),
                    graphics_status,
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
