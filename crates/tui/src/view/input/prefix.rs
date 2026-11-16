//! Style command prefixes without padding outside the selected segments.
use super::{Color, INPUT_ACCENT, Span, Style, syntax};
use crate::{appearance::Background, command::syntax::Role};
use std::ops::Range;

pub(super) fn spans(
    text: &str,
    selection: Option<Range<usize>>,
    background: Background,
) -> Vec<Span<'static>> {
    let normal = syntax::style(Role::Command).bg(match background {
        Background::Light => Color::Reset,
        Background::Dark | Background::Unreported => Color::Rgb(34, 52, 65),
    });
    let selected = Style::default().fg(Color::Black).bg(INPUT_ACCENT).bold();
    // 按字符划分，窄窗口截断为 … 时也不会切开 UTF-8。
    text.char_indices()
        .map(|(index, character)| {
            let active = selection
                .as_ref()
                .is_some_and(|range| range.contains(&index));
            Span::styled(
                character.to_string(),
                if active { selected } else { normal },
            )
        })
        .collect()
}
