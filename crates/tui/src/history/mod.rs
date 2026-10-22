//! Commit-graph sessions and read-only workflows, separate from terminal drawing.
mod actions;
mod session;
use crate::document;
use engine::local::history::Detail;
use ratatui::widgets::ListState;
pub(super) use session::Session;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub(super) struct Commit {
    pub graph: Session,
    pub detail: Arc<Detail>,
    pub files: ListState,
    pub scroll: usize,
    pub query: String,
}
impl Commit {
    fn new(graph: Session, detail: Detail) -> Self {
        let selected = (!detail.changes.is_empty()).then_some(0);
        Self {
            graph,
            detail: Arc::new(detail),
            files: ListState::default().with_selected(selected),
            scroll: 0,
            query: String::new(),
        }
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
        let query = self.query.to_lowercase();
        self.detail
            .changes
            .iter()
            .filter(|change| {
                change
                    .location
                    .name()
                    .to_string_lossy()
                    .to_lowercase()
                    .contains(&query)
            })
            .collect()
    }
    fn search(&mut self, query: String) {
        self.query = query;
        self.files.select((!self.rows().is_empty()).then_some(0));
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
