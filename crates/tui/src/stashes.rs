//! Stash browsing retains object identity independently of reflog positions.
use engine::local::stash::{Entry, Snapshot};
use ratatui::widgets::ListState;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub(super) struct Session {
    pub snapshot: Arc<Snapshot>,
    pub state: ListState,
    pub query: String,
    pub search_mode: crate::search::Mode,
}
impl Session {
    pub fn new(snapshot: Snapshot) -> Self {
        let selected = (!snapshot.entries.is_empty()).then_some(0);
        Self {
            snapshot: Arc::new(snapshot),
            state: ListState::default().with_selected(selected),
            query: String::new(),
            search_mode: Default::default(),
        }
    }
    pub fn rows(&self) -> Vec<&Entry> {
        self.snapshot
            .entries
            .iter()
            .filter(|entry| self.search_mode.includes(self.matches_entry(entry)))
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
    pub fn matches_entry(&self, entry: &Entry) -> bool {
        let query = self.query.to_lowercase();
        entry.message.to_lowercase().contains(&query) || entry.oid.to_string().contains(&query)
    }
    pub fn matches(&self) -> crate::search::Matches {
        crate::search::Matches::new(self.rows().iter().map(|entry| self.matches_entry(entry)))
    }
    pub fn set_search_mode(&mut self, mode: crate::search::Mode) {
        let selected = self.selected().map(|entry| entry.oid);
        self.search_mode = mode;
        self.state.select(
            selected
                .and_then(|oid| self.rows().iter().position(|entry| entry.oid == oid))
                .or_else(|| (!self.rows().is_empty()).then_some(0)),
        );
        if mode == crate::search::Mode::Locate
            && let Some(index) = self.matches().nearest(self.state.selected())
        {
            self.state.select(Some(index));
        }
    }
    pub fn refreshed(&self, snapshot: Snapshot) -> Self {
        let current = self.selected().map(|entry| entry.oid);
        let mut next = Self::new(snapshot);
        next.state = self.state;
        next.query = self.query.clone();
        next.search_mode = self.search_mode;
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
        inspect: crate::inspection::Session,
    },
    Diff {
        session: Session,
        inspect: Box<crate::inspection::Session>,
        content: Box<crate::document::View>,
        scroll: usize,
    },
}
impl View {
    pub fn session(&self) -> &Session {
        match self {
            Self::List(session) | Self::Detail { session, .. } | Self::Diff { session, .. } => {
                session
            }
        }
    }
    pub fn selected(&self) -> Option<&Entry> {
        self.session().selected()
    }
    pub fn query(&self) -> &str {
        match self {
            Self::List(session) => &session.query,
            Self::Detail { inspect, .. } => &inspect.query,
            Self::Diff { inspect, .. } => &inspect.query,
        }
    }
    pub fn search(&mut self, query: String) {
        match self {
            Self::List(session) => session.search(query),
            Self::Detail { inspect, .. } => inspect.search(query),
            Self::Diff { .. } => {}
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
                    self.mode = crate::Mode::Stashes(view);
                    self.return_to_view();
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
                KeyCode::Enter | KeyCode::Right if session.selected().is_some() => {
                    self.mode = crate::Mode::Stashes(view);
                    self.open_stash_detail(engine::local::stash::preview::Scope::All);
                    return;
                }
                _ => {}
            },
            View::Detail { .. } if matches!(key.code, KeyCode::Tab | KeyCode::BackTab) => {
                self.mode = crate::Mode::Stashes(view);
                self.cycle_pane(key.code == KeyCode::BackTab);
                self.sync_bar();
                return;
            }
            View::Detail { session, inspect } => match inspect.key(key) {
                crate::inspection::Input::Back => *view = View::List(session.clone()),
                crate::inspection::Input::Open => {
                    self.mode = crate::Mode::Stashes(view);
                    self.show_stash_diff();
                    return;
                }
                crate::inspection::Input::Stay => {}
            },
            View::Diff {
                session,
                inspect,
                scroll,
                ..
            } => match key.code {
                KeyCode::Esc => {
                    *view = View::Detail {
                        session: session.clone(),
                        inspect: (**inspect).clone(),
                    }
                }
                _ => crate::input::scroll_key(scroll, key),
            },
        }
        self.mode = crate::Mode::Stashes(view);
        self.sync_bar();
    }
}

impl crate::Screen {
    pub(crate) fn open_stash_detail(&mut self, scope: engine::local::stash::preview::Scope) {
        let crate::Mode::Stashes(view) = &self.mode else {
            return;
        };
        let session = view.session().clone();
        let Some(entry) = view.selected().cloned() else {
            return;
        };
        let previous = self.mode.inspection().cloned();
        self.start_repository(
            session.snapshot.path.clone(),
            i18n::tr!(StashRead),
            crate::Effect::ReadOnly,
            move |service, cancel, _| {
                let detail = crate::inspection::Source::Stash(std::sync::Arc::new(
                    service.stash_detail(&session.snapshot.path, &entry, scope, &cancel)?,
                ));
                let inspect = match previous {
                    Some(previous) => previous.with_detail(detail),
                    None => crate::inspection::Session::new(detail),
                };
                Ok(crate::Outcome::StashView(Box::new(View::Detail {
                    session,
                    inspect,
                })))
            },
        );
    }
    fn show_stash_diff(&mut self) {
        let crate::Mode::Stashes(view) = &self.mode else {
            return;
        };
        let View::Detail { session, inspect } = view.as_ref() else {
            return;
        };
        let Some(index) = inspect.selected_index() else {
            return;
        };
        let (session, inspect) = (session.clone(), inspect.clone());
        self.start_repository(
            session.snapshot.path.clone(),
            i18n::tr!(HistoryReadDiff),
            crate::Effect::ReadOnly,
            move |service, cancel, _| {
                let content = inspect.prepare_file(&service, index, &cancel)?;
                Ok(crate::Outcome::StashView(Box::new(View::Diff {
                    session,
                    inspect: Box::new(inspect),
                    content,
                    scroll: 0,
                })))
            },
        );
    }
}
