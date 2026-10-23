//! Stash browsing retains object identity independently of reflog positions.
use engine::local::stash::{Entry, Snapshot};
use ratatui::widgets::ListState;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub(super) struct Session {
    pub snapshot: Arc<Snapshot>,
    pub state: ListState,
    pub query: String,
}
impl Session {
    pub fn new(snapshot: Snapshot) -> Self {
        let selected = (!snapshot.entries.is_empty()).then_some(0);
        Self {
            snapshot: Arc::new(snapshot),
            state: ListState::default().with_selected(selected),
            query: String::new(),
        }
    }
    pub fn rows(&self) -> Vec<&Entry> {
        let query = self.query.to_lowercase();
        self.snapshot
            .entries
            .iter()
            .filter(|entry| {
                entry.message.to_lowercase().contains(&query)
                    || entry.oid.to_string().contains(&query)
            })
            .collect()
    }
    pub fn selected(&self) -> Option<&Entry> {
        self.state
            .selected()
            .and_then(|i| self.rows().get(i).copied())
    }
    pub fn advance(&mut self, delta: isize) {
        self.state
            .select(self.rows().len().checked_sub(1).map(|last| {
                self.state
                    .selected()
                    .unwrap_or(0)
                    .saturating_add_signed(delta)
                    .min(last)
            }));
    }
    pub fn search(&mut self, query: String) {
        if query != self.query {
            self.query = query;
            self.state.select((!self.rows().is_empty()).then_some(0));
        }
    }
    pub fn refreshed(&self, snapshot: Snapshot) -> Self {
        let current = self.selected().map(|entry| entry.oid);
        let mut next = Self::new(snapshot);
        next.query = self.query.clone();
        let selected = next
            .rows()
            .iter()
            .position(|entry| Some(entry.oid) == current)
            .or_else(|| {
                next.rows()
                    .len()
                    .checked_sub(1)
                    .map(|last| self.state.selected().unwrap_or(0).min(last))
            });
        next.state.select(selected);
        next
    }
}
#[derive(Debug)]
pub(super) enum View {
    List(Session),
    Detail {
        session: Session,
        entry: Entry,
        scroll: usize,
    },
}
impl View {
    pub fn session(&self) -> &Session {
        match self {
            Self::List(session) | Self::Detail { session, .. } => session,
        }
    }
    pub fn selected(&self) -> Option<&Entry> {
        match self {
            Self::List(session) => session.selected(),
            Self::Detail { entry, .. } => Some(entry),
        }
    }
    pub fn search(&mut self, query: String) {
        if self.session().query != query {
            let mut session = self.session().clone();
            session.search(query);
            *self = Self::List(session);
        }
    }
}
impl crate::Screen {
    pub(super) fn read_stashes(&mut self, path: std::path::PathBuf) {
        self.start_repository(
            path.clone(),
            i18n::tr!(StashRead),
            crate::Effect::ReadOnly,
            move |service, cancel, _| Ok(crate::Outcome::Stashes(service.stashes(&path, &cancel)?)),
        );
    }
    pub(super) fn stash_key(&mut self, mut view: Box<View>, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;
        match view.as_mut() {
            View::List(session) => match key.code {
                KeyCode::Esc | KeyCode::Left => {
                    self.mode = crate::Mode::Browsing;
                    self.sync_bar();
                    return;
                }
                KeyCode::Up => session.advance(-1),
                KeyCode::Down => session.advance(1),
                KeyCode::PageUp => session.advance(-10),
                KeyCode::PageDown => session.advance(10),
                KeyCode::Home => session
                    .state
                    .select((!session.rows().is_empty()).then_some(0)),
                KeyCode::End => session.state.select(session.rows().len().checked_sub(1)),
                KeyCode::Enter | KeyCode::Right => {
                    if let Some(entry) = session.selected().cloned() {
                        *view = View::Detail {
                            session: session.clone(),
                            entry,
                            scroll: 0,
                        };
                    }
                }
                _ => {}
            },
            View::Detail {
                session, scroll, ..
            } => match key.code {
                KeyCode::Esc | KeyCode::Left => *view = View::List(session.clone()),
                _ => crate::input::scroll_key(scroll, key),
            },
        }
        self.mode = crate::Mode::Stashes(view);
        self.sync_bar();
    }
}
