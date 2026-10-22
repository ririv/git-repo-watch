//! Simultaneous repository panes share existing file and history interaction states.
mod load;
use crate::{FileSession, Mode, Page, Screen, Service, history};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Section {
    Overview,
    Worktree,
    History,
}
impl Section {
    pub const ALL: [Self; 3] = [Self::Overview, Self::Worktree, Self::History];
    pub fn title(self) -> &'static str {
        match self {
            Self::Overview => i18n::tr!(WorkspaceOverview),
            Self::Worktree => i18n::tr!(Worktree),
            Self::History => i18n::tr!(WorkspaceHistory),
        }
    }
    pub fn of(mode: &Mode) -> Self {
        match mode {
            Mode::Panel(section) => *section,
            Mode::Files(_)
            | Mode::Diff { .. }
            | Mode::CommitMessage { .. }
            | Mode::CommitConfirm { .. } => Self::Worktree,
            Mode::History(_) => Self::History,
            Mode::Busy(pending) => Self::of(&pending.origin),
            Mode::Error { origin, .. } => Self::of(origin),
            _ => Self::Overview,
        }
    }
    pub fn advance(self, backwards: bool) -> Self {
        let index = Self::ALL
            .iter()
            .position(|section| *section == self)
            .expect("known repository pane");
        Self::ALL[(index + if backwards { 2 } else { 1 }) % 3]
    }
}
#[derive(Debug)]
pub(super) enum Panel<T> {
    Loading,
    Ready(T),
    Failed(String),
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum Arrangement {
    #[default]
    Tiled,
    Maximized,
}
#[derive(Debug, Default)]
enum State {
    #[default]
    Closed,
    Open(Box<Session>),
}
#[derive(Debug)]
pub(super) struct Session {
    path: PathBuf,
    pub arrangement: Arrangement,
    pub files: Panel<FileSession>,
    pub history: Panel<history::Session>,
}
#[derive(Debug, Default)]
pub(super) struct Workspace {
    state: State,
    loader: load::Loader,
}
impl Workspace {
    #[cfg(test)]
    pub fn pending(&self) -> bool {
        self.loader.pending()
    }
    pub fn session(&self) -> Option<&Session> {
        match &self.state {
            State::Closed => None,
            State::Open(session) => Some(session),
        }
    }
    pub fn session_mut(&mut self) -> Option<&mut Session> {
        match &mut self.state {
            State::Closed => None,
            State::Open(session) => Some(session),
        }
    }
    pub fn path(&self) -> Option<&Path> {
        self.session().map(|session| session.path.as_path())
    }
    fn open(&mut self, path: PathBuf, service: &Arc<Service>) {
        self.state = State::Open(Box::new(Session {
            path: path.clone(),
            arrangement: Arrangement::Tiled,
            files: Panel::Loading,
            history: Panel::Loading,
        }));
        self.loader.request(path, service);
    }
    pub fn save(&mut self, mode: Mode) {
        let Some(session) = self.session_mut() else {
            return;
        };
        match mode {
            Mode::Files(files) | Mode::Diff { session: files, .. } => {
                session.files = Panel::Ready(files)
            }
            Mode::History(view) => session.history = Panel::Ready(view.session().clone()),
            Mode::Confirm {
                operation: crate::Confirmation::Configure { origin, .. },
                ..
            } => self.save(*origin),
            _ => {}
        }
    }
    pub fn invalidate(&mut self) {
        if let Some(session) = self.session_mut() {
            session.files = Panel::Loading;
            session.history = Panel::Loading;
        }
    }
    fn reload(&mut self, service: &Arc<Service>) {
        if let Some(path) = self.path().map(Path::to_owned) {
            self.loader.request(path, service);
        }
    }
    pub fn spinner(&self) -> &'static str {
        self.loader.spinner()
    }
    pub fn animate(&mut self) -> bool {
        self.loader.animate()
    }
    pub fn shutdown(&mut self) {
        self.loader.shutdown();
    }
    pub fn is_root(mode: &Mode) -> bool {
        match mode {
            Mode::Browsing | Mode::Panel(_) | Mode::Files(_) => true,
            Mode::History(view) => matches!(view.as_ref(), history::View::Graph(_)),
            Mode::Busy(pending) => Self::is_root(&pending.origin),
            _ => false,
        }
    }
}
impl Screen {
    pub(super) fn enter_repository(&mut self, path: PathBuf) {
        self.workspace.open(path.clone(), &self.service);
        self.overview.open(path, &self.service);
    }
    pub(super) fn switch_repository_section(&mut self, section: Section) {
        if self.page != Page::Detail
            || !matches!(
                self.mode,
                Mode::Browsing
                    | Mode::Panel(_)
                    | Mode::Files(_)
                    | Mode::Diff { .. }
                    | Mode::History(_)
            )
        {
            return;
        }
        if Section::of(&self.mode) == section {
            return;
        }
        let mode = self.take_mode();
        self.workspace.save(mode);
        let Some(session) = self.workspace.session_mut() else {
            return;
        };
        self.mode = match section {
            Section::Overview => Mode::Browsing,
            Section::Worktree => match &session.files {
                Panel::Ready(files) => Mode::Files(files.clone()),
                _ => Mode::Panel(section),
            },
            Section::History => match &session.history {
                Panel::Ready(graph) => Mode::History(Box::new(history::View::Graph(graph.clone()))),
                _ => Mode::Panel(section),
            },
        };
        self.message.clear();
        self.sync_bar();
    }
    pub(super) fn poll_workspace(&mut self) -> bool {
        let events = self.workspace.loader.poll(&self.service);
        let changed = !events.is_empty();
        for (path, event) in events {
            if self.workspace.path() != Some(&path) {
                continue;
            }
            let focus = Section::of(&self.mode);
            let session = self
                .workspace
                .session_mut()
                .expect("matching repository is open");
            match event {
                load::Event::Files(result) => match result {
                    Ok(mut files)
                        if self.page == Page::Detail
                            && focus == Section::Worktree
                            && matches!(self.mode, Mode::Panel(_)) =>
                    {
                        if let crate::bar::InputBar::Search(editor) = &self.bar {
                            files.query = editor.text.clone();
                        }
                        session.files = Panel::Ready(files.clone());
                        self.mode = Mode::Files(files);
                    }
                    Ok(files) => session.files = Panel::Ready(files),
                    Err(error) => {
                        if self.page == Page::Detail && focus == Section::Worktree {
                            self.message = error.clone();
                        }
                        session.files = Panel::Failed(error);
                    }
                },
                load::Event::History(result) => match result {
                    Ok(mut view)
                        if self.page == Page::Detail
                            && focus == Section::History
                            && matches!(self.mode, Mode::Panel(_)) =>
                    {
                        if let crate::bar::InputBar::Search(editor) = &self.bar {
                            view.search(editor.text.clone());
                        }
                        session.history = Panel::Ready(view.session().clone());
                        self.mode = Mode::History(view);
                    }
                    Ok(view) => session.history = Panel::Ready(view.session().clone()),
                    Err(error) => {
                        if self.page == Page::Detail && focus == Section::History {
                            self.message = error.clone();
                        }
                        session.history = Panel::Failed(error);
                    }
                },
            }
        }
        changed
    }
    pub(super) fn park_workspace(&mut self) {
        let mode = self.take_mode();
        self.workspace.save(mode);
    }
    pub(super) fn refresh_workspace(&mut self) {
        if self.page == Page::Detail {
            self.refresh_overview();
            self.workspace.reload(&self.service);
        }
    }
}
