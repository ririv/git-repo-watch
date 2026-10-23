//! Two explicit groups retain their headings even in compact repository panes.
use super::*;
pub(super) fn render(frame: &mut Frame, area: Rect, session: &FileSession, title: &str) {
    let border = block(title.to_owned());
    let inner = border.inner(area);
    frame.render_widget(border, area);
    let rows = session.rows();
    let unstaged = rows
        .iter()
        .filter(|change| change.area != Area::Staged)
        .count();
    let natural = rows.len().saturating_add(2);
    let first_height = if natural <= usize::from(inner.height) {
        (unstaged + 1) as u16
    } else {
        inner.height.div_ceil(2)
    };
    let groups =
        Layout::vertical([Constraint::Length(first_height), Constraint::Min(0)]).split(inner);
    for (group, staged) in groups.iter().zip([false, true]) {
        let (indices, changes): (Vec<_>, Vec<_>) = rows
            .iter()
            .copied()
            .enumerate()
            .filter(|(_, change)| (change.area == Area::Staged) == staged)
            .unzip();
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
                    changes.len()
                ),
                Style::default().fg(ACCENT).bold(),
            ),
            parts[0],
        );
        let items = changes
            .iter()
            .map(|change| {
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
                    Span::styled(
                        format!("{} ", change.kind),
                        Style::default().fg(if staged { GOOD } else { WARN }),
                    ),
                    Span::raw(safe(&change.location.display())),
                ]))
            })
            .collect::<Vec<_>>();
        let mut state = ListState::default()
            .with_selected(indices.iter().position(|index| *index == session.cursor));
        frame.render_stateful_widget(
            List::new(items)
                .highlight_style(selected_style())
                .highlight_spacing(ratatui::widgets::HighlightSpacing::Always)
                .highlight_symbol("› "),
            parts[1],
            &mut state,
        );
        crate::scrollbar::vertical(frame, area, parts[1], changes.len(), state.offset());
    }
}
