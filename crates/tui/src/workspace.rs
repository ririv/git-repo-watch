//! Simultaneous repository panes share existing file and history interaction states.
mod load;
use crate::{FileSession, Mode, Page, Screen, Service, branches, history};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Section {
    Overview,
    Branches,
    Worktree,
    History,
}
impl Section {
    pub const ALL: [Self; 4] = [
        Self::Overview,
        Self::Branches,
        Self::Worktree,
        Self::History,
    ];
    pub fn title(self) -> &'static str {
        match self {
            Self::Overview => i18n::tr!(WorkspaceOverview),
            Self::Branches => i18n::tr!(BranchTitle),
            Self::Worktree => i18n::tr!(Worktree),
            Self::History => i18n::tr!(WorkspaceHistory),
        }
    }
    pub fn of(mode: &Mode) -> Self {
        match mode {
            Mode::Panel(section) => *section,
            Mode::Branches(_) => Self::Branches,
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
        Self::ALL[(index + if backwards { Self::ALL.len() - 1 } else { 1 }) % Self::ALL.len()]
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
    pub branches: Panel<branches::Session>,
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
            branches: Panel::Loading,
            history: Panel::Loading,
        }));
        self.loader.request(path, service);
    }
    pub fn save(&mut self, mode: Mode) {
        let Some(session) = self.session_mut() else {
            return;
        };
        match mode {
            Mode::Branches(view) => session.branches = Panel::Ready(view.session().clone()),
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
            Mode::Branches(view) => matches!(view.as_ref(), branches::View::List(_)),
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
                    | Mode::Branches(_)
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
            Section::Branches => match &session.branches {
                Panel::Ready(branches) => {
                    Mode::Branches(Box::new(branches::View::List(branches.clone())))
                }
                _ => Mode::Panel(section),
            },
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
                load::Event::Branches(result) => match result {
                    Ok(mut branches)
                        if self.page == Page::Detail
                            && focus == Section::Branches
                            && (matches!(self.mode, Mode::Panel(_))
                                || matches!(&self.mode, Mode::Branches(view) if matches!(view.as_ref(), branches::View::List(_)))) =>
                    {
                        if let Mode::Branches(current) = &self.mode {
                            let previous = current.session();
                            let selected = previous.selected().map(|entry| entry.reference.clone());
                            branches.filter = previous.filter;
                            branches.query = previous.query.clone();
                            let index = selected.and_then(|reference| {
                                branches
                                    .rows()
                                    .iter()
                                    .position(|entry| entry.reference == reference)
                            });
                            branches.state.select(
                                index.or_else(|| (!branches.rows().is_empty()).then_some(0)),
                            );
                        }
                        if let crate::bar::InputBar::Search(editor) = &self.bar {
                            branches.search(editor.text.clone());
                        }
                        session.branches = Panel::Ready(branches.clone());
                        self.mode = Mode::Branches(Box::new(branches::View::List(branches)));
                    }
                    Ok(branches) => session.branches = Panel::Ready(branches),
                    Err(error) => {
                        session.branches = Panel::Failed(error);
                    }
                },
                load::Event::Files(result) => match result {
                    Ok(mut files)
                        if self.page == Page::Detail
                            && focus == Section::Worktree
                            && matches!(self.mode, Mode::Panel(_) | Mode::Files(_)) =>
                    {
                        if let Mode::Files(current) = &self.mode {
                            files = current.refreshed(files.snapshot);
                        }
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
                            && (matches!(self.mode, Mode::Panel(_))
                                || matches!(&self.mode, Mode::History(current) if matches!(current.as_ref(), history::View::Graph(_)))) =>
                    {
                        if let Mode::History(current) = &self.mode {
                            let previous = current.session();
                            if let history::View::Graph(next) = view.as_mut() {
                                next.query = previous.query.clone();
                                next.pan = previous.pan.min(next.width.saturating_sub(1));
                                let selected = previous.selected().map(|entry| entry.oid);
                                let index = selected.and_then(|oid| {
                                    next.records
                                        .iter()
                                        .position(|record| record.entry.oid == oid)
                                });
                                next.state.select(
                                    index.or_else(|| (!next.records.is_empty()).then_some(0)),
                                );
                            }
                        }
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
