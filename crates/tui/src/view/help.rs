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
    let paragraph = Paragraph::new(crate::markdown::render(i18n::tr!(TuiHelp), inner.width))
        .wrap(Wrap { trim: false });
    let lines = paragraph.line_count(inner.width);
    let max_scroll = lines.saturating_sub(inner.height as usize);
    *scroll = (*scroll).min(max_scroll);
    frame.render_widget(
        paragraph
            .scroll(((*scroll).min(u16::MAX as usize) as u16, 0))
            .block(panel),
        area,
    );
    crate::scrollbar::vertical(frame, area, inner, lines, *scroll);
    hits.vertical(
        area,
        inner,
        max_scroll,
        *scroll,
        crate::pointer::ScrollTarget::Text,
    );
}
