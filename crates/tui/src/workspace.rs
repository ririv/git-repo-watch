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
    #[cfg(test)]
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
        Self::focused(mode).unwrap_or(Self::Overview)
    }
    pub fn focused(mode: &Mode) -> Option<Self> {
        match mode {
            Mode::Panel(section) => Some(*section),
            Mode::Branches(_) => Some(Self::Branches),
            Mode::Files(_) | Mode::Changes(_) | Mode::StashSave(_) | Mode::CommitMessage { .. } => {
                Some(Self::Worktree)
            }
            Mode::History(_) => Some(Self::History),
            Mode::Busy(pending) => Self::focused(&pending.origin),
            Mode::Error { origin, .. } => Self::focused(origin),
            _ => None,
        }
    }
    pub fn towards(self, direction: crate::focus::Direction) -> Self {
        use crate::focus::Direction::*;
        match (self, direction) {
            (Self::Overview, Right) | (Self::History, Up) => Self::Branches,
            (Self::Overview, Down) | (Self::History, Left) => Self::Worktree,
            (Self::Branches, Left) | (Self::Worktree, Up) => Self::Overview,
            (Self::Branches, Down) | (Self::Worktree, Right) => Self::History,
            _ => self,
        }
    }
    #[cfg(test)]
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
    pub selection: Section,
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
    pub fn loading(&self) -> bool {
        self.loader.pending()
    }
    pub fn observe(&mut self, readback: &crate::mutations::readback::Readback) {
        use crate::mutations::readback::Readback;
        let Some(session) = self.session_mut() else {
            return;
        };
        match readback {
            Readback::Files(files) if files.path == session.path => {
                session.files = Panel::Ready(match &session.files {
                    Panel::Ready(current) => current.refreshed(files.clone()),
                    _ => FileSession::new(files.clone()),
                });
            }
            Readback::Branches(branches) if branches.path == session.path => {
                session.branches = Panel::Ready(match &session.branches {
                    Panel::Ready(current) => current.refreshed(branches.clone()),
                    _ => crate::branches::Session::new(branches.clone()),
                });
            }
            _ => {}
        }
    }
    pub fn observe_history(&mut self, graph: &history::Session) {
        let Some(session) = self.session_mut() else {
            return;
        };
        if session.path == graph.references.path {
            session.history = Panel::Ready(match &session.history {
                Panel::Ready(current) => current.refreshed(graph.clone()),
                _ => graph.clone(),
            });
        }
    }
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
            selection: Section::Overview,
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
            Mode::Branches(view) if view.session().snapshot.path == session.path => {
                session.branches = Panel::Ready(view.session().clone())
            }
            Mode::Files(files) if files.snapshot.path == session.path => {
                session.files = Panel::Ready(files)
            }
            Mode::Changes(view) if view.files.snapshot.path == session.path => {
                session.files = Panel::Ready(view.combined_files())
            }
            Mode::History(view) if view.session().references.path == session.path => {
                session.history = Panel::Ready(view.session().clone())
            }
            Mode::StashSave(session) => self.save(*session.origin),
            Mode::Confirm {
                operation:
                    crate::Confirmation::ConflictReset { origin, .. }
                    | crate::Confirmation::DiscardFile { origin, .. }
                    | crate::Confirmation::Partial { origin, .. }
                    | crate::Confirmation::Configure { origin, .. }
                    | crate::Confirmation::Mutation { origin, .. }
                    | crate::Confirmation::ReplayParents { origin, .. },
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
    pub(crate) fn open_repository(&mut self, path: PathBuf) {
        if self.page == Page::Detail {
            self.park_workspace();
        } else {
            let origin = self.take_mode();
            self.remember_view(origin);
        }
        if let Some(index) = self.rows().iter().position(|row| row.path == path) {
            self.select(index);
        }
        self.page = Page::Detail;
        self.mode = Mode::Browsing;
        self.scroll = 0;
        self.enter_repository(path);
        self.sync_bar();
    }
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
                    | Mode::Changes(_)
                    | Mode::History(_)
            )
        {
            return;
        }
        if Section::focused(&self.mode) == Some(section) && Workspace::is_root(&self.mode) {
            return;
        }
        let mode = self.take_mode();
        self.workspace.save(mode);
        let Some(session) = self.workspace.session_mut() else {
            return;
        };
        session.selection = section;
        self.mode = match section {
            Section::Overview => Mode::Panel(Section::Overview),
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
    pub(super) fn leave_repository_section(&mut self) {
        // 面板焦点是一层导航；放大只是布局，返回时一并恢复组合界面。
        let mode = self.take_mode();
        self.workspace.save(mode);
        if let Some(session) = self.workspace.session_mut() {
            session.arrangement = Arrangement::Tiled;
        }
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
            let focus = Section::focused(&self.mode);
            let session = self
                .workspace
                .session_mut()
                .expect("matching repository is open");
            match event {
                load::Event::Branches(result) => match result {
                    Ok(mut branches)
                        if self.page == Page::Detail
                            && focus == Some(Section::Branches)
                            && (matches!(self.mode, Mode::Panel(_))
                                || matches!(&self.mode, Mode::Branches(view) if matches!(view.as_ref(), branches::View::List(_)))) =>
                    {
                        if let Mode::Branches(current) = &self.mode {
                            branches = current.session().refreshed((*branches.snapshot).clone());
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
                load::Event::Files(result) => match result.map(|files| *files) {
                    Ok(files)
                        if self.page == Page::Detail
                            && matches!(&self.mode, Mode::Changes(current) if current.files.snapshot.path == path) =>
                    {
                        if let Mode::Changes(current) = &mut self.mode {
                            current.refreshed(files.snapshot);
                            session.files = Panel::Ready(current.combined_files());
                        }
                    }
                    Ok(mut files)
                        if self.page == Page::Detail
                            && focus == Some(Section::Worktree)
                            && matches!(self.mode, Mode::Panel(_) | Mode::Files(_)) =>
                    {
                        if let Mode::Files(current) = &self.mode {
                            files = current.refreshed(files.snapshot);
                        }
                        if let crate::bar::InputBar::Search(editor) = &self.bar {
                            files.search(editor.text.clone());
                        }
                        session.files = Panel::Ready(files.clone());
                        self.mode = Mode::Files(files);
                    }
                    Ok(files) => session.files = Panel::Ready(files),
                    Err(error) => {
                        if self.page == Page::Detail && focus == Some(Section::Worktree) {
                            self.message = crate::message::Message::error(error.clone());
                        }
                        session.files = Panel::Failed(error);
                    }
                },
                load::Event::History(result) => match result {
                    Ok(mut view)
                        if self.page == Page::Detail
                            && focus == Some(Section::History)
                            && (matches!(self.mode, Mode::Panel(_))
                                || matches!(&self.mode, Mode::History(current) if matches!(current.as_ref(), history::View::Graph(_)))) =>
                    {
                        if let Mode::History(current) = &self.mode {
                            let previous = current.session();
                            if let history::View::Graph(next) = view.as_mut() {
                                next.query = previous.query.clone();
                                next.horizontal = previous.horizontal.clone();
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
                        if self.page == Page::Detail && focus == Some(Section::History) {
                            self.message = crate::message::Message::error(error.clone());
                        }
                        session.history = Panel::Failed(error);
                    }
                },
            }
        }
        if changed {
            self.sync_plain_bar();
        }
        changed
    }
    pub(super) fn park_workspace(&mut self) {
        let mode = self.take_mode();
        self.workspace.save(mode);
    }
    pub(super) fn refresh_workspace(&mut self) {
        if let Some(path) = self.workspace.path().map(std::path::Path::to_owned) {
            self.overview.open(path, &self.service);
            self.workspace.reload(&self.service);
        }
    }
}
