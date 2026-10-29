//! Commit-graph sessions and read-only workflows, separate from terminal drawing.
mod actions;
pub(crate) mod review;
mod selection;
mod session;
use crate::document;
use engine::local::history::Detail;
use ratatui::widgets::ListState;
pub(super) use session::Session;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionAction {
    Start,
    Toggle,
    All,
    Clear,
    Done,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Focus {
    Message,
    Files,
    Preview,
}

#[derive(Debug, Clone)]
pub(super) struct Commit {
    pub graph: Session,
    pub detail: Arc<Detail>,
    pub files: ListState,
    pub scroll: usize,
    pub query: String,
    pub search_mode: crate::search::Mode,
    pub focus: Focus,
    pub arrangement: crate::workspace::Arrangement,
    pub review: review::Review,
}
impl Commit {
    fn new(graph: Session, detail: Detail) -> Self {
        let selected = (!detail.changes.is_empty()).then_some(0);
        let detail = Arc::new(detail);
        let review = review::Review::new(detail.clone());
        Self {
            graph,
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
    fn with_detail(&self, detail: Detail) -> Self {
        let selected = self
            .files
            .selected()
            .and_then(|i| self.rows().get(i).map(|change| change.location.clone()));
        let mut next = Self::new(self.graph.clone(), detail);
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
            .map(|index| &self.detail.changes[index])
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
            .changes
            .iter()
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
    fn search(&mut self, query: String) {
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

#[derive(Debug)]
pub(super) enum View {
    Graph(Session),
    Commit(Commit),
    Diff {
        commit: Box<Commit>,
        content: Box<document::View>,
        scroll: usize,
    },
}
impl View {
    pub fn session(&self) -> &Session {
        match self {
            Self::Graph(session) => session,
            Self::Commit(commit) => &commit.graph,
            Self::Diff { commit, .. } => &commit.graph,
        }
    }
    pub fn search(&mut self, query: String) {
        if self.query() == query {
            return;
        }
        match self {
            Self::Graph(session) => session.search(query),
            Self::Commit(commit) => commit.search(query),
            Self::Diff { commit, .. } => {
                let mut commit = (**commit).clone();
                commit.search(query);
                *self = Self::Commit(commit);
            }
        }
    }
    pub fn query(&self) -> &str {
        match self {
            Self::Graph(session) => &session.query,
            Self::Commit(commit) => &commit.query,
            Self::Diff { commit, .. } => &commit.query,
        }
    }
    pub fn clear_search(&mut self) {
        match self {
            Self::Graph(session) => session.search(String::new()),
            Self::Commit(commit) => commit.search(String::new()),
            Self::Diff { commit, .. } => commit.search(String::new()),
        }
    }
}
