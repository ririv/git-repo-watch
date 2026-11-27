//! Discovery review shares list geometry with repository management and pointer hit regions.
use super::*;
use crate::management::{Selection, Session, Source, Status};
use crate::pointer::{Action as PointerAction, Target};

pub(super) fn render(
    frame: &mut Frame,
    area: Rect,
    session: &mut Session,
    hits: &mut crate::pointer::Map,
) {
    let (title, header_height, detail_height) = match session.source {
        Source::Managed => (
            format!("{} · {}", session.title(), session.entries.len()),
            0,
            0,
        ),
        Source::Scan { .. } => (session.title().to_owned(), 1, 2),
    };
    let border = block(title);
    let inner = border.inner(area);
    frame.render_widget(border, area);
    let parts = Layout::vertical([
        Constraint::Length(header_height),
        Constraint::Length(header_height),
        Constraint::Min(0),
        Constraint::Length(detail_height),
    ])
    .split(inner);
    if let Source::Scan { root, excluded } = &session.source {
        let heading = safe(&root.to_string_lossy());
        frame.render_widget(
            Paragraph::new(super::breadcrumbs::shorten(
                &heading,
                usize::from(inner.width),
            ))
            .style(Style::default().fg(ACCENT)),
            parts[0],
        );
        let summary = format!(
            "{} {} · {} {} · {} {} · {} {}",
            i18n::tr!(ScanAddSelected),
            session.selection().len(),
            i18n::tr!(ScanRegistered),
            session
                .entries
                .iter()
                .filter(|entry| matches!(entry.status, Status::Managed))
                .count(),
            i18n::tr!(ScanFailed),
            session
                .entries
                .iter()
                .filter(|entry| matches!(entry.status, Status::Failed(_)))
                .count(),
            i18n::tr!(ScanTemporaryExclusions),
            excluded.len(),
        );
        frame.render_widget(
            Paragraph::new(summary).style(Style::default().fg(MUTED)),
            parts[1],
        );
    }
    let rows = session.rows();
    let lines = rows
        .iter()
        .map(|&index| {
            let entry = &session.entries[index];
            let (prefix, label, color) = match &entry.status {
                Status::New(Selection::Included) => ("[✓] ", i18n::tr!(ScanNew), Color::Reset),
                Status::New(Selection::Skipped) => ("[ ] ", i18n::tr!(ScanNew), Color::Reset),
                Status::Managed => match session.source {
                    Source::Managed => ("", "", ACCENT),
                    Source::Scan { .. } => ("    ", i18n::tr!(ScanRegistered), ACCENT),
                },
                Status::Failed(_) => (" ×  ", i18n::tr!(ScanFailed), BAD),
            };
            let display = match &session.source {
                Source::Managed => safe(&entry.path.to_string_lossy()),
                Source::Scan { root, .. } => {
                    let relative = entry
                        .path
                        .strip_prefix(root)
                        .expect("discovered repository belongs to scan root");
                    if relative.as_os_str().is_empty() {
                        ".".into()
                    } else {
                        safe(&relative.to_string_lossy())
                    }
                }
            };
            let mut spans = match entry.status {
                Status::New(selection) => {
                    let mut spans = super::checkbox::spans(match selection {
                        Selection::Included => crate::files::CheckState::Checked,
                        Selection::Skipped => crate::files::CheckState::Unchecked,
                    });
                    spans.push(Span::raw(" "));
                    spans
                }
                _ => vec![Span::styled(prefix, Style::default().fg(color))],
            };
            spans.push(Span::styled(display, Style::default().fg(color)));
            let status = (!label.is_empty()).then(|| {
                Span::styled(
                    format!("  {label}"),
                    Style::default().fg(if matches!(entry.status, Status::Failed(_)) {
                        BAD
                    } else {
                        MUTED
                    }),
                )
            });
            (Line::from(spans), status)
        })
        .collect::<Vec<_>>();
    let leading = if matches!(session.source, Source::Scan { .. }) {
        4
    } else {
        0
    };
    let trailing = lines
        .iter()
        .filter_map(|(_, status)| status.as_ref().map(Span::width))
        .max()
        .unwrap_or(0);
    let widest = lines
        .iter()
        .map(|(line, _)| line.width().saturating_sub(leading))
        .max()
        .unwrap_or(0);
    let columns = crate::horizontal::Columns::new(parts[2], leading, trailing).scroll(
        frame,
        area,
        hits,
        crate::horizontal::Pane::Management,
        &mut session.horizontal,
        widest,
    );
    let selection = row_selection(session.state.selected(), lines.len());
    let items = lines
        .into_iter()
        .enumerate()
        .map(|(index, (label, status))| {
            ListItem::new(columns.line(&label, status)).style(row_style(selection == Some(index)))
        });
    frame.render_stateful_widget(List::new(items), parts[2], &mut session.state);
    hits.list(
        area,
        parts[2],
        &(0..rows.len()).collect::<Vec<_>>(),
        session.state.offset(),
        Target::Management,
    );
    for (line, &index) in rows
        .iter()
        .skip(session.state.offset())
        .take(parts[2].height as usize)
        .enumerate()
    {
        if matches!(session.entries[index].status, Status::New(_)) {
            hits.button(
                Rect::new(
                    parts[2].x,
                    parts[2].y + line as u16,
                    parts[2].width.min(3),
                    1,
                ),
                PointerAction::ManagementToggle(line + session.state.offset()),
            );
        }
    }
    crate::scrollbar::vertical(frame, area, parts[2], rows.len(), session.state.offset());
    if matches!(session.source, Source::Scan { .. })
        && let Some(entry) = session.selected()
    {
        let text = match &entry.status {
            Status::Failed(error) => safe(error),
            _ => safe(&entry.path.to_string_lossy()),
        };
        frame.render_widget(
            Paragraph::new(text)
                .wrap(Wrap { trim: false })
                .style(
                    Style::default().fg(if matches!(entry.status, Status::Failed(_)) {
                        BAD
                    } else {
                        MUTED
                    }),
                ),
            parts[3],
        );
    }
}
