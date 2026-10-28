//! Staged and unstaged trees share selectable rows and tri-state directory checkboxes.
use super::*;
use crate::files::{CheckState, Group};
pub(super) fn render(
    frame: &mut Frame,
    area: Rect,
    session: &mut FileSession,
    title: &str,
    layout: crate::files::Layout,
    hits: &mut crate::pointer::Map,
) {
    session.set_layout(layout);
    let border = block(title.to_owned());
    let mut inner = border.inner(area);
    frame.render_widget(border, area);
    if let engine::operations::replay::State::Active(active) = &session.snapshot.operation.state {
        let parts = Layout::vertical([Constraint::Length(2), Constraint::Min(0)]).split(inner);
        frame.render_widget(
            Paragraph::new(vec![
                Line::styled(
                    format!(
                        "{} · {}",
                        active.kind.label(),
                        &active.commit.to_string()[..12]
                    ),
                    Style::default().fg(WARN).bold(),
                ),
                Line::styled(
                    "/repo continue · /repo skip · /repo abort",
                    Style::default().fg(ACCENT),
                ),
            ]),
            parts[0],
        );
        inner = parts[1];
    }
    let mut offsets = session.offsets;
    let rows = session.rows();
    let projected = session.entries(layout);
    let unstaged = rows
        .iter()
        .filter(|change| change.area != Area::Staged)
        .count();
    let natural = projected
        .iter()
        .map(Vec::len)
        .sum::<usize>()
        .saturating_add(2);
    let first_height = if natural <= usize::from(inner.height) {
        (projected[0].len() + 1) as u16
    } else {
        inner.height.div_ceil(2)
    };
    let groups =
        Layout::vertical([Constraint::Length(first_height), Constraint::Min(0)]).split(inner);
    let mut row_offset = 0;
    for (group, staged) in groups.iter().zip([false, true]) {
        let entries = &projected[usize::from(staged)];
        let count = if staged {
            rows.len() - unstaged
        } else {
            unstaged
        };
        let parts = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(*group);
        frame.render_widget(
            Line::styled(
                format!(
                    "{} · {}",
                    if staged {
                        i18n::tr!(ChangesStaged)
                    } else {
                        i18n::tr!(ChangesUnstaged)
                    },
                    count
                ),
                Style::default().fg(ACCENT).bold(),
            ),
            parts[0],
        );
        let items = entries
            .iter()
            .map(|entry| {
                let mut spans = checkbox(session.check_state(entry, &rows));
                let crate::files::tree::Row::File {
                    index,
                    prefix,
                    label,
                } = entry
                else {
                    let crate::files::tree::Row::Directory { label, .. } = entry else {
                        unreachable!()
                    };
                    spans.push(Span::styled(safe(label), Style::default().fg(ACCENT)));
                    return ListItem::new(Line::from(spans));
                };
                let change = rows[*index];
                spans.extend([
                    Span::styled(prefix.clone(), Style::default().fg(MUTED)),
                    Span::styled(
                        safe(label),
                        if session.search_mode == crate::search::Mode::Locate
                            && !session.query.is_empty()
                            && session.matches_change(change)
                        {
                            Style::default().bold()
                        } else {
                            Style::default()
                        },
                    ),
                ]);
                let line = Line::from(spans);
                ListItem::new(marked_line(
                    &line,
                    change.kind,
                    if change.kind == 'U' {
                        BAD
                    } else if staged {
                        GOOD
                    } else {
                        WARN
                    },
                    parts[1].width as usize,
                ))
            })
            .collect::<Vec<_>>();
        let mut state = ListState::default()
            .with_offset(offsets[usize::from(staged)])
            .with_selected(entries.iter().position(|entry| {
                entry.cursor(Group::from_index(usize::from(staged))) == session.cursor
            }));
        frame.render_stateful_widget(
            List::new(items).highlight_style(selected_style()),
            parts[1],
            &mut state,
        );
        offsets[usize::from(staged)] = state.offset();
        crate::scrollbar::vertical(frame, area, parts[1], entries.len(), state.offset());
        hits.list(
            area,
            parts[1],
            &(row_offset..row_offset + entries.len()).collect::<Vec<_>>(),
            state.offset(),
            crate::pointer::Target::Files,
        );
        // 复选框单独命中，点击文件名仍只定位；复选框不参与双击进入。
        for (row, _) in entries
            .iter()
            .skip(state.offset())
            .take(parts[1].height as usize)
            .enumerate()
        {
            let checkbox =
                Rect::new(parts[1].x, parts[1].y + row as u16, 3, 1).intersection(parts[1]);
            hits.file_checkbox(checkbox, row_offset + state.offset() + row);
        }
        row_offset += entries.len();
    }
    session.offsets = offsets;
}

fn checkbox(state: CheckState) -> Vec<Span<'static>> {
    let mark = match state {
        CheckState::Unchecked => Span::raw(" "),
        CheckState::Mixed => Span::styled("−", Style::default().fg(WARN).bold()),
        CheckState::Checked => {
            Span::styled("✓", Style::default().fg(Color::Rgb(73, 199, 140)).bold())
        }
    };
    vec![
        Span::styled("[", Style::default().fg(MUTED)),
        mark,
        Span::styled("] ", Style::default().fg(MUTED)),
    ]
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
