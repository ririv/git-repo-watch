//! Markdown layout runs in the file worker and preserves styled graphemes.
use engine::cancel::Cancel;
use ratatui::{
    style::Style,
    text::{Line, Span, Text},
};
use unicode_width::UnicodeWidthStr;

pub(super) fn prepare(source: &str, width: u16, cancel: &Cancel) -> anyhow::Result<Text<'static>> {
    cancel.check()?;
    let text = crate::markdown::render(source, width);
    let width = usize::from(width.max(1));
    let mut lines = Vec::new();
    for line in text.lines {
        cancel.check()?;
        let mut pending: Vec<(String, Style, usize)> = Vec::new();
        let mut columns = 0;
        for grapheme in line.styled_graphemes(Style::default()) {
            cancel.check()?;
            let size = grapheme.symbol.width();
            if columns + size > width && !pending.is_empty() {
                let split = pending
                    .iter()
                    .rposition(|(text, _, _)| text.chars().all(char::is_whitespace))
                    .filter(|index| *index > 0)
                    .map_or(pending.len(), |index| index + 1);
                lines.push(Line::from(
                    pending
                        .drain(..split)
                        .map(|(text, style, _)| Span::styled(text, style))
                        .collect::<Vec<_>>(),
                ));
                columns = pending.iter().map(|(_, _, width)| width).sum();
                if columns + size > width && !pending.is_empty() {
                    lines.push(Line::from(
                        pending
                            .drain(..)
                            .map(|(text, style, _)| Span::styled(text, style))
                            .collect::<Vec<_>>(),
                    ));
                    columns = 0;
                }
            }
            let symbol = if size > width {
                " ".to_owned()
            } else {
                grapheme.symbol.to_owned()
            };
            let size = size.min(width);
            pending.push((symbol, grapheme.style, size));
            columns += size;
        }
        lines.push(Line::from(
            pending
                .into_iter()
                .map(|(text, style, _)| Span::styled(text, style))
                .collect::<Vec<_>>(),
        ));
    }
    Ok(Text::from(lines))
}
