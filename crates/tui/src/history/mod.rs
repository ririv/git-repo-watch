//! Commit-graph sessions and read-only workflows, separate from terminal drawing.
mod actions;
pub(crate) mod menu;
mod menu_actions;
pub(crate) mod preview;
mod selection;
mod session;
use crate::document;
use engine::local::history::Detail;
pub(super) use session::{GraphFocus, Session};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionAction {
    Start,
    Toggle,
    All,
    Clear,
    Done,
}

pub(crate) use crate::inspection::Focus;
#[derive(Debug, Clone)]
pub(super) struct Commit {
    pub graph: Session,
    pub inspect: Box<crate::inspection::Session>,
}
impl Commit {
    fn new(graph: Session, detail: Detail) -> Self {
        let source = crate::inspection::Source::Commit(Arc::new(detail));
        Self {
            graph,
            inspect: Box::new(crate::inspection::Session::new(source)),
        }
    }
    fn with_detail(&self, detail: Detail) -> Self {
        Self {
            graph: self.graph.clone(),
            inspect: Box::new(
                self.inspect
                    .with_detail(crate::inspection::Source::Commit(Arc::new(detail))),
            ),
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
            Self::Commit(commit) => commit.inspect.search(query),
            Self::Diff { commit, .. } => {
                let mut commit = (**commit).clone();
                commit.inspect.search(query);
                *self = Self::Commit(commit);
            }
        }
    }
    pub fn query(&self) -> &str {
        match self {
            Self::Graph(session) => &session.query,
            Self::Commit(commit) => &commit.inspect.query,
            Self::Diff { commit, .. } => &commit.inspect.query,
        }
    }
    pub fn clear_search(&mut self) {
        match self {
            Self::Graph(session) => session.search(String::new()),
            Self::Commit(commit) => commit.inspect.search(String::new()),
            Self::Diff { commit, .. } => commit.inspect.search(String::new()),
        }
    }
}
