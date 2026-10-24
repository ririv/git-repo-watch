//! Two explicit groups retain their headings even in compact repository panes.
use super::*;
pub(super) fn render(
    frame: &mut Frame,
    area: Rect,
    session: &mut FileSession,
    title: &str,
    layout: crate::files::Layout,
    hits: &mut crate::pointer::Map,
) {
    let border = block(title.to_owned());
    let inner = border.inner(area);
    frame.render_widget(border, area);
    let mut offsets = session.offsets;
    let rows = session.rows();
    let projected = [false, true].map(|staged| {
        let changes = rows
            .iter()
            .copied()
            .enumerate()
            .filter(|(_, change)| (change.area == Area::Staged) == staged)
            .collect::<Vec<_>>();
        crate::files::tree::project(&changes, layout)
    });
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
                let crate::files::tree::Row::File {
                    index,
                    prefix,
                    label,
                } = entry
                else {
                    let crate::files::tree::Row::Directory { label, .. } = entry else {
                        unreachable!()
                    };
                    return ListItem::new(Line::styled(
                        format!("    {}", safe(label)),
                        Style::default().fg(ACCENT),
                    ));
                };
                let change = rows[*index];
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!(
                            "[{}] ",
                            if session.selected.contains(&session.change_index(change)) {
                                "✓"
                            } else {
                                " "
                            }
                        ),
                        Style::default().fg(MUTED),
                    ),
                    Span::styled(prefix.clone(), Style::default().fg(MUTED)),
                    Span::styled(
                        format!("{} ", change.kind),
                        Style::default().fg(if staged { GOOD } else { WARN }),
                    ),
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
                ]))
            })
            .collect::<Vec<_>>();
        let mut state = ListState::default()
            .with_offset(offsets[usize::from(staged)])
            .with_selected(entries.iter().position(|entry| matches!(entry, crate::files::tree::Row::File {index, ..} if *index == session.cursor)));
        frame.render_stateful_widget(
            List::new(items).highlight_style(selected_style()),
            parts[1],
            &mut state,
        );
        offsets[usize::from(staged)] = state.offset();
        crate::scrollbar::vertical(frame, area, parts[1], entries.len(), state.offset());
        hits.list_headings(
            area,
            parts[1],
            &entries
                .iter()
                .map(|entry| match entry {
                    crate::files::tree::Row::File { index, .. } => {
                        crate::pointer::ListRow::Item(*index)
                    }
                    crate::files::tree::Row::Directory { following, .. } => {
                        crate::pointer::ListRow::Heading {
                            following: *following,
                        }
                    }
                })
                .collect::<Vec<_>>(),
            state.offset(),
            crate::pointer::Target::Files,
        );
        // 复选框单独命中，点击文件名仍只定位；复选框不参与双击进入。
        for (row, entry) in entries
            .iter()
            .skip(state.offset())
            .take(parts[1].height as usize)
            .enumerate()
        {
            let crate::files::tree::Row::File { index, .. } = entry else {
                continue;
            };
            let checkbox =
                Rect::new(parts[1].x, parts[1].y + row as u16, 3, 1).intersection(parts[1]);
            hits.file_checkbox(checkbox, *index);
        }
    }
    session.offsets = offsets;
}
