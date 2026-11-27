//! Metadata and file-level changes share the selected commit without loading patch bodies.
use super::*;
use crate::history::{
    GraphFocus,
    preview::{Content, Key, Preview},
};
use crate::horizontal::Pane;
use crate::view::metadata::{field, time};

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
            let labels: Vec<_> = changes
                .iter()
                .map(|change| crate::view::files::history_label(change, icons))
                .collect();
            let widest = labels.iter().map(Line::width).max().unwrap_or(0);
            let paths = crate::horizontal::Columns::new(inner, 0, 2).scroll(
                frame,
                parts[1],
                hits,
                Pane::HistoryPreview,
                &mut preview.file_paths,
                widest,
            );
            preview.maximum = labels.len().saturating_sub(usize::from(inner.height));
            preview.offset = preview.offset.min(preview.maximum);
            let rows: Vec<_> = labels
                .iter()
                .zip(changes)
                .skip(preview.offset)
                .take(usize::from(inner.height))
                .map(|(label, change)| {
                    paths.line(label, Some(crate::view::files::history_status(change.kind)))
                })
                .collect();
            frame.render_widget(Paragraph::new(rows), inner);
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
