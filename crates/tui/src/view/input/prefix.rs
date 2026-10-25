//! Keep normal prefix styling outside the selected command segments.
use super::{Color, INPUT_ACCENT, Span, Style};
use std::ops::Range;

pub(super) fn spans(text: &str, selection: Option<Range<usize>>) -> Vec<Span<'static>> {
    let normal = Style::default()
        .fg(Color::LightCyan)
        .bg(Color::Rgb(34, 52, 65))
        .bold();
    let selected = Style::default().fg(Color::Black).bg(INPUT_ACCENT).bold();
    // 标签前有一格内边距；按字符划分，窄窗口截断为 … 时也不会切开 UTF-8。
    text.char_indices()
        .map(|(index, character)| {
            let active = index > 0
                && selection
                    .as_ref()
                    .is_some_and(|range| range.contains(&(index - 1)));
            Span::styled(
                character.to_string(),
                if active { selected } else { normal },
            )
        })
        .collect()
}
