//! Shared multiline text and cursor rendering; callers own borders and scroll targets.
use super::*;
pub(super) fn render(
    frame: &mut Frame,
    inner: Rect,
    editor: &mut crate::multiline_editor::Editor,
    placeholder: &str,
    cursor: bool,
) -> usize {
    if inner.is_empty() {
        return 0;
    }
    let document = editor.view(inner.width.into(), inner.height.into());
    let count = document.lines.len();
    let text = if editor.text().is_empty() {
        Text::from(Span::styled(
            placeholder.to_owned(),
            Style::default().fg(MUTED),
        ))
    } else {
        Text::from(
            document
                .lines
                .into_iter()
                .map(|line| Line::from(safe(&line)))
                .collect::<Vec<_>>(),
        )
    };
    frame.render_widget(
        Paragraph::new(text).scroll((editor.scroll.min(u16::MAX as usize) as u16, 0)),
        inner,
    );
    if cursor
        && document.cursor.1 >= editor.scroll
        && document.cursor.1 < editor.scroll + usize::from(inner.height)
    {
        frame.set_cursor_position((
            inner.x
                + document
                    .cursor
                    .0
                    .min(usize::from(inner.width.saturating_sub(1))) as u16,
            inner.y + (document.cursor.1 - editor.scroll) as u16,
        ));
    }
    count
}
