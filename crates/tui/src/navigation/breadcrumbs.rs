//! Breadcrumbs describe actual return destinations, including nested content views.
mod follow;
use super::*;
use crate::features::Maturity;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Level {
    Page,
    PageRoot,
    Repository,
    Pane(crate::workspace::Section),
    History,
    ChangesParent,
    Commit,
    Branches,
    Stashes,
    StashEntry,
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
    pub maturity: Maturity,
}
pub(super) fn repository(mode: &Mode) -> Option<&Path> {
    match mode {
        Mode::StashSave(session) => Some(&session.plan.path),
        Mode::CompositionWaiting(waiting) => Some(&waiting.path),
        Mode::Composition(session) => Some(&session.prepared.snapshot.path),
        Mode::History(view) => Some(&view.session().references.path),
        Mode::Browser(session) => Some(&session.root),
        Mode::Branches(view) => Some(&view.session().snapshot.path),
        Mode::Stashes(view) => Some(&view.session().snapshot.path),
        Mode::Busy(pending) => repository(&pending.origin),
        Mode::Confirm {
            operation: crate::Confirmation::ReplayParents { candidate, .. },
            ..
        } => Some(&candidate.path),
        Mode::Confirm {
            operation: crate::Confirmation::Mutation { plan, .. },
            ..
        } => Some(plan.path()),
        Mode::Error { origin, .. } => repository(origin),
        _ => mode
            .file_session()
            .map(|session| session.snapshot.path.as_path()),
    }
}
fn entries(page: Page, mode: &Mode, path: Option<&Path>) -> Vec<(String, Level, Maturity)> {
    if let Mode::Busy(pending) = mode {
        return entries(page, &pending.origin, path);
    }
    if let Mode::Error { origin, title, .. } = mode {
        let mut entries = entries(page, origin, path);
        entries.push((title.clone(), Level::Page, Maturity::Stable));
        return entries;
    }
    let mut entries = Vec::new();
    let mut push = |label: String, level| entries.push((label, level, Maturity::Stable));
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
                View::Diff if matches!(mode, Mode::Changes(_)) => Level::ChangesParent,
                View::Diff | View::Files => Level::PageRoot,
            },
        ),
        Page::Configuration(_) if matches!(mode, Mode::Workflow(_)) => {}
        Page::Configuration(_) if matches!(mode, Mode::Management(_)) => {
            let Mode::Management(session) = mode else {
                unreachable!()
            };
            push(session.title().into(), Level::PageRoot);
        }
        Page::Configuration(section) => push(section.title().into(), Level::PageRoot),
        Page::Help => push(i18n::tr!(Help).into(), Level::PageRoot),
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
                push(
                    commit.inspect.detail.oid().to_string()[..8].into(),
                    Level::Commit,
                );
                if matches!(view.as_ref(), history::View::Diff { .. }) {
                    let change = commit
                        .inspect
                        .files
                        .selected()
                        .and_then(|index| commit.inspect.rows().get(index).copied())
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
            if !matches!(view.as_ref(), stashes::View::List(_))
                && let Some(entry) = view.selected()
            {
                push(format!("stash@{{{}}}", entry.index), Level::StashEntry);
                if let stashes::View::Diff { inspect, .. } = view.as_ref()
                    && let Some(index) = inspect.selected_index()
                {
                    push(
                        inspect
                            .detail
                            .change(index)
                            .location
                            .name()
                            .to_string_lossy()
                            .into_owned(),
                        Level::Page,
                    );
                }
            }
        }
        Mode::Changes(session) => push(session.active_group().label().into(), Level::Page),
        Mode::Composition(_) | Mode::CompositionWaiting(_) => {
            entries.push((
                i18n::tr!(ComposeTitle).into(),
                Level::Page,
                Maturity::Experimental,
            ));
        }
        Mode::CommitMessage { .. } => push(i18n::tr!(CommitStagedContents).into(), Level::Page),
        Mode::PromptEditor(_) => push(i18n::tr!(AgentPrompt).into(), Level::Page),
        Mode::Workflow(session) => push(session.title().into(), Level::Page),
        Mode::PushConfirm { .. } | Mode::PushTarget(_) => {
            push(i18n::tr!(PushCurrentBranch).into(), Level::Page)
        }
        Mode::StashSave(_) => push(i18n::tr!(StashSaveCommand).into(), Level::Page),
        Mode::Confirm { .. } => push(i18n::tr!(ConfirmOperation).into(), Level::Page),
        _ => {}
    }
    entries
}
impl Screen {
    pub(crate) fn breadcrumbs(&self) -> Vec<Crumb> {
        let mut trail = vec![Crumb {
            label: i18n::tr!(Repositories).into(),
            destination: Some(Destination::Home),
            maturity: Maturity::Stable,
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
                && let (Some(path), Some((label, _, _))) = (path, parts.first_mut())
            {
                *label = format!("{} · {label}", crate::view::name(path));
            }
            if matches!(page, Page::Detail | Page::View(_)) {
                previous_repository = path;
            }
            if page == Page::View(View::Diff)
                && parts
                    .first()
                    .is_some_and(|(label, _, _)| label == i18n::tr!(Worktree))
                && trail
                    .last()
                    .is_some_and(|crumb| crumb.label == i18n::tr!(Worktree))
            {
                parts.remove(0);
            }
            let count = parts.len();
            for (position, (label, level, maturity)) in parts.into_iter().enumerate() {
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
                trail.push(Crumb {
                    label,
                    destination,
                    maturity,
                });
            }
        }
        if let Some(last) = trail.last_mut() {
            last.destination = None;
        }
        if matches!(self.mode, Mode::Busy(_)) {
            for crumb in &mut trail {
                crumb.destination = None;
            }
        }
        trail
    }
}
