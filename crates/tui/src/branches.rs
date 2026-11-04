//! Branch browsing sessions and read-only commands.
use engine::local::branches::catalog::{Entry, Kind, Snapshot};
use ratatui::widgets::ListState;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum Filter {
    #[default]
    All,
    Local,
    Remote,
}
impl Filter {
    pub fn title(self) -> &'static str {
        match self {
            Self::All => i18n::tr!(BranchAll),
            Self::Local => i18n::tr!(BranchLocal),
            Self::Remote => i18n::tr!(BranchRemote),
        }
    }
    fn next(self, backwards: bool) -> Self {
        match (self, backwards) {
            (Self::All, false) | (Self::Remote, true) => Self::Local,
            (Self::Local, false) | (Self::All, true) => Self::Remote,
            (Self::Remote, false) | (Self::Local, true) => Self::All,
        }
    }
}
#[derive(Debug, Clone)]
pub(super) struct Session {
    pub snapshot: Arc<Snapshot>,
    pub state: ListState,
    pub query: String,
    pub search_mode: crate::search::Mode,
    pub filter: Filter,
}
impl Session {
    pub fn new(snapshot: Snapshot) -> Self {
        let selected = (!snapshot.entries.is_empty()).then_some(0);
        Self {
            snapshot: Arc::new(snapshot),
            state: ListState::default().with_selected(selected),
            query: String::new(),
            search_mode: Default::default(),
            filter: Filter::All,
        }
    }
    pub fn rows(&self) -> Vec<&Entry> {
        let query = self.query.to_lowercase();
        self.snapshot
            .entries
            .iter()
            .filter(|entry| {
                matches!(
                    (self.filter, &entry.kind),
                    (Filter::All, _)
                        | (Filter::Local, Kind::Local(_))
                        | (Filter::Remote, Kind::Remote)
                )
            })
            .filter(|entry| {
                self.search_mode
                    .includes(entry.name.to_lowercase().contains(&query))
            })
            .collect()
    }
    pub fn selected(&self) -> Option<&Entry> {
        self.state
            .selected()
            .and_then(|index| self.rows().get(index).copied())
    }
    pub fn search(&mut self, query: String) {
        if self.query != query {
            self.query = query;
            match self.search_mode {
                crate::search::Mode::Filter => {
                    self.state.select((!self.rows().is_empty()).then_some(0))
                }
                crate::search::Mode::Locate => {
                    if let Some(index) = self.matches().nearest(self.state.selected()) {
                        self.state.select(Some(index));
                    }
                }
            }
        }
    }
    pub fn matches(&self) -> crate::search::Matches {
        let query = self.query.to_lowercase();
        crate::search::Matches::new(
            self.rows()
                .iter()
                .map(|entry| entry.name.to_lowercase().contains(&query)),
        )
    }
    pub fn set_search_mode(&mut self, mode: crate::search::Mode) {
        let selected = self.selected().map(|entry| entry.reference.clone());
        self.search_mode = mode;
        self.state.select(
            selected
                .and_then(|reference| {
                    self.rows()
                        .iter()
                        .position(|entry| entry.reference == reference)
                })
                .or_else(|| (!self.rows().is_empty()).then_some(0)),
        );
        if mode == crate::search::Mode::Locate
            && let Some(index) = self.matches().nearest(self.state.selected())
        {
            self.state.select(Some(index));
        }
    }
    pub fn refreshed(&self, snapshot: Snapshot) -> Self {
        let selected = self.selected().map(|entry| entry.reference.clone());
        let mut next = Self::new(snapshot);
        next.state = self.state;
        next.filter = self.filter;
        next.query = self.query.clone();
        next.search_mode = self.search_mode;
        next.state.select(
            selected
                .and_then(|reference| {
                    next.rows()
                        .iter()
                        .position(|entry| entry.reference == reference)
                })
                .or_else(|| (!next.rows().is_empty()).then_some(0)),
        );
        next
    }
    pub fn filter_next(&mut self, backwards: bool) {
        self.filter = self.filter.next(backwards);
        self.state.select((!self.rows().is_empty()).then_some(0));
    }
    pub fn advance(&mut self, delta: isize) {
        let last = self.rows().len().checked_sub(1);
        self.state.select(last.map(|last| {
            self.state
                .selected()
                .unwrap_or(0)
                .saturating_add_signed(delta)
                .min(last)
        }));
    }
}
#[derive(Debug)]
pub(super) enum View {
    List(Session),
    Detail {
        session: Session,
        entry: Arc<Entry>,
        scroll: usize,
    },
}
impl View {
    pub fn session(&self) -> &Session {
        match self {
            Self::List(session) | Self::Detail { session, .. } => session,
        }
    }
    pub fn search(&mut self, query: String) {
        if self.session().query == query {
            return;
        }
        let mut session = self.session().clone();
        session.search(query);
        *self = Self::List(session);
    }
}
