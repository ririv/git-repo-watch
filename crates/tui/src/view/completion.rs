//! Directory candidates share the full-width panel above the persistent input.
use super::*;

pub(super) fn render(frame: &mut Frame, directories: &command::path::Directories, body: Rect) {
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
    render_candidates(
        frame,
        items,
        directories.selected(),
        i18n::tr!(DirectoryCandidates),
        body,
    );
}

pub(super) fn render_values(frame: &mut Frame, values: &command::values::Candidates, body: Rect) {
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
    render_candidates(
        frame,
        items,
        values.selected(),
        i18n::tr!(ArgumentCandidates),
        body,
    );
}

fn render_candidates(
    frame: &mut Frame,
    items: Vec<ListItem<'static>>,
    selected: usize,
    title: &'static str,
    body: Rect,
) {
    let count = items.len();
    let available = body.height.saturating_sub(u16::from(body.height >= 9));
    let height = (items.len().min(8) + 2).min(usize::from(available)) as u16;
    let area = Rect::new(
        body.x,
        body.bottom().saturating_sub(height),
        body.width,
        height,
    );
    let mut state = ListState::default().with_selected(Some(selected));
    let border = if height >= 3 {
        block(title)
    } else {
        Block::default()
    };
    let inner = border.inner(area);
    frame.render_widget(Clear, area);
    frame.render_stateful_widget(
        List::new(items)
            .block(border)
            .highlight_style(selected_style()),
        area,
        &mut state,
    );
    crate::scrollbar::vertical(frame, area, inner, count, state.offset());
}
