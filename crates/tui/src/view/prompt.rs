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
    let count = super::text_editor::render(frame, inner, &mut session.editor, "", true);
    hits.vertical(
        area,
        inner,
        count.saturating_sub(inner.height.into()),
        session.editor.scroll,
        crate::pointer::ScrollTarget::Text,
    );
    crate::scrollbar::vertical(frame, area, inner, count, session.editor.scroll);
}
