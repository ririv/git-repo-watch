//! Render the cached styled paragraph and measure wrapping only when width changes.
use super::*;
pub(super) fn render(
    frame: &mut Frame,
    area: Rect,
    content: &mut crate::diff::View,
    scroll: &mut usize,
) {
    let border = block(i18n::tr!(FileDiff));
    let inner = border.inner(area);
    let count = content.line_count(inner.width);
    *scroll = (*scroll).min(count.saturating_sub(inner.height as usize));
    content.paragraph =
        std::mem::take(&mut content.paragraph).scroll(((*scroll).min(u16::MAX as usize) as u16, 0));
    frame.render_widget(border, area);
    frame.render_widget(&content.paragraph, inner);
}
