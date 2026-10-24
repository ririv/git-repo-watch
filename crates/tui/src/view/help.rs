//! Styled quick reference with explicit headings, inline tokens and scroll position.
use super::*;

pub(super) fn render(
    frame: &mut Frame,
    area: Rect,
    scroll: &mut usize,
    hits: &mut crate::pointer::Map,
) {
    let panel = block(i18n::tr!(Instructions)).padding(ratatui::widgets::Padding::horizontal(1));
    let inner = panel.inner(area);
    let text = crate::markdown::linked(i18n::tr!(TuiHelp), inner.width, None);
    let lines = Paragraph::new(text.plain())
        .wrap(Wrap { trim: false })
        .line_count(inner.width);
    let max_scroll = lines.saturating_sub(inner.height as usize);
    *scroll = (*scroll).min(max_scroll);
    frame.render_widget(panel, area);
    hits.links.paragraph(frame, inner, &text, *scroll);
    crate::scrollbar::vertical(frame, area, inner, lines, *scroll);
    hits.vertical(
        area,
        inner,
        max_scroll,
        *scroll,
        crate::pointer::ScrollTarget::Text,
    );
}
