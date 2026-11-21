//! View entry points own navigation; command context follows the focused content.
pub(crate) mod breadcrumbs;
use crate::{
    Effect, FileSession, Mode, Outcome, Page, Screen, branches, changes, files::Group, history,
    stashes,
};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum View {
    Diff,
    History,
    Branches,
    Worktrees,
    Stashes,
    Files,
}
impl View {
    pub fn title(self) -> &'static str {
        match self {
            Self::Diff => i18n::tr!(Worktree),
            Self::History => i18n::tr!(HistoryTitle),
            Self::Branches => i18n::tr!(BranchTitle),
            Self::Worktrees => i18n::tr!(WorktreesTitle),
            Self::Stashes => i18n::tr!(StashTitle),
            Self::Files => i18n::tr!(BrowserTitle),
        }
    }
}
#[derive(Debug)]
pub(crate) enum Content {
    Diff(FileSession, Option<Group>),
    History(history::Session),
    Branches(branches::Session),
    Worktrees(crate::worktrees::Session),
    Stashes(stashes::Session),
    Browser(Box<crate::browser::Session>),
}
impl Content {
    fn view(&self) -> View {
        match self {
            Self::Diff(..) => View::Diff,
            Self::History(_) => View::History,
            Self::Branches(_) => View::Branches,
            Self::Worktrees(_) => View::Worktrees,
            Self::Stashes(_) => View::Stashes,
            Self::Browser(_) => View::Files,
        }
    }
    fn into_mode(self) -> Mode {
        match self {
            Self::Diff(files, group) => Mode::Changes(Box::new(changes::Session::new(
                files,
                changes::ReturnTo::Repositories,
                group,
            ))),
            Self::History(session) => Mode::History(Box::new(history::View::Graph(session))),
            Self::Worktrees(session) => Mode::Worktrees(session),
            Self::Branches(session) => Mode::Branches(Box::new(branches::View::List(session))),
            Self::Stashes(session) => Mode::Stashes(Box::new(stashes::View::List(session))),
            Self::Browser(session) => Mode::Browser(session),
        }
    }
}
#[derive(Debug)]
pub(crate) struct Visit {
    page: Page,
    mode: Mode,
    scroll: usize,
    setting_selected: usize,
    repository: Option<PathBuf>,
}
impl Visit {
    pub(crate) fn observe(&mut self, readback: &crate::mutations::readback::Readback) {
        crate::live::merge::mode(&mut self.mode, readback);
    }
    pub(crate) fn observe_history(&mut self, graph: &history::Session) {
        crate::live::merge::history_mode(&mut self.mode, graph);
    }
    pub(crate) fn complete_composition(
        &mut self,
        path: &std::path::Path,
        prepared: &engine::composition::Prepared,
    ) -> Option<Mode> {
        crate::composition::waiting::complete(&mut self.mode, path, prepared)
    }
    pub(crate) fn fail_composition(&mut self, path: &std::path::Path, detail: &str) {
        crate::composition::waiting::fail(&mut self.mode, path, detail);
    }
    pub(crate) fn refresh_browser(&mut self, path: &std::path::Path) {
        if let Mode::Browser(session) = &mut self.mode
            && session.root == path
        {
            session.refresh();
        }
    }
    pub(crate) fn refresh(&mut self, readback: crate::mutations::readback::Readback) {
        crate::mutations::readback::merge_mode(&mut self.mode, readback);
    }
}
impl Screen {
    pub(crate) fn remember_view(&mut self, mode: Mode) {
        let repository = breadcrumbs::repository(&mode)
            .map(std::path::Path::to_owned)
            .or_else(|| self.scope_path());
        self.history.push(Visit {
            page: self.page,
            mode,
            scroll: self.scroll,
            setting_selected: self.setting_selected,
            repository,
        });
    }
    pub(crate) fn open_page(&mut self, page: Page) {
        if self.page == page && matches!(self.mode, Mode::Browsing) {
            return;
        }
        let origin = self.take_mode();
        self.remember_view(origin);
        self.page = page;
        self.scroll = 0;
        self.setting_selected = 0;
        self.sync_bar();
    }
    pub(crate) fn return_to_view(&mut self) {
        let latest = match &self.mode {
            Mode::Changes(session) => {
                crate::mutations::readback::Readback::Files(session.files.snapshot.as_ref().clone())
            }
            Mode::Files(session) => {
                crate::mutations::readback::Readback::Files(session.snapshot.as_ref().clone())
            }
            Mode::Branches(view) => {
                crate::mutations::readback::Readback::Branches((*view.session().snapshot).clone())
            }
            Mode::Stashes(view) => {
                crate::mutations::readback::Readback::Stashes((*view.session().snapshot).clone())
            }
            _ => crate::mutations::readback::Readback::Unchanged,
        };
        if let Some(visit) = self.history.pop() {
            self.page = visit.page;
            self.mode = visit.mode;
            self.scroll = visit.scroll;
            self.setting_selected = visit.setting_selected;
            if self.page == Page::Detail
                && let Some(path) = visit.repository
                && self.workspace.path() != Some(path.as_path())
            {
                self.enter_repository(path);
                if let Some(session) = self.workspace.session_mut() {
                    session.selection = crate::workspace::Section::of(&self.mode);
                }
            }
        } else {
            self.page = Page::Repositories;
            self.mode = Mode::Browsing;
            self.scroll = 0;
        }
        self.merge_mutation_view(latest);
        if let Mode::Management(session) = &mut self.mode {
            session.refresh(&self.config.repositories);
        }
        if self.page == Page::Detail
            && let Mode::Panel(section) = self.mode
        {
            // 离开期间面板可能已加载完成，返回时采用已完成的快照。
            self.mode = Mode::Browsing;
            self.switch_repository_section(section);
        }
        self.sync_bar();
    }
    pub(crate) fn enter_view(&mut self, content: Content, origin: Mode) {
        let destination = Page::View(content.view());
        // 在同一完整视图重开入口只返回该视图根部，不增加重复返回层。
        if self.page != destination {
            self.remember_view(origin);
        }
        self.page = destination;
        self.mode = content.into_mode();
        self.scroll = 0;
        self.poll_changes();
        self.sync_bar();
    }
    pub(crate) fn open_view(&mut self, view: View, path: PathBuf) {
        self.open_view_target(view, path, None);
    }

