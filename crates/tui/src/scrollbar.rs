//! Thin scroll indicators share geometry and styling across both axes.
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState},
};

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
            .thumb_style(Style::default().fg(Color::LightBlue)),
        area,
        &mut state,
    );
}

/// Align a right-border scrollbar with content rows, excluding fixed headers.
pub(crate) fn vertical(frame: &mut Frame, panel: Rect, content: Rect, rows: usize, offset: usize) {
    if panel.width < 2 || panel.height < 3 || content.height == 0 {
        return;
    }
    let start = content.y.max(panel.y + 1);
    let end = content.bottom().min(panel.bottom() - 1);
    let height = end.saturating_sub(start);
    render(
        frame,
        Rect::new(panel.right() - 1, start, 1, height),
        Axis::Vertical,
        offset,
        rows.saturating_sub(usize::from(height)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend, widgets::Block};

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
