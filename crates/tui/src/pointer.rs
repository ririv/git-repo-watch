//! Hit regions are emitted by the same layout pass that draws the visible content.
use ratatui::layout::{Position, Rect};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Target {
    Repositories,
    Files,
    Branches,
    Stashes,
    History,
    HistoryFiles,
    Settings,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScrollTarget {
    Text,
    Overview,
    Diff,
    HistoryMessage,
    HistoryHorizontal,
    DiffHorizontal,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Action {
    Select(Target, usize),
    Scroll(ScrollTarget, usize),
}
#[derive(Debug, Clone)]
enum Region {
    Item(Rect, Action),
    Track {
        area: Rect,
        axis: crate::scrollbar::Axis,
        values: Values,
    },
}
#[derive(Debug, Clone)]
enum Values {
    Rows(Target, Vec<usize>),
    Offset(ScrollTarget, usize),
}
#[derive(Debug, Default, Clone)]
pub(crate) struct Map(Vec<Region>);
impl Map {
    pub fn clear(&mut self) {
        self.0.clear();
    }
    pub fn item(&mut self, area: Rect, target: Target, index: usize) {
        if !area.is_empty() {
            self.0
                .push(Region::Item(area, Action::Select(target, index)));
        }
    }
    pub fn list(
        &mut self,
        panel: Rect,
        content: Rect,
        indices: &[usize],
        offset: usize,
        target: Target,
    ) {
        for (row, index) in indices
            .iter()
            .skip(offset)
            .take(content.height as usize)
            .enumerate()
        {
            self.item(
                Rect::new(content.x, content.y + row as u16, content.width, 1),
                target,
                *index,
            );
        }
        if indices.len() > content.height as usize && !content.is_empty() {
            self.0.push(Region::Track {
                area: crate::scrollbar::vertical_area(panel, content),
                axis: crate::scrollbar::Axis::Vertical,
                values: Values::Rows(target, indices.to_vec()),
            });
        }
    }
    pub fn vertical(&mut self, panel: Rect, content: Rect, maximum: usize, target: ScrollTarget) {
        self.track(
            crate::scrollbar::vertical_area(panel, content),
            crate::scrollbar::Axis::Vertical,
            maximum,
            target,
        );
    }
    pub fn track(
        &mut self,
        area: Rect,
        axis: crate::scrollbar::Axis,
        maximum: usize,
        target: ScrollTarget,
    ) {
        if maximum > 0 && !area.is_empty() {
            self.0.push(Region::Track {
                area,
                axis,
                values: Values::Offset(target, maximum),
            });
        }
    }
    pub fn hit(&self, point: Position) -> Option<Action> {
        self.0.iter().rev().find_map(|region| match region {
            Region::Item(area, action) => area.contains(point).then_some(*action),
            Region::Track { area, axis, values } if area.contains(point) => {
                let (position, length) = match axis {
                    crate::scrollbar::Axis::Horizontal => (point.x - area.x, area.width),
                    crate::scrollbar::Axis::Vertical => (point.y - area.y, area.height),
                };
                let maximum = match values {
                    Values::Rows(_, rows) => rows.len().saturating_sub(1),
                    Values::Offset(_, maximum) => *maximum,
                };
                let index = (position as usize * maximum
                    + usize::from(length.saturating_sub(1)) / 2)
                    / usize::from(length.saturating_sub(1).max(1));
                Some(match values {
                    Values::Rows(target, rows) => Action::Select(*target, rows[index]),
                    Values::Offset(target, _) => Action::Scroll(*target, index),
                })
            }
            _ => None,
        })
    }
}
