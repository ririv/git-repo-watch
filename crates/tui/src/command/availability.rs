//! Explain command preconditions and selection amounts without reading repositories.
use crate::{Action, Mode, Screen, changes, clipboard};
use engine::local::files::Area;
use std::path::Path;

#[derive(Debug)]
pub(crate) enum Availability {
    Ready,
    Unavailable(Problem),
}
#[derive(Debug)]
pub(crate) enum Problem {
    Reason(String),
    WrongArea(engine::local::files::IndexAction),
}
impl Availability {
    pub fn reason(&self) -> Option<&str> {
        match self {
            Self::Ready => None,
            Self::Unavailable(Problem::Reason(reason)) => Some(reason),
            Self::Unavailable(Problem::WrongArea(_)) => Some(i18n::tr!(ChangesWrongArea)),
        }
    }
    pub fn label(&self) -> Option<&str> {
        match self {
            Self::Unavailable(Problem::WrongArea(engine::local::files::IndexAction::Stage)) => {
                Some(i18n::tr!(CommandNeedsUnstaged))
            }
            Self::Unavailable(Problem::WrongArea(engine::local::files::IndexAction::Unstage)) => {
                Some(i18n::tr!(CommandNeedsStaged))
            }
            _ => self.reason(),
        }
    }
}
impl Screen {
    pub(crate) fn command_availability(
        &self,
        action: Action,
        path: Option<&Path>,
        selection: &clipboard::Selection,
    ) -> Availability {
        use Action::*;
        if path.is_none()
            && matches!(
                action,
                OpenRepository
                    | Branches
                    | BranchCreate
                    | BranchSwitch
                    | Fetch
                    | Pull
                    | Sync
                    | Stashes
                    | StashSave
                    | StashSaveAll
                    | StashApply
                    | StashPop
                    | StashDrop
                    | Log
                    | Diff
                    | BrowseFiles
                    | Commit
                    | CommitGenerate
                    | Compose(_)
                    | Push
                    | Terminal
                    | Check
                    | Remove
                    | Replay(_)
                    | ReplayControl(_)
            )
        {
            return Availability::Unavailable(Problem::Reason(
                i18n::tr!(SelectARepositoryFirst).into(),
            ));
        }
        if matches!(action, Stage | Unstage) {
            return match self.index_target(action) {
                Ok(_) => Availability::Ready,
                Err(problem) => Availability::Unavailable(problem),
            };
        }
        let reason = match action {
            Copy(scope) => selection.unavailable(scope, path).map(str::to_owned),
            LogMore if !matches!(&self.mode, Mode::History(view) if matches!(view.as_ref(), crate::history::View::Graph(_))) => {
                Some(i18n::tr!(CommandRequiresHistory).into())
            }
            BrowserRefresh | BrowserDiff if !matches!(self.mode, Mode::Browser(_)) => {
                Some(i18n::tr!(CommandRequiresBrowser).into())
            }
            _ => None,
        };
        match reason {
            Some(reason) => Availability::Unavailable(Problem::Reason(reason)),
            None => Availability::Ready,
        }
    }
    pub(crate) fn command_target(
        &self,
        action: Action,
        selection: &clipboard::Selection,
    ) -> Option<String> {
        match action {
            Action::Stage | Action::Unstage => {
                self.index_target(action).ok().map(|target| match target {
                    IndexTarget::Files(count) => i18n::tf!(CommandTargetFiles, count = count),
                    IndexTarget::Lines(count) => i18n::tf!(CommandTargetLines, count = count),
                })
            }
            Action::Copy(scope) => selection.amount(scope).map(|amount| match amount {
                clipboard::Amount::Files(count) => i18n::tf!(CommandTargetFiles, count = count),
                clipboard::Amount::Commits(count) => i18n::tf!(CommandTargetCommits, count = count),
            }),
            Action::Replay(kind) => Some(i18n::tf!(
                CommandTargetCommits,
                count = match kind {
                    engine::operations::replay::Kind::CherryPick => selection.commits().len(),
                    engine::operations::replay::Kind::Revert =>
                        usize::from(selection.commit().is_some()),
                }
            )),
            _ => None,
        }
    }
    fn index_target(&self, action: Action) -> Result<IndexTarget, Problem> {
        let staged = action == Action::Unstage;
        if let Mode::Changes(session) = &self.mode
            && session.focus == changes::Focus::Preview
        {
            return match &session.preview {
                changes::Preview::Ready {
                    editing: changes::edit::State::Ready(editing),
                    ..
                } => {
                    let count = editing.selected().len();
                    if count == 0 {
                        Err(Problem::Reason(i18n::tr!(PartialSelectionIdle).into()))
                    } else if (editing.snapshot.change().area == Area::Staged) != staged {
                        Err(Problem::WrongArea(if staged {
                            engine::local::files::IndexAction::Unstage
                        } else {
                            engine::local::files::IndexAction::Stage
                        }))
                    } else {
                        Ok(IndexTarget::Lines(count))
                    }
                }
                changes::Preview::Ready {
                    editing: changes::edit::State::Unavailable(reason),
                    ..
                } => Err(Problem::Reason(reason.clone())),
                _ => Err(Problem::Reason(i18n::tr!(PartialSelectRequired).into())),
            };
        }
        let files = self
            .mode
            .file_session()
            .ok_or_else(|| Problem::Reason(i18n::tr!(ChangesSelectionRequired).to_owned()))?;
        let files = files.selection();
        if files.is_empty() {
            return Err(Problem::Reason(
                i18n::tr!(NoActionableFilesInThisArea).into(),
            ));
        }
        if files
            .iter()
            .any(|change| (change.area == Area::Staged) != staged)
        {
            return Err(Problem::WrongArea(if staged {
                engine::local::files::IndexAction::Unstage
            } else {
                engine::local::files::IndexAction::Stage
            }));
        }
        Ok(IndexTarget::Files(files.len()))
    }
}
enum IndexTarget {
    Files(usize),
    Lines(usize),
}
