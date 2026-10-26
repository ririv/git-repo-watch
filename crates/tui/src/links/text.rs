//! Styled lines retain explicit link ranges through prefixing, clipping and wrapping.
use engine::desktop::Target;
use ratatui::{
    style::Style,
    text::{Line as PlainLine, Span, Text as PlainText},
};
use std::{ops::Range, sync::Arc};
use unicode_width::UnicodeWidthStr;
#[derive(Debug, Clone)]
pub(crate) struct Anchor {
    pub columns: Range<usize>,
    pub target: Arc<Target>,
}
#[derive(Debug, Default, Clone)]
pub(crate) struct Line {
    pub text: PlainLine<'static>,
    pub anchors: Vec<Anchor>,
}
impl From<PlainLine<'static>> for Line {
    fn from(text: PlainLine<'static>) -> Self {
        Self {
            text,
            anchors: Vec::new(),
        }
    }
}
impl Line {
    pub fn width(&self) -> usize {
        self.text.width()
    }
    pub fn prepend(&mut self, prefix: Vec<Span<'static>>) {
        let width = prefix.iter().map(Span::width).sum::<usize>();
        for anchor in &mut self.anchors {
            anchor.columns.start += width;
            anchor.columns.end += width;
        }
        let mut spans = prefix;
        spans.append(&mut self.text.spans);
        self.text.spans = spans;
    }
    pub fn append(&mut self, mut other: Self) {
        let width = self.width();
        for anchor in &mut other.anchors {
            anchor.columns.start += width;
            anchor.columns.end += width;
        }
        self.text.spans.extend(other.text.spans);
        self.anchors.extend(other.anchors);
    }
    pub fn detect(&mut self, paths: &[std::path::PathBuf]) {
        let text = self.text.to_string();
        for (range, target) in super::detect::references(&text, paths) {
            self.add_bytes(&text, range, Arc::new(target));
        }
    }
    fn add_bytes(&mut self, text: &str, bytes: Range<usize>, target: Arc<Target>) {
        let columns = text[..bytes.start].width()..text[..bytes.end].width();
        if !self
            .anchors
            .iter()
            .any(|old| old.columns.start < columns.end && columns.start < old.columns.end)
        {
            self.anchors.push(Anchor { columns, target });
        }
    }
    /// Wrap only the body, retaining the first gutter and blank continuation gutters.
    pub fn wrap_pinned(&self, width: usize, fixed: usize) -> Vec<Self> {
        let fixed = fixed.min(width);
        if width == fixed {
            return vec![self.slice(0, width)];
        }
        let prefix = self.slice(0, fixed);
        self.slice(fixed, self.width())
            .wrap(width - fixed, false)
            .into_iter()
            .enumerate()
            .map(|(index, mut line)| {
                if index == 0 {
                    let mut first = prefix.clone();
                    first.append(line);
                    first
                } else {
                    line.prepend(vec![Span::styled(" ".repeat(fixed), self.text.style)]);
                    line
                }
            })
            .collect()
    }
    fn slice(&self, start: usize, width: usize) -> Self {
        self.wrapped_line(
            crate::horizontal::clip(&self.text, start, width),
            start,
            start + width,
        )
    }
    pub fn wrap(&self, width: usize, words: bool) -> Vec<Self> {
        let mut result = Vec::new();
        let mut pending: Vec<(String, Style, usize)> = Vec::new();
        let mut used = 0;
        let mut start = 0;
        for grapheme in self.text.styled_graphemes(Style::default()) {
            let size = grapheme.symbol.width();
            if used + size > width && !pending.is_empty() {
                let split = if words {
                    pending
                        .iter()
                        .rposition(|(text, _, _)| text.chars().all(char::is_whitespace))
                        .filter(|index| *index > 0)
                        .map_or(pending.len(), |index| index + 1)
                } else {
                    pending.len()
                };
                let count = pending[..split]
                    .iter()
                    .map(|(_, _, size)| *size)
                    .sum::<usize>();
                let text = PlainLine::from(
                    pending
                        .drain(..split)
                        .map(|(text, style, _)| Span::styled(text, style))
                        .collect::<Vec<_>>(),
                );
                result.push(self.wrapped_line(text, start, start + count));
                start += count;
                used -= count;
                if used + size > width && !pending.is_empty() {
                    let text = PlainLine::from(
                        pending
                            .drain(..)
                            .map(|(text, style, _)| Span::styled(text, style))
                            .collect::<Vec<_>>(),
                    );
                    result.push(self.wrapped_line(text, start, start + used));
                    start += used;
                    used = 0;
                }
            }
            if size > width {
                result.push(Line::from(PlainLine::raw(" ")));
                start += size;
            } else {
                pending.push((grapheme.symbol.to_owned(), grapheme.style, size));
                used += size;
            }
        }
        let text = PlainLine::from(
            pending
                .into_iter()
                .map(|(text, style, _)| Span::styled(text, style))
                .collect::<Vec<_>>(),
        );
        result.push(self.wrapped_line(text, start, start + used));
        result
    }
    fn wrapped_line(&self, text: PlainLine<'static>, start: usize, end: usize) -> Self {
        Self {
            text: text.style(self.text.style),
            anchors: self
                .anchors
                .iter()
                .filter_map(|anchor| {
                    let from = anchor.columns.start.max(start);
                    let to = anchor.columns.end.min(end);
                    (from < to).then(|| Anchor {
                        columns: from - start..to - start,
                        target: anchor.target.clone(),
                    })
                })
                .collect(),
        }
    }
}
#[derive(Debug, Default, Clone)]
pub(crate) struct Text {
    pub lines: Vec<Line>,
}
impl Text {
    pub fn new(text: PlainText<'static>, paths: &[std::path::PathBuf]) -> Self {
        let mut lines = text.lines.into_iter().map(Line::from).collect::<Vec<_>>();
        let mut paths = paths.to_vec();
        paths.sort_by_key(|path| std::cmp::Reverse(path.as_os_str().len()));
        for line in &mut lines {
            line.detect(&paths);
        }
        Self { lines }
    }
    pub fn plain(&self) -> PlainText<'static> {
        PlainText::from(
            self.lines
                .iter()
                .map(|line| line.text.clone())
                .collect::<Vec<_>>(),
        )
    }
    pub fn width(&self) -> usize {
        self.lines.iter().map(Line::width).max().unwrap_or(0)
    }
}
impl std::fmt::Display for Text {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.plain())
    }
}

impl std::fmt::Display for Line {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.text)
    }
}
