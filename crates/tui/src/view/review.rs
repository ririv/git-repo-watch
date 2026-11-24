use super::*;
pub(super) fn text_view(
    frame: &mut Frame,
    area: Rect,
    text: &str,
    scroll: &mut usize,
    title: &str,
    hits: &mut crate::pointer::Map,
) {
    styled_text_view(
        frame,
        area,
        Text::from(text.to_owned()),
        scroll,
        title,
        hits,
        crate::pointer::ScrollTarget::Text,
    );
}

pub(super) fn styled_text_view(
    frame: &mut Frame,
    area: Rect,
    text: Text<'static>,
    scroll: &mut usize,
    title: &str,
    hits: &mut crate::pointer::Map,
    target: crate::pointer::ScrollTarget,
) {
    let block = block(title.to_owned());
    let inner = block.inner(area);
    let text = hits.links.text(text);
    let paragraph = Paragraph::new(text.plain()).wrap(Wrap { trim: false });
    let lines = paragraph.line_count(inner.width);
    *scroll = (*scroll).min(lines.saturating_sub(inner.height as usize));
    frame.render_widget(block, area);
    hits.links.paragraph(frame, inner, &text, *scroll);
    crate::scrollbar::vertical(frame, area, inner, lines, *scroll);
    hits.vertical(
        area,
        inner,
        lines.saturating_sub(inner.height as usize),
        *scroll,
        target,
    );
}
