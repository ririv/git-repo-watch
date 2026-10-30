//! Search emphasis composes with syntax and status colors without replacing foregrounds.
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, Copy)]
pub(crate) enum Emphasis {
    Current,
    Other,
}
impl Emphasis {
    pub fn at(index: usize, selected: Option<usize>) -> Self {
        if selected == Some(index) {
            Self::Current
        } else {
            Self::Other
        }
    }
    pub fn style(self) -> Style {
        let style = Style::default()
            .remove_modifier(Modifier::DIM)
            .add_modifier(Modifier::UNDERLINED);
        match self {
            Self::Current => style
                .bg(Color::Rgb(124, 83, 24))
                .underline_color(Color::Rgb(255, 211, 122))
                .add_modifier(Modifier::BOLD),
            Self::Other => style
                .bg(Color::Rgb(81, 65, 37))
                .underline_color(Color::Rgb(210, 164, 92)),
        }
    }
}

/// Highlight before horizontal clipping so partially visible matches remain marked.
pub(crate) fn line(line: Line<'static>, query: &str, emphasis: Emphasis) -> Line<'static> {
    if query.is_empty() {
        return line;
    }
    let ranges = super::literal::ranges(&line.to_string(), &query.to_lowercase());
    if ranges.is_empty() {
        return line;
    }
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut column = 0;
    let mut hit = 0;
    for grapheme in line.styled_graphemes(Style::default()) {
        let end = column + grapheme.symbol.width();
        while hit < ranges.len() && ranges[hit].end <= column {
            hit += 1;
        }
        let style = if hit < ranges.len() && ranges[hit].start < end {
            grapheme.style.patch(emphasis.style())
        } else {
            grapheme.style
        };
        if let Some(last) = spans.last_mut()
            && last.style == style
        {
            last.content.to_mut().push_str(grapheme.symbol);
        } else {
            spans.push(Span::styled(grapheme.symbol.to_owned(), style));
        }
        column = end;
    }
    Line { spans, ..line }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_cross_style_boundaries_and_clip_without_changing_unicode_or_colors() {
        let source = Line::from(vec![
            Span::styled("前中", Style::default().fg(Color::LightBlue)),
            Span::styled(
                "İ e\u{301}尾",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::DIM),
            ),
        ]);
        let marked = line(source.clone(), "中i", Emphasis::Current);
        assert_eq!(marked.to_string(), source.to_string());
        let clipped = crate::horizontal::clip(&marked, 3, 3);
        assert_eq!(clipped.to_string(), " İ ");
        assert_eq!(clipped.spans[0].style.fg, Some(Color::LightBlue));
        assert_eq!(clipped.spans[1].style.fg, Some(Color::Green));
        assert_eq!(clipped.spans[0].style.bg, Emphasis::Current.style().bg);
        assert_eq!(clipped.spans[1].style.bg, Emphasis::Current.style().bg);
        assert!(!clipped.spans[1].style.add_modifier.contains(Modifier::DIM));
        assert_eq!(clipped.spans[2].style.bg, None);
        let combining = line(source.clone(), "e\u{301}", Emphasis::Other);
        assert!(
            combining
                .spans
                .iter()
                .any(|span| span.content == "e\u{301}"
                    && span.style.bg == Emphasis::Other.style().bg)
        );
        assert_eq!(line(source.clone(), "missing", Emphasis::Other), source);
    }
}
