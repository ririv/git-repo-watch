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
    pub filter: Filter,
}
impl Session {
    pub fn new(snapshot: Snapshot) -> Self {
        let selected = (!snapshot.entries.is_empty()).then_some(0);
        Self {
            snapshot: Arc::new(snapshot),
            state: ListState::default().with_selected(selected),
            query: String::new(),
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
            .filter(|entry| entry.name.to_lowercase().contains(&query))
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
            self.state.select((!self.rows().is_empty()).then_some(0));
        }
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
impl crate::Screen {
    pub(super) fn read_branches(&mut self, path: std::path::PathBuf) {
        self.start_repository(
            path.clone(),
            i18n::tr!(BranchCatalogRead),
            crate::Effect::ReadOnly,
            move |service, cancel, _| {
                Ok(crate::Outcome::Branches(Box::new(View::List(
                    Session::new(service.branches(&path, &cancel)?),
                ))))
            },
        );
    }
}
