//! Directory candidates share the full-width panel above the persistent input.
use super::*;

pub(super) fn render(frame: &mut Frame, directories: &command::path::Directories, body: Rect) {
    let items = directories
        .names()
        .iter()
        .map(|name| {
            ListItem::new(Line::from(Span::styled(
                truncate(&format!("{name}/"), body.width.saturating_sub(4) as usize),
                Style::default().fg(ACCENT),
            )))
        })
        .collect::<Vec<_>>();
    let height = (items.len().min(8) + 2).min(body.height.saturating_sub(3) as usize) as u16;
    let area = Rect::new(
        body.x,
        body.bottom().saturating_sub(height),
        body.width,
        height,
    );
    let mut state = ListState::default().with_selected(Some(directories.selected()));
    frame.render_widget(Clear, area);
    frame.render_stateful_widget(
        List::new(items)
            .block(block(i18n::tr!(DirectoryCandidates)))
            .highlight_style(selected_style())
            .highlight_symbol("› "),
        area,
        &mut state,
    );
}
