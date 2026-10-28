//! Breadcrumbs describe actual return destinations, including nested content views.
use super::*;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Level {
    Page,
    Repository,
    Pane(crate::workspace::Section),
    History,
    Commit,
    Branches,
    Stashes,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Destination {
    Home,
    Saved { index: usize, level: Level },
    Current(Level),
}
#[derive(Debug)]
pub(crate) struct Crumb {
    pub label: String,
    pub destination: Option<Destination>,
}
pub(super) fn repository(mode: &Mode) -> Option<&Path> {
    match mode {
        Mode::CompositionWaiting(waiting) => Some(&waiting.path),
        Mode::Composition(session) => Some(&session.prepared.snapshot.path),
        Mode::History(view) => Some(&view.session().references.path),
        Mode::Browser(session) => Some(&session.root),
        Mode::Branches(view) => Some(&view.session().snapshot.path),
        Mode::Stashes(view) => Some(&view.session().snapshot.path),
        Mode::Busy(pending) => repository(&pending.origin),
        Mode::Error { origin, .. } => repository(origin),
        _ => mode
            .file_session()
            .map(|session| session.snapshot.path.as_path()),
    }
}
fn entries(page: Page, mode: &Mode, path: Option<&Path>) -> Vec<(String, Level)> {
    if let Mode::Busy(pending) = mode {
        return entries(page, &pending.origin, path);
    }
    if let Mode::Error { origin, title, .. } = mode {
        let mut entries = entries(page, origin, path);
        entries.push((title.clone(), Level::Page));
        return entries;
    }
    let mut entries = Vec::new();
    let mut push = |label: String, level| entries.push((label, level));
    match page {
        Page::Repositories => {}
        Page::Detail => push(
            path.map(crate::view::name)
                .unwrap_or_else(|| i18n::tr!(OverviewTitle).into()),
            Level::Repository,
        ),
        Page::View(view) => push(
            view.title().into(),
            match view {
                View::History => Level::History,
                View::Branches => Level::Branches,
                View::Stashes => Level::Stashes,
                View::Diff | View::Files => Level::Page,
            },
        ),
        Page::Configuration(_) if matches!(mode, Mode::Workflow(_)) => {}
        Page::Configuration(section) => push(section.title().into(), Level::Page),
        Page::Help => push(i18n::tr!(Help).into(), Level::Page),
    }
    if page == Page::Detail
        && let Some(section) = crate::workspace::Section::focused(mode)
    {
        push(section.title().into(), Level::Pane(section));
    }
    match mode {
        Mode::History(view) => {
            let commit = match view.as_ref() {
                history::View::Graph(_) => None,
                history::View::Commit(commit) => Some(commit),
                history::View::Diff { commit, .. } => Some(commit.as_ref()),
            };
            if let Some(commit) = commit {
                push(commit.detail.oid.to_string()[..8].into(), Level::Commit);
                if matches!(view.as_ref(), history::View::Diff { .. }) {
                    let change = commit
                        .files
                        .selected()
                        .and_then(|index| commit.rows().get(index).copied())
                        .expect("historical diff has a selected change");
                    push(
                        change.location.name().to_string_lossy().into_owned(),
                        Level::Page,
                    );
                }
            }
        }
        Mode::Branches(view) => {
            if let branches::View::Detail { entry, .. } = view.as_ref() {
                push(entry.name.clone(), Level::Page);
            }
        }
        Mode::Stashes(view) => {
            if let stashes::View::Detail { entry, .. } = view.as_ref() {
                push(format!("stash@{{{}}}", entry.index), Level::Page);
            }
        }
        Mode::Changes(_) if !matches!(page, Page::View(View::Diff)) => {
            push(i18n::tr!(ChangesTitle).into(), Level::Page)
        }
        Mode::Composition(_) | Mode::CompositionWaiting(_) => {
            push(i18n::tr!(ComposeTitle).into(), Level::Page)
        }
        Mode::CommitMessage { .. } => push(i18n::tr!(CommitStagedContents).into(), Level::Page),
        Mode::PromptEditor(_) => push(i18n::tr!(AgentPrompt).into(), Level::Page),
        Mode::Workflow(session) => push(session.title().into(), Level::Page),
        Mode::PushConfirm { .. } => push(i18n::tr!(PushCurrentBranch).into(), Level::Page),
        _ => {}
    }
    entries
}
impl Screen {
    pub(crate) fn breadcrumbs(&self) -> Vec<Crumb> {
        let mut trail = vec![Crumb {
            label: i18n::tr!(Repositories).into(),
            destination: Some(Destination::Home),
        }];
        let mut previous_repository: Option<&Path> = None;
        for (index, page, mode, saved_path) in self
            .history
            .iter()
            .enumerate()
            .map(|(i, visit)| {
                (
                    Some(i),
                    visit.page,
                    &visit.mode,
                    visit.repository.as_deref(),
                )
            })
            .chain(std::iter::once((
                None,
                self.page,
                &self.mode,
                self.workspace.path(),
            )))
        {
            let path = repository(mode).or(saved_path);
            let mut parts = entries(page, mode, path);
            // 从首页直接打开仓库子视图时，在入口标签中带仓库名，不虚构未进入的概览层。
            if matches!(page, Page::View(_))
                && path != previous_repository
                && let (Some(path), Some((label, _))) = (path, parts.first_mut())
            {
                *label = format!("{} · {label}", crate::view::name(path));
            }
            if matches!(page, Page::Detail | Page::View(_)) {
                previous_repository = path;
            }
            let count = parts.len();
            for (position, (label, level)) in parts.into_iter().enumerate() {
                let destination = match index {
                    Some(index) => Some(Destination::Saved {
                        index,
                        level: if position + 1 == count {
                            Level::Page
                        } else {
                            level
                        },
                    }),
                    None if position + 1 == count => None,
                    None => Some(Destination::Current(level)),
                };
                trail.push(Crumb { label, destination });
            }
        }
        if let Some(last) = trail.last_mut() {
            last.destination = None;
        }
        trail
    }
    pub(crate) fn follow_breadcrumb(&mut self, destination: Destination) {
        if !matches!(
            self.mode,
            Mode::Browsing
                | Mode::Panel(_)
                | Mode::Files(_)
                | Mode::Changes(_)
                | Mode::History(_)
                | Mode::Branches(_)
                | Mode::Stashes(_)
        ) {
            return;
        }
        let level = match destination {
            Destination::Home => {
                while !self.history.is_empty() {
                    self.return_to_view();
                }
                if self.page != Page::Repositories {
                    self.open_repository_list(self.scope_path());
                }
                self.sync_bar();
                return;
            }
            Destination::Saved { index, level } => {
                if index >= self.history.len() {
                    return;
                }
                while self.history.len() > index {
                    self.return_to_view();
                }
                level
            }
            Destination::Current(level) => level,
        };
        match level {
            Level::Page => {}
            Level::Repository => self.leave_repository_section(),
            Level::Pane(section) => self.switch_repository_section(section),
            Level::History => {
                if let Mode::History(view) = &self.mode {
                    self.mode =
                        Mode::History(Box::new(history::View::Graph(view.session().clone())));
                }
            }
            Level::Commit => {
                if let Mode::History(view) = &self.mode
                    && let history::View::Diff { commit, .. } = view.as_ref()
                {
                    self.mode = Mode::History(Box::new(history::View::Commit((**commit).clone())));
                }
            }
            Level::Branches => {
                if let Mode::Branches(view) = &self.mode {
                    self.mode =
                        Mode::Branches(Box::new(branches::View::List(view.session().clone())));
                }
            }
            Level::Stashes => {
                if let Mode::Stashes(view) = &self.mode {
                    self.mode =
                        Mode::Stashes(Box::new(stashes::View::List(view.session().clone())));
                }
            }
        }
        self.message.clear();
        self.sync_bar();
    }
}
