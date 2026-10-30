//! Hit regions are emitted by the same layout pass that draws the visible content.
pub(crate) mod drag;
use ratatui::layout::{Position, Rect};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Target {
    Management,
    CompositionCommits,
    CompositionUnits,
    Browser,
    Repositories,
    Files,
    Branches,
    Stashes,
    History,
    HistoryFiles,
    ReplayParent,
    Settings,
    ScheduleButton,
    WorkflowSteps,
    WorkflowToggle,
    WorkflowActions,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScrollTarget {
    BrowserTreeHorizontal,
    HistoryActions,
    BrowserPreview(crate::browser::opening::TextMode),
    BrowserHorizontal(crate::browser::opening::TextMode),
    Text,
    CommitMessage,
    Overview,
    Diff,
    HistoryMessage,
    HistoryReview,
    HistoryReviewHorizontal,
    HistoryHorizontal,
    DiffHorizontal,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Action {
    Management(crate::management::Action),
    ManagementToggle(usize),
    BrowserDirectory(usize),
    PushRemote(usize),
    HistoryMenu(crate::history::menu::PointerInput),
    RepositoryMenu(crate::repository_list::Menu),
    DragRows {
        target: Target,
        index: usize,
        offset: usize,
    },
    ToggleFile(usize),
    ToggleCommit(usize),
    ReplayControl(engine::operations::replay::Control),
    Select(Target, usize),
    Scroll(ScrollTarget, usize),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Hit {
    Button(Action),
    FileCheckbox(usize),
    Item(Target, usize),
    Scrollbar(Action),
}
impl Hit {
    pub fn action(self) -> Action {
        match self {
            Self::Button(action) => action,
            Self::FileCheckbox(index) => Action::ToggleFile(index),
            Self::Item(target, index) => Action::Select(target, index),
            Self::Scrollbar(action) => action,
        }
    }
}
#[derive(Debug, Clone)]
enum Region {
    Button(Rect, Action),
    FileCheckbox(Rect, usize),
    Item(Rect, Target, usize),
    Track {
        area: Rect,
        axis: crate::scrollbar::Axis,
        values: Values,
        offset: usize,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
enum Values {
    Rows(Target, Vec<usize>),
    Offset(ScrollTarget, usize),
}
#[derive(Debug, Default, Clone)]
pub(crate) struct Map {
    regions: Vec<Region>,
    pub links: crate::links::Map,
}
impl Map {
    pub fn button(&mut self, area: Rect, action: Action) {
        if !area.is_empty() {
            self.regions.push(Region::Button(area, action));
        }
    }
    pub fn file_checkbox(&mut self, area: Rect, index: usize) {
        if !area.is_empty() {
            self.regions.push(Region::FileCheckbox(area, index));
        }
    }
    pub fn clear(&mut self) {
        self.regions.clear();
    }
    pub fn item(&mut self, area: Rect, target: Target, index: usize) {
        if !area.is_empty() {
            self.regions.push(Region::Item(area, target, index));
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
            self.regions.push(Region::Track {
                area: crate::scrollbar::vertical_area(panel, content),
                axis: crate::scrollbar::Axis::Vertical,
                values: Values::Rows(target, indices.to_vec()),
                offset,
            });
        }
    }
    pub fn vertical(
        &mut self,
        panel: Rect,
        content: Rect,
        maximum: usize,
        offset: usize,
        target: ScrollTarget,
    ) {
        self.track(
            crate::scrollbar::vertical_area(panel, content),
            crate::scrollbar::Axis::Vertical,
            maximum,
            offset,
            target,
        );
    }
    pub fn track(
        &mut self,
        area: Rect,
        axis: crate::scrollbar::Axis,
        maximum: usize,
        offset: usize,
        target: ScrollTarget,
    ) {
        if maximum > 0 && !area.is_empty() {
            self.regions.push(Region::Track {
                area,
                axis,
                values: Values::Offset(target, maximum),
                offset,
            });
        }
    }
    pub fn hit(&self, point: Position) -> Option<Hit> {
        self.regions.iter().rev().find_map(|region| match region {
            Region::Button(area, action) => area.contains(point).then_some(Hit::Button(*action)),
            Region::FileCheckbox(area, index) if area.contains(point) => {
                Some(Hit::FileCheckbox(*index))
            }
            Region::Item(area, target, index) => {
                area.contains(point).then_some(Hit::Item(*target, *index))
            }
            Region::Track {
                area, axis, values, ..
            } if area.contains(point) => {
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
                Some(Hit::Scrollbar(match values {
                    Values::Rows(target, rows) => Action::Select(*target, rows[index]),
                    Values::Offset(target, _) => Action::Scroll(*target, index),
                }))
            }
            _ => None,
        })
    }
}
