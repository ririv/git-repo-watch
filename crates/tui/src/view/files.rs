//! Staged and unstaged trees share one viewport, selectable rows and directory checkboxes.
use super::*;
use crate::files::{Group, list::Row};
use crate::search::highlight::{self, Emphasis};

pub(super) fn render(
    frame: &mut Frame,
    area: Rect,
    session: &mut FileSession,
    title: &str,
    options: crate::files::Options,
    hits: &mut crate::pointer::Map,
) {
    session.set_layout(options.layout);
    let border = block(title.to_owned());
    let inner = border.inner(area);
    frame.render_widget(border, area);
    let changes = session.rows();
    let rows = session.list(options.layout);
    if rows.is_empty() {
        session.offset = 0;
        frame.render_widget(
            Paragraph::new(if session.snapshot.changes.is_empty() {
                i18n::tr!(FileListClean)
            } else {
                i18n::tr!(FileListNoMatches)
            })
            .style(Style::default().fg(MUTED)),
            inner,
        );
        return;
    }
    let items = rows
        .iter()
        .map(|row| match row {
            Row::Header { group, count } => ListItem::new(Line::styled(
                format!(
                    "{} · {}",
                    match group {
                        Group::Staged => i18n::tr!(ChangesStaged),
                        Group::Unstaged => i18n::tr!(ChangesUnstaged),
                    },
                    count
                ),
                Style::default().fg(ACCENT).bold(),
            )),
            Row::Entry { group, entry, .. } => {
                item(session, entry, *group, &changes, options.icons, inner.width)
            }
        })
        .collect::<Vec<_>>();
    let selected = rows
        .iter()
        .position(|row| row.cursor().as_ref() == Some(&session.cursor));
    let mut offset = session
        .offset
        .min(rows.len().saturating_sub(usize::from(inner.height)));
    if inner.height >= 2
        && let Some(index) = selected
        && index > 0
        && matches!(rows[index - 1], Row::Header { .. })
    {
        // 选中分组第一项时，同时露出它上方的标题。
        offset = offset.min(index - 1);
    }
    let mut state = ListState::default()
        .with_offset(offset)
        .with_selected(selected);
    frame.render_stateful_widget(List::new(items), inner, &mut state);
    session.offset = state.offset();
    crate::scrollbar::vertical(frame, area, inner, rows.len(), session.offset);
    hits.vertical(
        area,
        inner,
        rows.len().saturating_sub(usize::from(inner.height)),
        session.offset,
        crate::pointer::ScrollTarget::Files,
    );
    for (visible, row) in rows
        .iter()
        .skip(session.offset)
        .take(usize::from(inner.height))
        .enumerate()
    {
        let Row::Entry { index, .. } = row else {
            continue;
        };
        let line = Rect::new(inner.x, inner.y + visible as u16, inner.width, 1);
        hits.item(line, crate::pointer::Target::Files, *index);
        // 复选框单独命中，分组标题不参与选择或勾选。
        hits.file_checkbox(Rect::new(line.x, line.y, 3, 1).intersection(line), *index);
    }
}

fn item(
    session: &FileSession,
    entry: &crate::files::tree::Row,
    group: Group,
    changes: &[&Change],
    icons: crate::file_icons::Display,
    width: u16,
) -> ListItem<'static> {
    let selected = entry.cursor(group) == session.cursor;
    let emphasis = if selected {
        Emphasis::Current
    } else {
        Emphasis::Other
    };
    let mut spans = super::checkbox::spans(session.check_state(entry, changes));
    spans.push(Span::raw(" "));
    match entry {
        crate::files::tree::Row::Directory {
            prefix,
            label,
            expansion,
            ..
        } => {
            spans.push(Span::styled(prefix.clone(), Style::default().fg(MUTED)));
            spans.extend(icons.directory_slot(match expansion {
                crate::files::Expansion::Expanded => crate::file_icons::Folder::Expanded,
                crate::files::Expansion::Collapsed => crate::file_icons::Folder::Collapsed,
            }));
            spans.extend(
                highlight::line(
                    Line::styled(safe(label), Style::default().fg(ACCENT)),
                    &session.query,
                    emphasis,
                )
                .spans,
            );
            ListItem::new(Line::from(spans)).style(row_style(selected))
        }
        crate::files::tree::Row::File {
            index,
            prefix,
            label,
        } => {
            let change = changes[*index];
            spans.push(Span::styled(prefix.clone(), Style::default().fg(MUTED)));
            spans.extend(icons.file(change.location.name()));
            spans.extend(highlight::line(Line::raw(safe(label)), &session.query, emphasis).spans);
            ListItem::new(marked_line(
                &Line::from(spans),
                change.kind,
                if change.kind == 'U' {
                    BAD
                } else if group == Group::Staged {
                    GOOD
                } else {
                    WARN
                },
                usize::from(width),
            ))
            .style(row_style(selected))
        }
    }
}

/// Keep a file status marker pinned to the last column, clipping only the label.
pub(super) fn marked_line(
    label: &Line<'_>,
    marker: char,
    color: Color,
    width: usize,
) -> Line<'static> {
    let mut line = crate::horizontal::clip(label, 0, width.saturating_sub(2));
    if width > 0 {
        line.spans.push(Span::raw(
            " ".repeat(width.saturating_sub(line.width() + 1)),
        ));
        line.spans
            .push(Span::styled(marker.to_string(), Style::default().fg(color)));
    }
    line
}
