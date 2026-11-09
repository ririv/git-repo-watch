//! Explain command preconditions and selection amounts without reading repositories.
use super::target::{IndexSelection, Status};
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
        if let Status::Unavailable(reason) | Status::OutOfScope(reason) =
            self.target_status(action, path, selection)
        {
            return Availability::Unavailable(Problem::Reason(reason.into()));
        }
        if matches!(action, Stage | Unstage) {
            return match self.index_target(action) {
                Ok(_) => Availability::Ready,
                Err(problem) => Availability::Unavailable(problem),
            };
        }
        if let Some(reason) =
            crate::replay::unavailable(path.and_then(|path| self.replay_state(path)), action)
        {
            return Availability::Unavailable(Problem::Reason(reason.into()));
        }
        let reason = match action {
            ConflictView(crate::conflict::DisplayMode::Pair(target)) => self
                .conflict_view()
                .and_then(|view| view.pairs.reason(target))
                .map(str::to_owned),
            CopyPath => selection
                .file_target()
                .filter(|target| target.relative_path().to_str().is_none())
                .map(|_| i18n::tr!(CopyPathNotUtf8).into()),
            StashSaveMode(crate::stash_save::Scope::Selected)
                if selection.marked_file_count() == 0 =>
            {
                Some(i18n::tr!(StashSelectFiles).into())
            }
            ComposeOrderMenu
            | Compose(crate::composition::Action::Up | crate::composition::Action::Down) => {
                match &self.mode {
                    Mode::Composition(session)
                        if matches!(session.state, crate::composition::State::Browsing) =>
                    {
                        match action {
                            Compose(crate::composition::Action::Up) if session.selected == 0 => {
                                Some(i18n::tr!(ComposeFirstStep).into())
                            }
                            Compose(crate::composition::Action::Down)
                                if session.selected + 1 >= session.prepared.plan.commits.len() =>
                            {
                                Some(i18n::tr!(ComposeLastStep).into())
                            }
                            _ => None,
                        }
                    }
                    _ => Some(i18n::tr!(ComposeSelected).into()),
                }
            }
            Copy => match selection.copy_scope() {
                Some(scope) => selection.unavailable(scope, path).map(str::to_owned),
                None => Some(i18n::tr!(CopySelectionRequired).into()),
            },
            LogMore
                if self
                    .history_graph_target()
                    .is_some_and(|graph| graph.cursor().is_none()) =>
            {
                Some(i18n::tr!(HistoryNoMore).into())
            }
            LogSelect | LogSelection(_) if self.history_graph_target().is_none() => {
                Some(i18n::tr!(CommandRequiresHistory).into())
            }
            LogSelect if selection.commit().is_none() => Some(i18n::tr!(ReplaySelectCommit).into()),
            PartialSelectMenu | PartialSelect(_) => self.partial_target().err(),
            Discard => match self.partial_target() {
                Err(reason) => Some(reason),
                Ok(editing) if editing.selected().is_empty() => {
                    Some(i18n::tr!(PartialSelectionIdle).into())
                }
                Ok(editing) if editing.snapshot.change().area == Area::Staged => {
                    Some(i18n::tr!(CommandNeedsUnstaged).into())
                }
                Ok(editing)
                    if Some(editing.snapshot.files().path.as_path()) != path
                        || self
                            .diff_selection_target()
                            .and_then(|target| target.selection_key())
                            .as_ref()
                            != selection.diff_selection() =>
                {
                    Some(i18n::tr!(PartialChanged).into())
                }
                Ok(_) => None,
            },
            BrowserMode(mode) => {
                use crate::browser::opening::{Presentation, TextMode};
                match selection.opener().map(|target| target.presentation) {
                    Some(Presentation::Single(current)) if current == mode => Some(match mode {
                        TextMode::Source => i18n::tr!(CommandSourceCurrent).into(),
                        TextMode::Preview => i18n::tr!(CommandPreviewCurrent).into(),
                    }),
                    Some(_) => None,
                    None => Some(i18n::tr!(CommandRequiresPreviewableFile).into()),
                }
            }
            _ => None,
        };
        match reason {
            Some(reason) => Availability::Unavailable(Problem::Reason(reason)),
            None => Availability::Ready,
        }
    }
    fn partial_target(&self) -> Result<&changes::edit::Session, String> {
        match self.diff_selection_target().map(|target| &target.preview) {
            Some(changes::Preview::Ready {
                editing: changes::edit::State::Ready(editing),
                ..
            }) => Ok(editing),
            Some(changes::Preview::Ready {
                editing: changes::edit::State::Unavailable(reason),
                ..
            }) => Err(reason.clone()),
            _ => Err(i18n::tr!(PartialSelectRequired).into()),
        }
    }
    pub(crate) fn command_target(
        &self,
        action: Action,
        path: Option<&Path>,
        selection: &clipboard::Selection,
    ) -> Option<String> {
        match action {
            Action::Refresh => crate::view_refresh::Target::current(self)
                .ok()
                .map(|target| target.label().into()),
            Action::OpenEditor | Action::Terminal => {
                let root = path?;
                let target = match action {
                    Action::OpenEditor => selection.editor_target(root)?,
                    Action::Terminal => selection.terminal_target(root)?,
                    _ => unreachable!(),
                };
                let label = match target.strip_prefix(root) {
                    Ok(relative) if !relative.as_os_str().is_empty() => relative,
                    _ => root.file_name().map(Path::new).unwrap_or(root),
                };
                Some(label.display().to_string())
            }
            Action::CopyPath => selection.file_target().map(|target| target.label()),
            Action::StashSaveMode(crate::stash_save::Scope::Selected) => Some(i18n::tf!(
                CommandTargetFiles,
                count = selection.marked_file_count()
            )),
            Action::Stage | Action::Unstage => {
                self.index_target(action).ok().map(|target| match target {
                    IndexTarget::Files(count) => i18n::tf!(CommandTargetFiles, count = count),
                    IndexTarget::Lines(count) => i18n::tf!(CommandTargetLines, count = count),
                })
            }
            Action::Copy => selection
                .copy_scope()
                .and_then(|scope| selection.amount(scope))
                .map(|amount| match amount {
                    clipboard::Amount::Files(count) => i18n::tf!(CommandTargetFiles, count = count),
                    clipboard::Amount::Commits(count) => {
                        i18n::tf!(CommandTargetCommits, count = count)
                    }
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
        let files = match IndexSelection::current(self) {
            Some(IndexSelection::Files(files)) => files,
            Some(IndexSelection::Diff(session)) => {
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
            None => return Err(Problem::Reason(i18n::tr!(ChangesSelectionRequired).into())),
        };
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
