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

/// Repaint a changing prefix boundary even if its blank separator matches the cached frame.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) enum Surface {
    #[default]
    Plain,
    Label(ratatui::layout::Rect),
}
impl Surface {
    pub fn update(&mut self, frame: &mut ratatui::Frame, next: Self) {
        if *self == next {
            return;
        }
        let previous = std::mem::replace(self, next);
        // 标签删除或缩短时，其后的空格在缓存里可能一直是默认背景，增量绘制会跳过它。
        // 把旧标签及分隔空格一起刷回当前样式；新标签的边界也显式写出，避免终端留下色块。
        for surface in [&previous, &*self] {
            if let Self::Label(area) = surface {
                let area = area.intersection(frame.area());
                for y in area.y..area.bottom() {
                    for x in area.x..area.right() {
                        frame.buffer_mut()[(x, y)]
                            .set_diff_option(ratatui::buffer::CellDiffOption::AlwaysUpdate);
                    }
                }
            }
        }
    }
}
