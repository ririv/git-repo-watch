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
    let count = items.len();
    let available = body.height.saturating_sub(u16::from(body.height >= 9));
    let height = (items.len().min(8) + 2).min(usize::from(available)) as u16;
    let area = Rect::new(
        body.x,
        body.bottom().saturating_sub(height),
        body.width,
        height,
    );
    let mut state = ListState::default().with_selected(Some(directories.selected()));
    let border = if height >= 3 {
        block(i18n::tr!(DirectoryCandidates))
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
