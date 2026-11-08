//! Shared navigation and continuous diff state for immutable repository snapshots.
pub(crate) mod review;
mod source;
use crate::{Mode, Screen};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::widgets::ListState;
pub(crate) use source::Source;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Input {
    Stay,
    Back,
    Open,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Focus {
    Message,
    Files,
    Preview,
}

#[derive(Debug, Clone)]
pub(crate) struct Session {
    pub detail: Source,
    pub files: ListState,
    pub scroll: usize,
    pub query: String,
    pub search_mode: crate::search::Mode,
    pub focus: Focus,
    pub arrangement: crate::workspace::Arrangement,
    pub review: review::Review,
}
impl Session {
    pub fn prepare_file(
        &self,
        service: &crate::Service,
        index: usize,
        theme: highlight::Theme,
        cancel: &engine::cancel::Cancel,
    ) -> anyhow::Result<Box<crate::document::View>> {
        crate::document::prepare_with_theme(
            &self.detail.change(index).location.display(),
            self.detail.document(service, index, cancel)?,
            self.detail.comparison(index),
            theme,
            cancel,
        )
    }
    pub fn key(&mut self, key: KeyEvent) -> Input {
        use crate::workspace::Arrangement;
        match key.code {
            KeyCode::Esc if self.arrangement == Arrangement::Maximized => {
                self.arrangement = Arrangement::Tiled
            }
            KeyCode::Esc | KeyCode::Left => return Input::Back,
            KeyCode::Enter | KeyCode::Right if self.focus != Focus::Message => {
                self.open_preview_file();
                return Input::Open;
            }
            KeyCode::Up | KeyCode::Down | KeyCode::PageUp | KeyCode::PageDown => {
                let delta = match key.code {
                    KeyCode::Up => -1,
                    KeyCode::Down => 1,
                    KeyCode::PageUp => -10,
                    _ => 10,
                };
                match self.focus {
                    Focus::Files => self.advance(delta),
                    Focus::Message => self.scroll = self.scroll.saturating_add_signed(delta),
                    Focus::Preview => self.review.advance(delta),
                }
            }
            KeyCode::Home | KeyCode::End => match self.focus {
                Focus::Files => self.files.select(if key.code == KeyCode::Home {
                    (!self.rows().is_empty()).then_some(0)
                } else {
                    self.rows().len().checked_sub(1)
                }),
                Focus::Message => {
                    self.scroll = if key.code == KeyCode::Home {
                        0
                    } else {
                        usize::MAX
                    }
                }
                Focus::Preview => self.review.seek(if key.code == KeyCode::Home {
                    0
                } else {
                    usize::MAX
                }),
            },
            _ => {}
        }
        Input::Stay
    }
    pub fn new(detail: Source) -> Self {
        let selected = (detail.len() > 0).then_some(0);
        let review = review::Review::new(detail.clone());
        Self {
            detail,
            files: ListState::default().with_selected(selected),
            scroll: 0,
            query: String::new(),
            search_mode: Default::default(),
            focus: Focus::Files,
            arrangement: crate::workspace::Arrangement::Tiled,
            review,
        }
    }
    pub fn with_detail(&self, detail: Source) -> Self {
        let selected = self
            .files
            .selected()
            .and_then(|i| self.rows().get(i).map(|change| change.location.clone()));
        let mut next = Self::new(detail);
        next.search_mode = self.search_mode;
        next.focus = self.focus;
        next.arrangement = self.arrangement;
        next.search(self.query.clone());
        if let Some(index) = selected.and_then(|location| {
            next.rows()
                .iter()
                .position(|change| change.location == location)
        }) {
            next.files.select(Some(index));
        }
        next
    }
    pub fn advance(&mut self, delta: isize) {
        let count = self.rows().len();
        if count > 0 {
            let current = self.files.selected().unwrap_or(0);
            self.files
                .select(Some(current.saturating_add_signed(delta).min(count - 1)));
        }
    }
    pub fn rows(&self) -> Vec<&engine::local::history::Change> {
        self.indices()
            .into_iter()
            .map(|index| self.detail.change(index))
            .collect()
    }
    pub fn selected_index(&self) -> Option<usize> {
        self.files
            .selected()
            .and_then(|index| self.indices().get(index).copied())
    }
    pub fn open_preview_file(&mut self) {
        if self.focus == Focus::Preview
            && let Some(file) = self.review.visible_file(self.selected_index())
            && let Some(index) = self.indices().iter().position(|index| *index == file)
        {
            self.files.select(Some(index));
        }
    }
    pub fn indices(&self) -> Vec<usize> {
        let query = self.query.to_lowercase();
        self.detail
            .changes()
            .into_iter()
            .enumerate()
            .filter(|(_, change)| {
                self.search_mode.includes(
                    change
                        .location
                        .name()
                        .to_string_lossy()
                        .to_lowercase()
                        .contains(&query),
                )
            })
            .map(|(index, _)| index)
            .collect()
    }
    pub fn matches(&self) -> crate::search::Matches {
        let query = self.query.to_lowercase();
        crate::search::Matches::new(self.rows().iter().map(|change| {
            change
                .location
                .name()
                .to_string_lossy()
                .to_lowercase()
                .contains(&query)
        }))
    }
    pub fn set_search_mode(&mut self, mode: crate::search::Mode) {
        let selected = self
            .files
            .selected()
            .and_then(|i| self.rows().get(i).map(|change| change.location.clone()));
        self.search_mode = mode;
        self.files.select(
            selected
                .and_then(|location| {
                    self.rows()
                        .iter()
                        .position(|change| change.location == location)
                })
                .or_else(|| (!self.rows().is_empty()).then_some(0)),
        );
        if mode == crate::search::Mode::Locate
            && let Some(index) = self.matches().nearest(self.files.selected())
        {
            self.files.select(Some(index));
        }
    }
    pub fn search(&mut self, query: String) {
        self.query = query;
        match self.search_mode {
            crate::search::Mode::Filter => {
                self.files.select((!self.rows().is_empty()).then_some(0))
            }
            crate::search::Mode::Locate => {
                if let Some(index) = self.matches().nearest(self.files.selected()) {
                    self.files.select(Some(index));
                }
            }
        }
    }
}

impl Mode {
    pub(crate) fn inspection(&self) -> Option<&Session> {
        match self {
            Self::History(view) => match view.as_ref() {
                crate::history::View::Commit(commit) => Some(&commit.inspect),
                _ => None,
            },
            Self::Stashes(view) => match view.as_ref() {
                crate::stashes::View::Detail { inspect, .. } => Some(inspect),
                _ => None,
            },
            _ => None,
        }
    }
    pub(crate) fn inspection_mut(&mut self) -> Option<&mut Session> {
        match self {
            Self::History(view) => match view.as_mut() {
                crate::history::View::Commit(commit) => Some(&mut commit.inspect),
                _ => None,
            },
            Self::Stashes(view) => match view.as_mut() {
                crate::stashes::View::Detail { inspect, .. } => Some(inspect),
                _ => None,
            },
            _ => None,
        }
    }
}
impl Screen {
    pub(crate) fn poll_inspection(&mut self) -> bool {
        let Some(area) = self.body_area else {
            return false;
        };
        let Some(inspect) = self.mode.inspection() else {
            return false;
        };
        if crate::view::inspection::areas(area, inspect)
            .iter()
            .any(|(_, focus)| *focus == Focus::Preview)
        {
            inspect
                .review
                .poll(&self.service, self.background.syntax_theme())
        } else {
            false
        }
    }
}
