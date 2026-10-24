//! Directory candidates share the full-width panel above the persistent input.
use super::*;

pub(super) fn render(
    frame: &mut Frame,
    screen: &Screen,
    directories: &command::path::Directories,
    body: Rect,
) {
    let items = directories
        .names()
        .iter()
        .map(|name| {
            ListItem::new(Line::from(Span::styled(
                truncate(&format!("{name}/"), body.width.saturating_sub(2) as usize),
                Style::default().fg(ACCENT),
            )))
        })
        .collect::<Vec<_>>();
    render_candidates(frame, screen, items, directories.selected(), body);
}

pub(super) fn render_values(
    frame: &mut Frame,
    screen: &Screen,
    values: &command::values::Candidates,
    body: Rect,
) {
    let items = values
        .agents()
        .iter()
        .map(|agent| {
            ListItem::new(Line::from(vec![
                Span::styled(agent.name(), Style::default().fg(ACCENT)),
                Span::styled(
                    if *agent == values.current {
                        format!("  ✓ {}", i18n::tr!(LayoutCurrent))
                    } else {
                        String::new()
                    },
                    Style::default().fg(GOOD),
                ),
            ]))
        })
        .collect();
    render_candidates(frame, screen, items, values.selected(), body);
}

fn render_candidates(
    frame: &mut Frame,
    screen: &Screen,
    items: Vec<ListItem<'static>>,
    selected: usize,
    body: Rect,
) {
    let count = items.len();
    let (area, inner) = candidates::panel(frame, screen, body, count);
    let mut state = ListState::default().with_selected(Some(selected));
    frame.render_stateful_widget(
        List::new(items).highlight_style(selected_style()),
        inner,
        &mut state,
    );
    crate::scrollbar::vertical(frame, area, inner, count, state.offset());
}
