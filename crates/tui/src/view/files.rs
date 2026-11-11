//! Staged and unstaged trees share one viewport, selectable rows and directory checkboxes.
use super::*;
use crate::files::{Group, list::Row};
use crate::search::highlight::{self, Emphasis};

enum Label {
    Header(Line<'static>),
    Entry {
        line: Line<'static>,
        marker: Option<(char, Color)>,
        selected: bool,
    },
}

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
        session.horizontal.constrain(0);
        frame.render_widget(
            Paragraph::new(match (session.has_changes(), session.scope()) {
                (true, _) => i18n::tr!(FileListNoMatches),
                (false, Some(Group::Staged)) => i18n::tr!(ChangesNoStaged),
                (false, Some(Group::Unstaged)) => i18n::tr!(ChangesNoUnstaged),
                (false, None) => i18n::tr!(FileListClean),
            })
            .style(Style::default().fg(MUTED)),
            inner,
        );
        return;
    }
    let labels = rows
        .iter()
        .map(|row| match row {
            Row::Header { group, count } => Label::Header(Line::styled(
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
                label(session, entry, *group, &changes, options.icons)
            }
        })
        .collect::<Vec<_>>();
    let pinned = usize::from((super::checkbox::WIDTH + 1).min(inner.width));
    let path_width = usize::from(inner.width).saturating_sub(pinned + 2);
    session.horizontal.constrain(
        labels
            .iter()
            .filter_map(|label| match label {
                Label::Entry { line, .. } => Some(line.width().saturating_sub(pinned + path_width)),
                Label::Header(_) => None,
            })
            .max()
            .unwrap_or(0),
    );
    let items = labels
        .into_iter()
        .map(|label| match label {
            Label::Header(line) => ListItem::new(line),
            Label::Entry {
                line,
                marker,
                selected,
            } => {
                let mut visible = crate::horizontal::clip(&line, 0, pinned);
                visible.spans.extend(
                    crate::horizontal::clip(
                        &line,
                        pinned + session.horizontal.offset(),
                        path_width,
                    )
                    .spans,
                );
                let visible = match marker {
                    Some((marker, color)) => {
                        marked_line(&visible, marker, color, usize::from(inner.width))
                    }
                    None => visible,
                };
                ListItem::new(visible).style(row_style(selected))
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
    session.horizontal.render_on_border(
        frame,
        area,
        inner.x + pinned as u16..inner.right().saturating_sub(2),
        hits,
        crate::pointer::ScrollTarget::WorktreeHorizontal,
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

fn label(
    session: &FileSession,
    entry: &crate::files::tree::Row,
    group: Group,
    changes: &[&Change],
    icons: crate::file_icons::Display,
) -> Label {
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
            Label::Entry {
                line: Line::from(spans),
                marker: None,
                selected,
            }
        }
        crate::files::tree::Row::File {
            index,
            prefix,
            label,
        } => {
            let change = changes[*index];
            spans.push(Span::styled(prefix.clone(), Style::default().fg(MUTED)));
            spans.extend(icons.file(change.location.name()));
            spans.extend(
                highlight::line(path_line(&change.location, label), &session.query, emphasis).spans,
            );
            Label::Entry {
                line: Line::from(spans),
                marker: Some((
                    change.kind,
                    if change.kind == 'U' {
                        BAD
                    } else if group == Group::Staged {
                        GOOD
                    } else {
                        WARN
                    },
                )),
                selected,
            }
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

/// Render rename separators as structure, without styling arrows inside real filenames.
pub(super) fn path_line(location: &engine::local::files::Location, label: &str) -> Line<'static> {
    use engine::local::files::Location;
    match location {
        Location::Path(_) => Line::raw(safe(label)),
        Location::Rename { before, after } | Location::Copy { before, after } => Line::from(vec![
            Span::raw(safe(&before.to_string_lossy())),
            Span::styled(" → ", Style::default().fg(ACCENT).bold()),
            Span::raw(safe(&after.to_string_lossy())),
        ]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rename_separator_has_its_own_style_and_literal_filename_arrows_are_unchanged() {
        use engine::local::files::Location;
        let path = Location::Rename {
            before: "old/目录/a.ts".into(),
            after: "new/目录/a.ts".into(),
        };
        let line = path_line(&path, &path.display());
        assert_eq!(line.to_string(), "old/目录/a.ts → new/目录/a.ts".to_owned());
        assert_eq!(line.spans[1].style.fg, Some(crate::view::ACCENT));
        assert!(
            line.spans[1]
                .style
                .add_modifier
                .contains(ratatui::style::Modifier::BOLD)
        );
        assert_eq!(line.spans[0].style.fg, None);
        assert_eq!(line.spans[2].style.fg, None);
        let name = "literal → arrow.ts";
        let literal = path_line(&Location::Path(name.into()), name);
        assert_eq!(literal.spans.len(), 1);
        assert_eq!(literal.spans[0].style, ratatui::style::Style::default());
    }
}
