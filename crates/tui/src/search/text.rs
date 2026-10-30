//! Literal text matches retain terminal-cell ranges independently from visual wrapping.
use super::{highlight::Emphasis, literal::ranges};
use crate::links::Text;
use std::{ops::Range, sync::Arc};

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
            Emphasis::Current
        } else {
            Emphasis::Other
        }
        .style();
        for column in from..to {
            frame.buffer_mut()[(area.x + (fixed + column - start) as u16, area.y)].set_style(style);
        }
    }
}
