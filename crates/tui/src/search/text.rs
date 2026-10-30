//! Literal text matches retain terminal-cell ranges independently from visual wrapping.
use crate::links::Text;
use ratatui::style::Style;
use std::{ops::Range, sync::Arc};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Hit {
    pub line: usize,
    pub columns: Range<usize>,
}
#[derive(Debug, Clone, Copy, Default)]
enum Selection {
    #[default]
    None,
    Current {
        index: usize,
        reveal: Reveal,
    },
}
#[derive(Debug, Clone, Copy)]
enum Reveal {
    Requested,
    Visible,
}

/// Only the small draft is retained in per-file viewport memories.
#[derive(Debug, Clone, Default)]
pub(crate) struct Draft {
    query: String,
    selection: Selection,
}
impl Draft {
    pub fn query(&self) -> &str {
        &self.query
    }
    pub fn set_query(&mut self, query: String) -> bool {
        if self.query == query {
            return false;
        }
        self.query = query;
        self.selection = Selection::None;
        true
    }
    pub fn reveal(&mut self) {
        if let Selection::Current { reveal, .. } = &mut self.selection {
            *reveal = Reveal::Requested;
        }
    }
}

/// Results belong to the loaded pane, rather than accumulating in file memories.
#[derive(Debug, Clone, Default)]
pub(crate) struct Index {
    hits: Arc<Vec<Hit>>,
}
impl Index {
    pub fn rebuild(&mut self, text: &Text, gutter: usize, draft: &mut Draft) {
        let needle = draft.query.to_lowercase();
        let hits = if needle.is_empty() {
            Vec::new()
        } else {
            text.lines
                .iter()
                .enumerate()
                .flat_map(|(line, value)| {
                    let body =
                        crate::horizontal::clip(&value.text, gutter, value.width()).to_string();
                    ranges(&body, &needle).into_iter().map(move |columns| Hit {
                        line,
                        columns: columns.start + gutter..columns.end + gutter,
                    })
                })
                .collect::<Vec<_>>()
        };
        draft.selection = if hits.is_empty() {
            Selection::None
        } else {
            let index = match draft.selection {
                Selection::Current { index, .. } => index.min(hits.len() - 1),
                Selection::None => 0,
            };
            Selection::Current {
                index,
                reveal: Reveal::Requested,
            }
        };
        self.hits = Arc::new(hits);
    }
    pub fn step(&self, draft: &mut Draft, backwards: bool) {
        if self.hits.is_empty() {
            return;
        }
        let index = match draft.selection {
            Selection::Current { index, .. } if backwards => {
                (index + self.hits.len() - 1) % self.hits.len()
            }
            Selection::Current { index, .. } => (index + 1) % self.hits.len(),
            Selection::None => 0,
        };
        draft.selection = Selection::Current {
            index,
            reveal: Reveal::Requested,
        };
    }
    pub fn current(&self, draft: &Draft) -> Option<&Hit> {
        match draft.selection {
            Selection::Current { index, .. } => self.hits.get(index),
            Selection::None => None,
        }
    }
    pub fn take_reveal(&self, draft: &mut Draft) -> Option<Hit> {
        match &mut draft.selection {
            Selection::Current {
                index,
                reveal: reveal @ Reveal::Requested,
            } => {
                *reveal = Reveal::Visible;
                self.hits.get(*index).cloned()
            }
            _ => None,
        }
    }
    pub fn label(&self, draft: &Draft) -> String {
        match draft.selection {
            Selection::Current { index, .. } => format!("{}/{}", index + 1, self.hits.len()),
            Selection::None => "0/0".into(),
        }
    }
    pub fn line(&self, source: usize) -> &[Hit] {
        let start = self.hits.partition_point(|hit| hit.line < source);
        let end = self.hits.partition_point(|hit| hit.line <= source);
        &self.hits[start..end]
    }
}

fn ranges(body: &str, needle: &str) -> Vec<Range<usize>> {
    if body.is_ascii() {
        return body
            .to_ascii_lowercase()
            .match_indices(needle)
            .map(|(start, value)| start..start + value.len())
            .collect();
    }
    let mut folded = String::new();
    let mut map = Vec::new();
    let mut column = 0;
    for grapheme in body.graphemes(true) {
        folded.push_str(&grapheme.to_lowercase());
        let next = column + grapheme.width();
        map.push((folded.len(), column..next));
        column = next;
    }
    let mut result = Vec::new();
    for (start, value) in folded.match_indices(needle) {
        let first = map.partition_point(|(end, _)| *end <= start);
        let last = map.partition_point(|(end, _)| *end < start + value.len());
        let columns = map[first].1.start..map[last].1.end;
        if result.last() != Some(&columns) {
            result.push(columns);
        }
    }
    result
}

pub(crate) fn paint(
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    row: &crate::flow::Row,
    fixed: usize,
    offset: usize,
    index: &Index,
    draft: &Draft,
) {
    let fixed = fixed.min(usize::from(area.width));
    let start = row.columns.start + offset;
    let end = row
        .columns
        .end
        .min(start + usize::from(area.width).saturating_sub(fixed));
    if start >= end {
        return;
    }
    let line = index.line(row.source);
    let first = line.partition_point(|hit| hit.columns.end <= start);
    let last = line.partition_point(|hit| hit.columns.start < end);
    for hit in &line[first..last] {
        let from = hit.columns.start.max(start);
        let to = hit.columns.end.min(end);
        let current = index.current(draft) == Some(hit);
        let style = if current {
            Style::default()
                .bg(ratatui::style::Color::Rgb(40, 88, 120))
                .add_modifier(ratatui::style::Modifier::BOLD | ratatui::style::Modifier::UNDERLINED)
        } else {
            Style::default()
                .bg(ratatui::style::Color::Rgb(55, 50, 85))
                .add_modifier(ratatui::style::Modifier::UNDERLINED)
        };
        for column in from..to {
            frame.buffer_mut()[(area.x + (fixed + column - start) as u16, area.y)].set_style(style);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unicode_case_expansion_and_combining_graphemes_keep_real_cell_coordinates() {
        assert_eq!(ranges("中İ e\u{301}👩‍💻 中İ", "i"), vec![2..3, 10..11]);
        assert_eq!(ranges("e\u{301}e\u{301}", "\u{301}"), vec![0..1, 1..2]);
        assert_eq!(ranges("Foo FOO foo", "foo"), vec![0..3, 4..7, 8..11]);
    }
}
