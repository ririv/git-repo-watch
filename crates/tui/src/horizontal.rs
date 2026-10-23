//! Cell-based horizontal viewports shared by styled text views.
use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
};
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Default, Clone)]
pub(crate) struct Viewport {
    offset: usize,
    maximum: usize,
}
impl Viewport {
    pub fn offset(&self) -> usize {
        self.offset
    }
    #[cfg(test)]
    pub fn overflowing(&self) -> bool {
        self.maximum > 0
    }
    pub fn constrain(&mut self, maximum: usize) {
        self.maximum = maximum;
        self.offset = self.offset.min(maximum);
    }
    pub fn seek(&mut self, offset: usize) {
        self.offset = offset.min(self.maximum);
    }
    pub fn advance(&mut self, delta: isize) {
        self.offset = self.offset.saturating_add_signed(delta).min(self.maximum);
    }
    /// Draw only under the scrolling columns, leaving fixed columns and corners intact.
    pub fn render_on_border(
        &self,
        frame: &mut Frame,
        panel: Rect,
        columns: std::ops::Range<u16>,
        hits: &mut crate::pointer::Map,
        target: crate::pointer::ScrollTarget,
    ) {
        if panel.width > 2 && panel.height > 1 {
            let start = columns.start.max(panel.x + 1);
            let end = columns.end.min(panel.right() - 1);
            hits.track(
                Rect::new(start, panel.bottom() - 1, end.saturating_sub(start), 1),
                crate::scrollbar::Axis::Horizontal,
                self.maximum,
                target,
            );
            self.render(
                frame,
                Rect::new(start, panel.bottom() - 1, end.saturating_sub(start), 1),
            );
        }
    }
    fn render(&self, frame: &mut Frame, area: Rect) {
        crate::scrollbar::render(
            frame,
            area,
            crate::scrollbar::Axis::Horizontal,
            self.offset,
            self.maximum,
        );
    }
}

/// Clip in terminal cells, preserving graphemes and their styles.
pub(crate) fn clip(line: &Line<'_>, offset: usize, width: usize) -> Line<'static> {
    let end = offset.saturating_add(width);
    let mut column = 0;
    let mut spans: Vec<Span<'static>> = Vec::new();
    for grapheme in line.styled_graphemes(Style::default()) {
        let size = UnicodeWidthStr::width(grapheme.symbol);
        let next = column + size;
        if column >= end {
            break;
        }
        if next > offset {
            // 宽字符只显示一部分时，用等宽空格占位，避免切出半个字符或错位。
            let text = if column < offset || next > end {
                " ".repeat(next.min(end) - column.max(offset))
            } else {
                grapheme.symbol.to_owned()
            };
            if let Some(last) = spans.last_mut()
                && last.style == grapheme.style
            {
                last.content.to_mut().push_str(&text);
            } else {
                spans.push(Span::styled(text, grapheme.style));
            }
        }
        column = next;
    }
    Line::from(spans).style(line.style)
}

/// Keep the gutter visible while scrolling only the body of a styled line.
pub(crate) fn pinned(line: &Line<'_>, fixed: usize, offset: usize, width: usize) -> Line<'static> {
    let fixed = fixed.min(width);
    let mut result = clip(line, 0, fixed);
    result
        .spans
        .extend(clip(line, fixed + offset, width - fixed).spans);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    #[test]
    fn clips_wide_and_combining_graphemes_without_losing_styles() {
        let blue = Style::default().fg(Color::Blue);
        let green = Style::default().fg(Color::Green);
        let line = Line::from(vec![
            Span::styled("中e\u{301}👩‍💻", blue),
            Span::styled("尾end", green),
        ]);
        assert_eq!(clip(&line, 1, 5).to_string(), " e\u{301}👩‍💻 ");
        assert_eq!(clip(&line, 2, 8).to_string(), "e\u{301}👩‍💻尾end");
        assert_eq!(clip(&line, 6, 4).to_string(), " end");
        assert_eq!(clip(&line, 6, 4).spans[0].style, green);
        for offset in 0..12 {
            for width in 0..12 {
                assert!(clip(&line, offset, width).width() <= width);
            }
        }
        assert_eq!(clip(&line, 2, 1).spans[0].style, blue);
    }

    #[test]
    fn pinned_gutters_stay_styled_while_content_scrolls_and_bounds_shrink() {
        let line = Line::from(vec![
            Span::styled("12 │+", Style::default().fg(Color::Red)),
            Span::styled("abc中文TAIL", Style::default().fg(Color::Green)),
        ])
        .style(Style::default().bg(Color::Blue));
        let slice = pinned(&line, 5, 7, 9);
        assert_eq!(slice.to_string(), "12 │+TAIL");
        assert_eq!(slice.style.bg, Some(Color::Blue));
        assert_eq!(slice.spans[0].style.fg, Some(Color::Red));
        assert_eq!(slice.spans.last().unwrap().style.fg, Some(Color::Green));
        let mut viewport = Viewport::default();
        viewport.constrain(12);
        viewport.advance(20);
        assert_eq!(viewport.offset(), 12);
        viewport.constrain(3);
        assert_eq!(viewport.offset(), 3);
        viewport.advance(-4);
        assert_eq!(viewport.offset(), 0);
        viewport.constrain(0);
        assert!(!viewport.overflowing());
    }
    #[test]
    fn scrollbar_reaches_both_ends_of_the_visible_track() {
        use ratatui::{Terminal, backend::TestBackend};
        let mut terminal = Terminal::new(TestBackend::new(30, 1)).unwrap();
        let mut viewport = Viewport::default();
        viewport.constrain(200);
        terminal
            .draw(|frame| viewport.render(frame, frame.area()))
            .unwrap();
        assert_eq!(terminal.backend().buffer()[(0, 0)].symbol(), "━");
        assert_ne!(terminal.backend().buffer()[(29, 0)].symbol(), "━");
        viewport.advance(200);
        terminal
            .draw(|frame| viewport.render(frame, frame.area()))
            .unwrap();
        assert_eq!(terminal.backend().buffer()[(29, 0)].symbol(), "━");
        assert_ne!(terminal.backend().buffer()[(0, 0)].symbol(), "━");
    }
}
