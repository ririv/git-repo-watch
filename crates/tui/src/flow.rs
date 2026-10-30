//! Cached visual rows retain logical anchors across wrapping and width changes.
use crate::links::{Line, Text};
use std::ops::Range;
use std::sync::Arc;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Wrapping {
    Off,
    Cells,
    Words,
}
#[derive(Debug, Clone, Default)]
pub(crate) struct Cache(Option<Arc<Layout>>);
#[derive(Debug)]
pub(crate) struct Row {
    pub line: Line,
    pub source: usize,
    pub columns: Range<usize>,
}
#[derive(Debug)]
pub(crate) struct Layout {
    width: usize,
    wrap: Wrapping,
    pub rows: Vec<Row>,
}
impl Cache {
    pub fn is_current(&self, width: usize, wrap: Wrapping) -> bool {
        self.0
            .as_ref()
            .is_some_and(|layout| layout.width == width && layout.wrap == wrap)
    }
    pub fn arrange(
        &mut self,
        text: &Text,
        width: usize,
        fixed: usize,
        wrap: Wrapping,
        scroll: &mut usize,
    ) -> &Layout {
        if self
            .0
            .as_ref()
            .is_none_or(|layout| layout.width != width || layout.wrap != wrap)
        {
            let anchor = self
                .0
                .as_ref()
                .and_then(|layout| layout.rows.get(*scroll))
                .map(|row| row.source);
            let rows = text
                .lines
                .iter()
                .enumerate()
                .flat_map(|(source, line)| {
                    let lines = match wrap {
                        Wrapping::Off => vec![line.clone()],
                        Wrapping::Cells => line.wrap_pinned(width, fixed),
                        Wrapping::Words if width == 0 => line.wrap_pinned(0, 0),
                        Wrapping::Words => line.wrap(width, true),
                    };
                    let mut column = fixed;
                    let available = width.saturating_sub(fixed);
                    let mut glyphs = line
                        .text
                        .styled_graphemes(ratatui::style::Style::default())
                        .scan(0, |column, glyph| {
                            use unicode_width::UnicodeWidthStr;
                            let start = *column;
                            let size = glyph.symbol.width();
                            *column += size;
                            Some((start, size))
                        })
                        .filter(|(start, _)| *start >= fixed)
                        .map(|(_, size)| size)
                        .peekable();
                    lines
                        .into_iter()
                        .map(|line| {
                            let cells = line.width().saturating_sub(fixed.min(width));
                            let consume = if wrap != Wrapping::Off
                                && cells > 0
                                && available > 0
                                && glyphs.peek().is_some_and(|size| *size > available)
                            {
                                *glyphs.peek().expect("oversized grapheme")
                            } else {
                                cells
                            };
                            let mut consumed = 0;
                            while consumed < consume {
                                let Some(size) = glyphs.next() else {
                                    break;
                                };
                                consumed += size;
                            }
                            let columns = column..column + consumed;
                            column += consumed;
                            Row {
                                source,
                                line,
                                columns,
                            }
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            if let Some(anchor) = anchor {
                *scroll = rows
                    .iter()
                    .position(|row| row.source == anchor)
                    .expect("every text line has a visual row");
            }
            self.0 = Some(Arc::new(Layout { width, wrap, rows }));
        }
        self.0.as_ref().expect("text layout prepared")
    }
}
impl Layout {
    pub fn row_at(&self, line: usize, column: usize) -> Option<usize> {
        self.rows
            .iter()
            .position(|row| row.source == line && row.columns.contains(&column))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{
        style::{Color, Style},
        text::{Line as PlainLine, Span},
    };
    #[test]
    fn wrapped_gutters_preserve_unicode_styles_whitespace_and_complete_links() {
        let body = "中文e\u{301}👩‍💻  https://example.invalid/long/target/end";
        let style = Style::default().fg(Color::Green).bg(Color::Blue);
        let mut line = Line::from(
            PlainLine::from(vec![Span::raw("12 │"), Span::styled(body, style)])
                .style(Style::default().bg(Color::Blue)),
        );
        line.detect(&[]);
        let parts = line.wrap_pinned(15, 4);
        assert!(parts.len() > 3);
        assert!(parts.iter().all(|line| line.width() <= 15));
        assert!(parts[0].to_string().starts_with("12 │"));
        assert!(
            parts[1..]
                .iter()
                .all(|line| line.to_string().starts_with("    "))
        );
        let joined = parts
            .iter()
            .map(|line| line.text.to_string().chars().skip(4).collect::<String>())
            .collect::<String>();
        assert_eq!(joined, body);
        assert!(
            parts
                .iter()
                .all(|line| line.text.style.bg == Some(Color::Blue))
        );
        assert!(parts.iter().flat_map(|line| &line.anchors).count() > 1);
        assert!(
            parts
                .iter()
                .flat_map(|line| &line.anchors)
                .all(|a| a.target.uri() == "https://example.invalid/long/target/end")
        );
        assert!(
            parts
                .iter()
                .flat_map(|line| &line.text.spans)
                .filter(|span| span.content.contains('中') || span.content.contains('👩'))
                .all(|span| span.style.fg == Some(Color::Green))
        );
        for width in 0..7 {
            assert!(
                line.wrap_pinned(width, 4)
                    .iter()
                    .all(|row| row.width() <= width)
            );
        }
    }
    #[test]
    fn cached_reflow_reuses_rows_and_preserves_the_source_line_across_modes() {
        let text = Text::new(
            ratatui::text::Text::from(vec![
                PlainLine::raw("first ".repeat(30)),
                PlainLine::raw("second ".repeat(30)),
                PlainLine::raw("third"),
            ]),
            &[],
        );
        let mut cache = Cache::default();
        let mut scroll = 1;
        cache.arrange(&text, 20, 0, Wrapping::Off, &mut scroll);
        let original = cache.0.clone().unwrap();
        cache.arrange(&text, 20, 0, Wrapping::Off, &mut scroll);
        assert!(Arc::ptr_eq(&original, cache.0.as_ref().unwrap()));
        for width in [20, 9, 31, 20] {
            let layout = cache.arrange(&text, width, 0, Wrapping::Cells, &mut scroll);
            assert_eq!(layout.rows[scroll].source, 1);
        }
        cache.arrange(&text, 20, 0, Wrapping::Off, &mut scroll);
        assert_eq!(scroll, 1);
    }
}
