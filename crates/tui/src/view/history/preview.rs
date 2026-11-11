//! Metadata and file-level changes share the selected commit without loading patch bodies.
use super::*;
use crate::history::{
    GraphFocus,
    preview::{Content, Key, Preview},
};
use crate::view::metadata::{field, time};
use engine::local::history::Kind;

pub(super) fn render(
    frame: &mut Frame,
    area: Rect,
    session: &Session,
    preview: &mut Preview,
    icons: crate::file_icons::Display,
    hits: &mut crate::pointer::Map,
) {
    let Some(entry) = session.selected() else {
        return;
    };
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
    let refs = labels(&session.references, entry);
    if !refs.is_empty() {
        lines.push(Line::from(refs));
        lines.push(Line::default());
    }
    lines.push(Line::raw(safe(&entry.subject)));
    let metadata = Paragraph::new(lines).wrap(Wrap { trim: false });
    let height = (metadata.line_count(area.width.saturating_sub(2)) as u16 + 2)
        .min(area.height / 2)
        .max(3);
    let parts = Layout::vertical([Constraint::Length(height), Constraint::Min(4)]).split(area);
    frame.render_widget(
        metadata.block(block(entry.oid.to_string()[..12].to_owned())),
        parts[0],
    );
    preview.select(
        Key {
            path: session.references.path.clone(),
            oid: entry.oid,
        },
        parts[1],
    );
    let title = match &preview.content {
        Content::Ready { changes, .. } => {
            format!("{} · {}", i18n::tr!(HistoryChanges), changes.len())
        }
        _ => i18n::tr!(HistoryChanges).into(),
    };
    let panel = block(title);
    let inner = panel.inner(parts[1]);
    frame.render_widget(panel, parts[1]);
    hits.button(parts[1], crate::pointer::Action::HistoryFiles);
    match &preview.content {
        Content::Ready { changes, .. } if changes.is_empty() => {
            frame.render_widget(
                Paragraph::new(i18n::tr!(HistoryNoChanges)).style(Style::default().fg(MUTED)),
                inner,
            );
        }
        Content::Ready { changes, .. } => {
            let rows: Vec<_> = changes
                .iter()
                .map(|change| {
                    let color = match change.kind {
                        Kind::Added => GOOD,
                        Kind::Deleted => BAD,
                        Kind::Modified | Kind::Typechange => WARN,
                        Kind::Renamed | Kind::Copied => ACCENT,
                    };
                    let mut spans = icons.file(change.location.name());
                    spans.push(Span::raw(safe(&change.location.display())));
                    crate::view::files::marked_line(
                        &Line::from(spans),
                        change.kind.symbol(),
                        color,
                        inner.width as usize,
                    )
                })
                .collect();
            preview.maximum = rows.len().saturating_sub(inner.height as usize);
            preview.offset = preview.offset.min(preview.maximum);
            frame.render_widget(
                Paragraph::new(rows.into_iter().skip(preview.offset).collect::<Vec<_>>()),
                inner,
            );
            crate::scrollbar::vertical(frame, parts[1], inner, changes.len(), preview.offset);
            hits.vertical(
                parts[1],
                inner,
                preview.maximum,
                preview.offset,
                crate::pointer::ScrollTarget::HistoryFiles,
            );
        }
        Content::Failed { error, .. } => {
            frame.render_widget(
                Paragraph::new(multiline(error))
                    .style(Style::default().fg(BAD))
                    .wrap(Wrap { trim: false }),
                inner,
            );
        }
        Content::Loading(_) if preview.feedback.visible() => {
            frame.render_widget(
                Paragraph::new(i18n::tr!(HistoryReadCommit)).style(Style::default().fg(MUTED)),
                inner,
            );
        }
        Content::Hidden | Content::Loading(_) => {}
    }
    if session.focus == GraphFocus::Files {
        highlight_border(frame, parts[1]);
    }
}
