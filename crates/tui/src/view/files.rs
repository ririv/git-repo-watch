//! Staged and unstaged trees share one viewport, selectable rows and directory checkboxes.
use super::*;
use crate::files::{Group, list::Row};
use crate::search::highlight::{self, Emphasis};
use unicode_width::UnicodeWidthStr;

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
    let list = session.list(options.layout);
    let rows = &list.rows;
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
    let selecting = session.selected.active();
    let pinned = if selecting {
        usize::from((super::checkbox::WIDTH + 1).min(inner.width))
    } else {
        0
    };
    let path_width = usize::from(inner.width).saturating_sub(pinned + 2);
    let widest = list.widest(options.icons, |row| {
        label_width(session, row, options.icons)
    });
    session
        .horizontal
        .constrain(widest.saturating_sub(path_width));

    let selected = list.position(&session.cursor);
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
    let visible = list_window::visible(inner, rows.len(), &mut state);
    list_window::render(frame, inner, visible.clone(), |index| match &rows[index] {
        Row::Header { group, count } => ListItem::new(Line::styled(
            format!("{} · {count}", group.label()),
            Style::default().fg(ACCENT).bold(),
        )),
        Row::Entry { group, entry, .. } => {
            let (line, marker) = label(
                session,
                entry,
                *group,
                options.icons,
                selected == Some(index),
            );
            let mut shown = crate::horizontal::clip(&line, 0, pinned);
            shown.spans.extend(
                crate::horizontal::clip(&line, pinned + session.horizontal.offset(), path_width)
                    .spans,
            );
            if let Some((marker, color)) = marker {
                shown = marked_line(&shown, marker, color, usize::from(inner.width));
            }
            ListItem::new(shown).style(row_style(selected == Some(index)))
        }
    });
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
    for (visible, row) in rows[visible].iter().enumerate() {
        let Row::Entry { index, .. } = row else {
            continue;
        };
        let line = Rect::new(inner.x, inner.y + visible as u16, inner.width, 1);
        hits.item(line, crate::pointer::Target::Files, *index);
        if selecting {
            let checkbox = Rect::new(line.x, line.y, super::checkbox::WIDTH, 1);
            hits.file_checkbox(checkbox.intersection(line), *index);
        }
    }
}

fn label_width(session: &FileSession, row: &Row, icons: crate::file_icons::Display) -> usize {
    use crate::files::tree::Row as File;
    let Row::Entry { entry, .. } = row else {
        return 0;
    };
    match entry {
        File::Directory {
            prefix,
            label,
            expansion,
            ..
        } => {
            prefix.width()
                + safe(label).width()
                + directory_icon(icons, *expansion)
                    .iter()
                    .map(Span::width)
                    .sum::<usize>()
        }
        File::File {
            index,
            prefix,
            label,
        } => {
            let change = session.change(*index).expect("visible change");
            prefix.width()
                + path_line(&change.location, label).width()
                + icons
                    .file(change.location.name())
                    .iter()
                    .map(Span::width)
                    .sum::<usize>()
        }
    }
}
fn directory_icon(
    icons: crate::file_icons::Display,
    expansion: crate::files::Expansion,
) -> Vec<Span<'static>> {
    icons.directory_slot(match expansion {
        crate::files::Expansion::Expanded => crate::file_icons::Folder::Expanded,
        crate::files::Expansion::Collapsed => crate::file_icons::Folder::Collapsed,
    })
}
fn label(
    session: &FileSession,
    entry: &crate::files::tree::Row,
    group: Group,
    icons: crate::file_icons::Display,
    selected: bool,
) -> (Line<'static>, Option<(char, Color)>) {
    let emphasis = if selected {
        Emphasis::Current
    } else {
        Emphasis::Other
    };
    let mut spans = Vec::new();
    if session.selected.active() {
        spans.extend(super::checkbox::spans(session.check_state(entry, group)));
        spans.push(Span::raw(" "));
    }
    let (path, marker) = match entry {
        crate::files::tree::Row::Directory {
            prefix,
            label,
            expansion,
            ..
        } => {
            spans.push(Span::styled(prefix.clone(), Style::default().fg(MUTED)));
            spans.extend(directory_icon(icons, *expansion));
            (Line::styled(safe(label), Style::default().fg(ACCENT)), None)
        }
        crate::files::tree::Row::File {
            index,
            prefix,
            label,
        } => {
            let change = session.change(*index).expect("visible change");
            spans.push(Span::styled(prefix.clone(), Style::default().fg(MUTED)));
            spans.extend(icons.file(change.location.name()));
            let color = if change.kind == 'U' {
                BAD
            } else if group == Group::Staged {
                GOOD
            } else {
                WARN
            };
            (
                path_line(&change.location, label),
                Some((change.kind, color)),
            )
        }
    };
    spans.extend(highlight::line(path, &session.query, emphasis).spans);
    (Line::from(spans), marker)
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
