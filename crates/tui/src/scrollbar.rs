//! Thin scroll indicators share geometry and styling across both axes.
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState},
};

pub(crate) const THUMB_COLOR: Color = Color::Rgb(230, 160, 189);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Axis {
    Horizontal,
    Vertical,
}

pub(crate) fn render(frame: &mut Frame, area: Rect, axis: Axis, offset: usize, maximum: usize) {
    if maximum == 0 || area.is_empty() {
        return;
    }
    let (orientation, track, thumb, visible) = match axis {
        Axis::Horizontal => (ScrollbarOrientation::HorizontalBottom, "─", "━", area.width),
        Axis::Vertical => (ScrollbarOrientation::VerticalRight, "│", "┃", area.height),
    };
    let mut state = ScrollbarState::new(maximum + 1)
        .viewport_content_length(usize::from(visible))
        .position(offset.min(maximum));
    frame.render_stateful_widget(
        Scrollbar::new(orientation)
            .begin_symbol(None)
            .end_symbol(None)
            .track_symbol(Some(track))
            .track_style(
                Style::default()
                    .fg(Color::Rgb(65, 75, 85))
                    .add_modifier(Modifier::DIM),
            )
            .thumb_symbol(thumb)
            .thumb_style(Style::default().fg(THUMB_COLOR)),
        area,
        &mut state,
    );
}

/// Align a right-border scrollbar with content rows, excluding fixed headers.
pub(crate) fn vertical(frame: &mut Frame, panel: Rect, content: Rect, rows: usize, offset: usize) {
    if panel.width < 2 || panel.height < 3 || content.height == 0 {
        return;
    }
    let area = vertical_area(panel, content);
    render(
        frame,
        area,
        Axis::Vertical,
        offset,
        rows.saturating_sub(usize::from(area.height)),
    );
}

pub(crate) fn vertical_area(panel: Rect, content: Rect) -> Rect {
    if panel.width < 2 || panel.height < 3 || content.is_empty() {
        return Rect::default();
    }
    let start = content.y.max(panel.y + 1);
    let end = content.bottom().min(panel.bottom() - 1);
    Rect::new(panel.right() - 1, start, 1, end.saturating_sub(start))
}

/// Thumb cell range matching Ratatui's arrowless scrollbar geometry.
pub(crate) fn thumb(length: usize, offset: usize, maximum: usize) -> std::ops::Range<usize> {
    if length == 0 {
        return 0..0;
    }
    let total = maximum.saturating_add(length);
    let round = |n: usize| n.saturating_add(total / 2) / total;
    let size = round(length.saturating_mul(length)).clamp(1, length);
    let start = round(offset.min(maximum).saturating_mul(length)).min(length - size);
    start..start + size
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend, widgets::Block};

    #[test]
    fn thumb_hit_geometry_matches_rendered_cells_on_both_axes() {
        for length in [1, 2, 5, 20] {
            for maximum in [1, 5, 100] {
                for offset in [0, maximum / 2, maximum] {
                    for axis in [Axis::Horizontal, Axis::Vertical] {
                        let area = match axis {
                            Axis::Horizontal => Rect::new(0, 0, length, 1),
                            Axis::Vertical => Rect::new(0, 0, 1, length),
                        };
                        let mut terminal =
                            Terminal::new(TestBackend::new(area.width, area.height)).unwrap();
                        terminal
                            .draw(|frame| render(frame, area, axis, offset, maximum))
                            .unwrap();
                        for cell in 0..length {
                            let position = match axis {
                                Axis::Horizontal => (cell, 0),
                                Axis::Vertical => (0, cell),
                            };
                            assert_eq!(
                                terminal.backend().buffer()[position].fg == THUMB_COLOR,
                                thumb(length as usize, offset, maximum).contains(&(cell as usize))
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn vertical_track_excludes_headers_and_corners_and_hides_when_content_fits() {
        let mut terminal = Terminal::new(TestBackend::new(16, 12)).unwrap();
        let panel = Rect::new(2, 1, 12, 9);
        let content = Rect::new(3, 4, 10, 5);
        for (rows, offset) in [(30, 0), (30, 25), (5, 0)] {
            terminal
                .draw(|frame| {
                    frame.render_widget(Block::bordered(), panel);
                    vertical(frame, panel, content, rows, offset);
                })
                .unwrap();
            let buffer = terminal.backend().buffer();
            assert_eq!(buffer[(13, 1)].symbol(), "┐");
            assert_eq!(buffer[(13, 9)].symbol(), "┘");
            assert_eq!(buffer[(13, 3)].symbol(), "│");
            assert_eq!(buffer[(12, 4)].symbol(), " ");
            match (rows, offset) {
                (30, 0) => {
                    assert_eq!(buffer[(13, 4)].symbol(), "┃");
                    assert_ne!(buffer[(13, 8)].symbol(), "┃");
                }
                (30, 25) => {
                    assert_ne!(buffer[(13, 4)].symbol(), "┃");
                    assert_eq!(buffer[(13, 8)].symbol(), "┃");
                }
                _ => assert!(!buffer.content.iter().any(|cell| cell.symbol() == "┃")),
            }
        }
    }
}
