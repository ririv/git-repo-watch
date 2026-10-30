//! Staged and unstaged trees share selectable rows and tri-state directory checkboxes.
use super::*;
use crate::files::Group;
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
    let mut inner = border.inner(area);
    frame.render_widget(border, area);
    if let engine::operations::replay::State::Active(active) = &session.snapshot.operation.state {
        let parts = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).split(inner);
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
                    format!(
                        "{}: {} · {}: {}",
                        i18n::tr!(ReplayConflictsShort),
                        active.conflicts.len(),
                        i18n::tr!(ReplayRemainingShort),
                        active.remaining().len()
                    ),
                    Style::default().fg(if active.conflicts.is_empty() {
                        GOOD
                    } else {
                        BAD
                    }),
                ),
            ]),
            parts[0],
        );
        let buttons = Layout::horizontal([
            Constraint::Percentage(34),
            Constraint::Percentage(33),
            Constraint::Percentage(33),
        ])
        .split(Rect::new(
            parts[0].x,
            parts[0].y.saturating_add(2),
            parts[0].width,
            parts[0].height.saturating_sub(2).min(1),
        ));
        for (button, control) in buttons.iter().zip([
            engine::operations::replay::Control::Continue,
            engine::operations::replay::Control::Skip,
            engine::operations::replay::Control::Abort,
        ]) {
            let enabled = control != engine::operations::replay::Control::Continue
                || active.conflicts.is_empty();
            let label = match control {
                engine::operations::replay::Control::Continue => i18n::tr!(ReplayContinueShort),
                engine::operations::replay::Control::Skip => i18n::tr!(ReplaySkipShort),
                engine::operations::replay::Control::Abort => i18n::tr!(ReplayAbortShort),
            };
            frame.render_widget(
                Paragraph::new(format!("[{label}]")).style(Style::default().fg(if enabled {
                    ACCENT
                } else {
                    MUTED
                })),
                *button,
            );
            if enabled {
                hits.button(*button, crate::pointer::Action::ReplayControl(control));
            }
        }
        inner = parts[1];
    }
    let mut offsets = session.offsets;
    let rows = session.rows();
    let projected = session.entries(options.layout);
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
                let selected =
                    entry.cursor(Group::from_index(usize::from(staged))) == session.cursor;
                let emphasis = if selected {
                    Emphasis::Current
                } else {
                    Emphasis::Other
                };
                let mut spans = super::checkbox::spans(session.check_state(entry, &rows));
                spans.push(Span::raw(" "));
                let crate::files::tree::Row::File {
                    index,
                    prefix,
                    label,
                } = entry
                else {
                    let crate::files::tree::Row::Directory { prefix, label, .. } = entry else {
                        unreachable!()
                    };
                    spans.push(Span::styled(prefix.clone(), Style::default().fg(MUTED)));
                    spans.extend(options.icons.directory());
                    spans.extend(
                        highlight::line(
                            Line::styled(safe(label), Style::default().fg(ACCENT)),
                            &session.query,
                            emphasis,
                        )
                        .spans,
                    );
                    return ListItem::new(Line::from(spans)).style(row_style(selected));
                };
                let change = rows[*index];
                spans.push(Span::styled(prefix.clone(), Style::default().fg(MUTED)));
                spans.extend(options.icons.file(change.location.name()));
                spans.extend(
                    highlight::line(Line::raw(safe(label)), &session.query, emphasis).spans,
                );
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
                .style(row_style(selected))
            })
            .collect::<Vec<_>>();
        let mut state = ListState::default()
            .with_offset(offsets[usize::from(staged)])
            .with_selected(entries.iter().position(|entry| {
                entry.cursor(Group::from_index(usize::from(staged))) == session.cursor
            }));
        frame.render_stateful_widget(List::new(items), parts[1], &mut state);
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
