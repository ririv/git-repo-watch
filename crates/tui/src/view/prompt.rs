//! Full-width editable generation instructions, separate from repository context.
use super::*;
pub(super) fn render(
    frame: &mut Frame,
    area: Rect,
    session: &mut crate::prompt_editor::Session,
    hits: &mut crate::pointer::Map,
) {
    let panel = block(i18n::tr!(AgentPrompt)).padding(ratatui::widgets::Padding::horizontal(1));
    let inner = panel.inner(area);
    frame.render_widget(panel, area);
    if inner.is_empty() {
        return;
    }
    let document = session.view(inner.width.into(), inner.height.into());
    let count = document.lines.len();
    frame.render_widget(
        Paragraph::new(
            document
                .lines
                .into_iter()
                .map(|line| Line::from(safe(&line)))
                .collect::<Vec<_>>(),
        )
        .scroll((session.scroll.min(u16::MAX as usize) as u16, 0)),
        inner,
    );
    if document.cursor.1 >= session.scroll
        && document.cursor.1 < session.scroll + usize::from(inner.height)
    {
        frame.set_cursor_position((
            inner.x
                + document
                    .cursor
                    .0
                    .min(usize::from(inner.width.saturating_sub(1))) as u16,
            inner.y + (document.cursor.1 - session.scroll) as u16,
        ));
    }
    hits.vertical(
        area,
        inner,
        count.saturating_sub(inner.height.into()),
        session.scroll,
        crate::pointer::ScrollTarget::Text,
    );
    crate::scrollbar::vertical(frame, area, inner, count, session.scroll);
}
