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
                    | StashPreviewMenu
                    | StashPreview(_)
                    | StashSave
                    | StashSaveMode(_)
                    | StashApply
                    | StashPop
                    | StashDrop
                    | Log
                    | Diff
                    | BrowseFiles
                    | Commit
                    | CommitGenerate
                    | Compose(_)
                    | ComposeOrderMenu
                    | Push
                    | Terminal
                    | OpenEditor
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
        if let Some(reason) =
            crate::replay::unavailable(path.and_then(|path| self.replay_state(path)), action)
        {
            return Availability::Unavailable(Problem::Reason(reason.into()));
        }
        let reason = match action {
            ConflictView(crate::conflict::DisplayMode::Pair(target)) if matches!(&self.mode, Mode::Changes(session) if matches!(&session.preview,crate::changes::Preview::Ready {content,..} if matches!(content.as_ref(),crate::document::View::Conflict(_)))) =>
            {
                let Mode::Changes(session) = &self.mode else {
                    unreachable!()
                };
                let crate::changes::Preview::Ready { content, .. } = &session.preview else {
                    unreachable!()
                };
                let crate::document::View::Conflict(view) = content.as_ref() else {
                    unreachable!()
                };
                view.pairs.reason(target).map(str::to_owned)
            }
            ConflictMenu | ConflictView(_) if !matches!(&self.mode, Mode::Changes(session) if matches!(&session.preview, crate::changes::Preview::Ready {content,..} if matches!(content.as_ref(), crate::document::View::Conflict(_)))) => {
                Some(i18n::tr!(ConflictFileRequired).into())
            }
            BranchFilterMenu | BranchFilter(_) if !matches!(&self.mode, Mode::Branches(view) if matches!(view.as_ref(), crate::branches::View::List(_))) => {
                Some(i18n::tr!(BranchFilterRequiresView).into())
            }
            LogParent if !matches!(&self.mode, Mode::History(view) if matches!(view.as_ref(), crate::history::View::Commit(commit) if matches!(&commit.inspect.detail, crate::inspection::Source::Commit(detail) if detail.parents.len() > 1))) => {
                Some(i18n::tr!(HistoryParentRequiresMerge).into())
            }
            Refresh => crate::view_refresh::Target::current(self)
                .err()
                .map(str::to_owned),
            Terminal
                if path
                    .and_then(|root| selection.terminal_target(root))
                    .is_none() =>
            {
                Some(i18n::tr!(TerminalSelectionRequired).into())
            }
            OpenEditor
                if path
                    .and_then(|root| selection.editor_target(root))
                    .is_none() =>
            {
                Some(i18n::tr!(ExternalEditorSelectionRequired).into())
            }
            StashPreviewMenu | StashPreview(_) if !matches!(&self.mode, Mode::Stashes(view) if matches!(view.as_ref(), crate::stashes::View::Detail { .. })) => {
                Some(i18n::tr!(StashPreviewRequiresDetail).into())
            }
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
            Replay(_) if selection.commit().is_none() => Some(i18n::tr!(ReplaySelectCommit).into()),
            Copy => match selection.copy_scope() {
                Some(scope) => selection.unavailable(scope, path).map(str::to_owned),
                None => Some(i18n::tr!(CopySelectionRequired).into()),
            },
            LogMore | LogSelect | LogSelection(_) => match &self.mode {
                Mode::History(view) if matches!(view.as_ref(), crate::history::View::Graph(_)) => {
                    if action == LogMore && view.session().cursor().is_none() {
                        Some(i18n::tr!(HistoryNoMore).into())
                    } else if matches!(action, LogSelect) && selection.commit().is_none() {
                        Some(i18n::tr!(ReplaySelectCommit).into())
                    } else {
                        None
                    }
                }
                _ => Some(i18n::tr!(CommandRequiresHistory).into()),
            },
            PartialSelectMenu | PartialSelect(_) => self.partial_target().err(),
            Discard => match &self.mode {
                Mode::Changes(session) if session.focus == changes::Focus::Preview => {
                    match self.partial_target() {
                        Err(reason) => Some(reason),
                        Ok(editing) if editing.selected().is_empty() => {
                            Some(i18n::tr!(PartialSelectionIdle).into())
                        }
                        Ok(editing) if editing.snapshot.change().area == Area::Staged => {
                            Some(i18n::tr!(CommandNeedsUnstaged).into())
                        }
                        Ok(_) => None,
                    }
                }
                _ => Some(i18n::tr!(PartialSelectRequired).into()),
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
            BrowserLayoutMenu | BrowserLayout(_) if selection.opener().is_none() => {
                Some(i18n::tr!(CommandRequiresPreviewableFile).into())
            }
            BrowserDiff if !matches!(self.mode, Mode::Browser(_)) => {
                Some(i18n::tr!(CommandRequiresBrowser).into())
            }
            _ => None,
        };
        match reason {
            Some(reason) => Availability::Unavailable(Problem::Reason(reason)),
            None => Availability::Ready,
        }
    }
    fn partial_target(&self) -> Result<&changes::edit::Session, String> {
        match &self.mode {
            Mode::Changes(session) => match &session.preview {
                changes::Preview::Ready {
                    editing: changes::edit::State::Ready(editing),
                    ..
                } => Ok(editing),
                changes::Preview::Ready {
                    editing: changes::edit::State::Unavailable(reason),
                    ..
                } => Err(reason.clone()),
                _ => Err(i18n::tr!(PartialSelectRequired).into()),
            },
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
