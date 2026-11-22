//! Resolved capabilities carry the displayed objects needed by command consumers.
use super::{File, worktree};
use crate::{clipboard, composition, history};
use engine::local::history::Oid;
use std::path::{Path, PathBuf};

pub(crate) struct Destination<'a> {
    pub root: &'a Path,
    pub path: PathBuf,
}
pub(crate) struct Commits<'a> {
    pub repository: &'a Path,
    pub selected: Oid,
    pub marked: Vec<Oid>,
}
pub(crate) enum Composition<'a> {
    Plan(&'a composition::Session),
    Waiting(&'a composition::waiting::Session),
}
pub(crate) struct Conflict<'a> {
    pub view: &'a crate::conflict::View,
    pub resolution: &'a crate::conflict::resolution::Session,
    pub repository: &'a Path,
}
pub(crate) struct Stash<'a> {
    pub session: &'a crate::stashes::Session,
    pub inspect: &'a crate::inspection::Session,
}
pub(crate) struct Resolved<'a> {
    pub(super) value: Value<'a>,
}
pub(super) enum Value<'a> {
    Task(crate::task_center::Operation),
    Global,
    Repository(&'a Path),
    Destination(Destination<'a>),
    Clipboard(clipboard::Scope),
    File(File<'a>),
    Search(crate::search::Support),
    Refresh(crate::view_refresh::Target),
    Index(worktree::Index<'a>),
    Partial(worktree::Partial<'a>),
    WorktreeFiles(worktree::Files<'a>),
    Conflict(Conflict<'a>),
    Branches(&'a crate::branches::Session),
    Worktree(&'a Path, &'a engine::local::worktrees::Entry),
    History(&'a history::Session),
    Selection(super::selection::Selection<'a>),
    Commits(Commits<'a>),
    MergeCommit(&'a history::Commit),
    Stash(Stash<'a>),
    Browser(&'a crate::browser::Session),
    Previewable(&'a crate::browser::opening::OpenTarget),
    Composition(Composition<'a>),
}
impl Resolved<'_> {
    pub(super) fn new(value: Value<'_>) -> Resolved<'_> {
        Resolved { value }
    }
}
impl Resolved<'_> {
    pub fn repository_path(&self) -> Option<&Path> {
        match &self.value {
            Value::Repository(path) => Some(path),
            Value::Commits(target) => Some(target.repository),
            _ => None,
        }
    }
    pub fn is_partial_index(&self) -> bool {
        matches!(self.value, Value::Index(worktree::Index::Partial(_)))
    }
    pub fn label(&self, action: crate::Action, selection: &clipboard::Selection) -> Option<String> {
        match &self.value {
            Value::Refresh(target) => Some(target.label().into()),
            Value::Previewable(target) => {
                use crate::browser::opening::Presentation;
                Some(
                    match action {
                        crate::Action::BrowserModeMenu => match target.presentation {
                            Presentation::Single(mode) => target.opening(mode).label(),
                            Presentation::Split(_) => i18n::tr!(FileSplit),
                        },
                        crate::Action::BrowserLayoutMenu => match target.presentation {
                            Presentation::Single(_) => i18n::tr!(FileSingle),
                            Presentation::Split(_) => i18n::tr!(FileSplit),
                        },
                        _ => return None,
                    }
                    .into(),
                )
            }
            Value::Search(support) if action == crate::Action::SearchMenu => {
                support.mode().map(|mode| mode.label().into())
            }
            Value::Branches(session) if action == crate::Action::BranchFilterMenu => {
                Some(session.filter.title().into())
            }
            Value::Conflict(target) if action == crate::Action::ConflictMenu => {
                Some(target.view.mode.label().into())
            }
            Value::Partial(target) if action == crate::Action::PartialSelectMenu => {
                Some(target.editing.mode().label().into())
            }
            Value::Destination(target) => {
                let label = match target.path.strip_prefix(target.root) {
                    Ok(relative) if !relative.as_os_str().is_empty() => relative,
                    _ => target
                        .root
                        .file_name()
                        .map(Path::new)
                        .unwrap_or(target.root),
                };
                Some(label.display().to_string())
            }
            Value::Worktree(_, entry) => Some(entry.path.display().to_string()),
            Value::Selection(super::selection::Selection::Files(files))
                if action == crate::Action::Select(crate::command::SelectionAction::All) =>
            {
                files.group().map(|group| group.label().into())
            }
            Value::File(target) => Some(target.label()),
            Value::Index(target) => Some(target.label()),
            Value::WorktreeFiles(target) => match target.changes {
                [] => None,
                [change] => Some(change.location.display()),
                files => Some(format!(
                    "{} · {}",
                    crate::files::Group::Unstaged.label(),
                    i18n::tf!(CommandTargetFiles, count = files.len())
                )),
            },
            Value::Clipboard(scope) => selection.amount(*scope).map(|amount| match amount {
                clipboard::Amount::Files(count) => i18n::tf!(CommandTargetFiles, count = count),
                clipboard::Amount::Commits(count) => i18n::tf!(CommandTargetCommits, count = count),
            }),
            Value::Commits(target) => Some(i18n::tf!(
                CommandTargetCommits,
                count = match action {
                    crate::Action::Replay(engine::operations::replay::Kind::Revert) => 1,
                    _ => target.marked.len(),
                }
            )),
            Value::Repository(_)
                if matches!(
                    action,
                    crate::Action::StashSaveMode(crate::stash_save::Scope::Selected)
                ) =>
            {
                Some(i18n::tf!(
                    CommandTargetFiles,
                    count = selection.marked_file_count()
                ))
            }
            _ => None,
        }
    }
}