    pub(crate) fn open_changes_group(&mut self, path: PathBuf, group: Group) {
        if matches!(&self.mode, Mode::Changes(session) if session.files.snapshot.path == path) {
            self.select_changes_group(group);
        } else {
            self.open_view_target(View::Diff, path, Some(group));
        }
    }

    fn open_changes_parent(&mut self) {
        let Mode::Changes(session) = &self.mode else {
            return;
        };
        let files = session.combined_files();
        let path = files.snapshot.path.clone();
        if self.history.last().is_some_and(|visit| {
            visit.page == Page::Detail && visit.repository.as_ref() == Some(&path)
        }) {
            self.return_to_view();
        } else {
            self.focus(Some(path.clone()));
            self.page = Page::Detail;
            self.mode = Mode::Browsing;
            self.scroll = 0;
            self.enter_repository(path);
            self.workspace.save(Mode::Files(files));
        }
        self.switch_repository_section(crate::workspace::Section::Worktree);
    }

    fn open_view_target(&mut self, view: View, path: PathBuf, group: Option<Group>) {
        if self.page == Page::View(view) && self.scope_path().as_ref() == Some(&path) {
            let at_root = match (&self.mode, view) {
                (Mode::Changes(_), View::Diff) => true,
                (Mode::Browser(_), View::Files) => true,
                (Mode::Worktrees(_), View::Worktrees) => true,
                (Mode::History(current), View::History) => {
                    matches!(current.as_ref(), history::View::Graph(_))
                }
                (Mode::Branches(current), View::Branches) => {
                    matches!(current.as_ref(), branches::View::List(_))
                }
                (Mode::Stashes(current), View::Stashes) => {
                    matches!(current.as_ref(), stashes::View::List(_))
                }
                _ => false,
            };
            if at_root {
                return;
            }
        }
        if view == View::Files {
            let origin = self.take_mode();
            self.enter_view(
                Content::Browser(Box::new(crate::browser::Session::new(path))),
                origin,
            );
            self.poll_browser();
            return;
        }
        // 从当前焦点进入完整视图时复用选择和搜索，布局由入口单独决定。
        let cached = match (&self.mode, view) {
            (Mode::Files(session), View::Diff) if session.snapshot.path == path => {
                Some(Content::Diff(session.clone(), group))
            }
            (Mode::Changes(session), View::Diff) if session.files.snapshot.path == path => {
                Some(Content::Diff(
                    session.combined_files(),
                    group.or(Some(session.active_group())),
                ))
            }
            (Mode::History(current), View::History)
                if current.session().references.path == path =>
            {
                Some(Content::History(current.session().clone()))
            }
            (Mode::Branches(current), View::Branches)
                if current.session().snapshot.path == path =>
            {
                Some(Content::Branches(current.session().clone()))
            }
            (Mode::Stashes(current), View::Stashes) if current.session().snapshot.path == path => {
                Some(Content::Stashes(current.session().clone()))
            }
            _ => None,
        };
        let cached = cached.or_else(|| {
            if self.page != Page::Detail || self.workspace.path() != Some(path.as_path()) {
                return None;
            }
            let session = self.workspace.session()?;
            match view {
                View::Diff => match &session.files {
                    crate::workspace::Panel::Ready(files) => {
                        Some(Content::Diff(files.clone(), group))
                    }
                    _ => None,
                },
                View::History => match &session.history {
                    crate::workspace::Panel::Ready(graph) => Some(Content::History(graph.clone())),
                    _ => None,
                },
                View::Branches => match &session.branches {
                    crate::workspace::Panel::Ready(branches) => {
                        Some(Content::Branches(branches.clone()))
                    }
                    _ => None,
                },
                View::Stashes | View::Files | View::Worktrees => None,
            }
        });
        if let Some(content) = cached {
            let origin = self.take_mode();
            self.enter_view(content, origin);
            return;
        }
        self.start_repository(
            path.clone(),
            view.title(),
            Effect::ReadOnly,
            move |service, cancel, _| {
                let content = match view {
                    View::Diff => {
                        Content::Diff(FileSession::new(service.files(&path, &cancel)?), group)
                    }
                    View::History => Content::History(history::Session::new(
                        service.history(&path, engine::local::history::Request::Start, &cancel)?,
                        &cancel,
                    )?),
                    View::Branches => {
                        Content::Branches(branches::Session::new(service.branches(&path, &cancel)?))
                    }
                    View::Worktrees => Content::Worktrees(crate::worktrees::Session::new(
                        service.worktrees(&path, &cancel)?,
                    )),
                    View::Stashes => {
                        Content::Stashes(stashes::Session::new(service.stashes(&path, &cancel)?))
                    }
                    View::Files => unreachable!("browser opens before the repository task"),
                };
                Ok(Outcome::Opened(Box::new(content)))
            },
        );
    }
}

impl Screen {
    pub(crate) fn hide_disabled_features(&mut self) {
        let allowed = |mode: &Mode| match mode {
            Mode::PromptEditor(_) => self.config.ai.enabled,
            Mode::Composition(_) | Mode::CompositionWaiting(_) => {
                self.config.ai.enabled && self.config.experimental_features
            }
            _ => true,
        };
        self.history.retain(|visit| allowed(&visit.mode));
        if !allowed(&self.mode) {
            self.mode = Mode::Browsing;
            self.sync_bar();
        }
    }
}
