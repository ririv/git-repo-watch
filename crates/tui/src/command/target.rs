//! Commands declare the target they consume. Views resolve it from their current selection.
use super::Action;
use crate::{Mode, Page, Screen, changes, clipboard, history};
use engine::local::history::Oid;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy)]
pub(super) enum Requirement {
    Global,
    Repository,
    EditorDestination,
    TerminalDirectory,
    Clipboard,
    FilePath,
    Search,
    LocalRefresh,
    IndexSelection,
    DiffSelection,
    WorktreeFiles,
    Conflict,
    BranchList,
    HistoryGraph,
    HistorySelection,
    CommitSelection,
    MergeCommit,
    StashDetail,
    Browser,
    PreviewableFile,
    Composition,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum Status {
    Present,
    Unavailable(&'static str),
    OutOfScope(&'static str),
}

impl Status {
    fn requiring(supported: bool, missing: Self) -> Self {
        if supported { Self::Present } else { missing }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct HistoricalFile {
    pub object: Oid,
    pub path: PathBuf,
}

pub(crate) enum File<'a> {
    Working(&'a crate::working_target::Target),
    Historical(&'a HistoricalFile),
}
impl File<'_> {
    pub fn relative_path(&self) -> &Path {
        match self {
            Self::Working(target) => target.relative_path(),
            Self::Historical(target) => &target.path,
        }
    }
    pub fn label(&self) -> String {
        match self {
            Self::Working(_) => self.relative_path().display().to_string(),
            Self::Historical(target) => {
                format!(
                    "{} · {:.8}",
                    target.path.display(),
                    target.object.to_string()
                )
            }
        }
    }
}

pub(crate) enum IndexSelection<'a> {
    Files(&'a crate::FileSession),
    Diff(&'a changes::Session),
}
impl<'a> IndexSelection<'a> {
    pub fn current(screen: &'a Screen) -> Option<Self> {
        match &screen.mode {
            Mode::Changes(session) if session.focus == changes::Focus::Preview => {
                Some(Self::Diff(session))
            }
            Mode::Changes(session) => Some(Self::Files(&session.files)),
            Mode::Files(files) => Some(Self::Files(files)),
            _ => None,
        }
    }
}

impl Action {
    pub(super) fn requirement(self) -> Requirement {
        use crate::composition::Action as ComposeAction;
        use Action::*;
        match self {
            OpenRepository
            | Branches
            | BranchCreate
            | BranchSwitch
            | Fetch
            | Pull
            | Sync
            | Stashes
            | StashSave
            | StashSaveMode(_)
            | StashApply
            | StashPop
            | StashDrop
            | Log
            | Diff
            | DiffGroup(_)
            | BrowseFiles
            | Commit
            | CommitGenerate
            | Compose(ComposeAction::Open)
            | Push
            | Remove
            | ReplayControl(_) => Requirement::Repository,
            OpenEditor => Requirement::EditorDestination,
            Terminal => Requirement::TerminalDirectory,
            Copy => Requirement::Clipboard,
            CopyPath => Requirement::FilePath,
            SearchMenu | Search(_) => Requirement::Search,
            Refresh => Requirement::LocalRefresh,
            Stage | Unstage => Requirement::IndexSelection,
            Discard | PartialSelectMenu | PartialSelect(_) => Requirement::DiffSelection,
            DiscardFile => Requirement::WorktreeFiles,
            ConflictMenu | ConflictView(_) => Requirement::Conflict,
            BranchFilterMenu | BranchFilter(_) => Requirement::BranchList,
            LogMore => Requirement::HistoryGraph,
            LogSelect | LogSelection(_) => Requirement::HistorySelection,
            Replay(_) => Requirement::CommitSelection,
            LogParent => Requirement::MergeCommit,
            StashPreviewMenu | StashPreview(_) => Requirement::StashDetail,
            BrowserDiff => Requirement::Browser,
            BrowserLayoutMenu | BrowserLayout(_) | BrowserModeMenu | BrowserMode(_) => {
                Requirement::PreviewableFile
            }
            Compose(_) | ComposeOrderMenu => Requirement::Composition,
            Tasks
            | Focus
            | Zoom
            | FileLayoutMenu
            | FileLayout(_)
            | DiffLayoutMenu
            | DiffHeader
            | DiffHeaderVisibility(_)
            | DiffContextMenu
            | DiffContext(_)
            | DiffLayout(_)
            | FilterMenu
            | Filter(_)
            | SortMenu
            | Sort(_)
            | AiAgent
            | AiModel
            | AiPrompt
            | EditorSetting
            | FetchAll
            | CancelCheck
            | Settings
            | AiSettings
            | Schedule
            | Repositories
            | RepositorySettings
            | Add
            | Scan
            | Interval
            | Anchor
            | Autopush
            | Concurrency
            | Workflow
            | AutoFetchMenu
            | AutoFetch(_)
            | AutoFetchInterval
            | StartupFetchMenu
            | StartupFetch(_)
            | WrapMenu
            | Wrap(_)
            | Apply
            | EnableSchedule
            | DisableSchedule
            | ScheduleEnabledMenu
            | Help
            | InteractionMenu
            | Interaction(_)
            | Quit => Requirement::Global,
        }
    }
}

impl Screen {
    pub(crate) fn conflict_view(&self) -> Option<&crate::conflict::View> {
        match &self.mode {
            Mode::Changes(session) => match &session.preview {
                changes::Preview::Ready { content, .. } => match content.as_ref() {
                    crate::document::View::Conflict(view) => Some(view),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        }
    }

    pub(crate) fn command_repository(&self) -> Option<PathBuf> {
        match &self.mode {
            Mode::Browsing => match self.page {
                Page::Repositories => self.selected_path(),
                Page::Detail => self.workspace.path().map(Path::to_owned),
                _ => None,
            },
            Mode::Panel(_) => self.workspace.path().map(Path::to_owned),
            Mode::Management(_)
            | Mode::Composition(_)
            | Mode::CompositionWaiting(_)
            | Mode::CommitMessage { .. }
            | Mode::Browser(_)
            | Mode::Stashes(_)
            | Mode::Branches(_)
            | Mode::History(_)
            | Mode::Changes(_)
            | Mode::Files(_) => self.scope_path(),
            _ => None,
        }
    }

    pub(crate) fn worktree_file_list(&self) -> Option<&crate::FileSession> {
        match &self.mode {
            Mode::Files(files) => Some(files),
            Mode::Changes(session) if session.focus == changes::Focus::Files => {
                Some(&session.files)
            }
            _ => None,
        }
    }

    pub(super) fn diff_selection_target(&self) -> Option<&changes::Session> {
        match &self.mode {
            Mode::Changes(session)
                if session.focus == changes::Focus::Preview && session.previews_file_diff() =>
            {
                Some(session)
            }
            _ => None,
        }
    }

    pub(super) fn target_status(
        &self,
        action: Action,
        repository: Option<&Path>,
        selection: &clipboard::Selection,
    ) -> Status {
        use Requirement::*;
        use Status::*;
        match action.requirement() {
            Global => Present,
            Repository => Status::requiring(
                repository.is_some(),
                Unavailable(i18n::tr!(SelectARepositoryFirst)),
            ),
            EditorDestination => Status::requiring(
                repository
                    .and_then(|root| selection.editor_target(root))
                    .is_some(),
                Unavailable(i18n::tr!(ExternalEditorSelectionRequired)),
            ),
            TerminalDirectory => Status::requiring(
                repository
                    .and_then(|root| selection.terminal_target(root))
                    .is_some(),
                Unavailable(i18n::tr!(TerminalSelectionRequired)),
            ),
            Clipboard => Status::requiring(
                selection.copy_scope().is_some(),
                Unavailable(i18n::tr!(CopySelectionRequired)),
            ),
            FilePath => Status::requiring(
                selection.file_target().is_some(),
                OutOfScope(i18n::tr!(CommandRequiresFileTarget)),
            ),
            Search => Status::requiring(
                self.search_support().admits(action),
                OutOfScope(i18n::tr!(SearchUnavailable)),
            ),
            LocalRefresh => match crate::view_refresh::Target::current(self) {
                Ok(_) => Present,
                Err(reason) => Unavailable(reason),
            },
            IndexSelection => {
                let Some(target) = self::IndexSelection::current(self) else {
                    return Unavailable(i18n::tr!(ChangesSelectionRequired));
                };
                let group = match target {
                    self::IndexSelection::Files(files) => files.scope().or_else(|| files.group()),
                    self::IndexSelection::Diff(session) => Some(session.active_group()),
                };
                match (group, action) {
                    (Some(crate::files::Group::Staged), Action::Stage) => {
                        OutOfScope(i18n::tr!(CommandNeedsUnstaged))
                    }
                    (Some(crate::files::Group::Unstaged), Action::Unstage) => {
                        OutOfScope(i18n::tr!(CommandNeedsStaged))
                    }
                    _ => Present,
                }
            }
            DiffSelection => match self.diff_selection_target() {
                Some(session) if !self.tasks.session.open() => {
                    if action == Action::Discard
                        && session.active_group() == crate::files::Group::Staged
                    {
                        OutOfScope(i18n::tr!(CommandNeedsUnstaged))
                    } else {
                        Present
                    }
                }
                _ => OutOfScope(i18n::tr!(CommandRequiresWorktreePreview)),
            },
            WorktreeFiles if self.tasks.session.open() || self.worktree_file_list().is_none() => {
                OutOfScope(i18n::tr!(DiscardFileSelect))
            }
            WorktreeFiles
                if self
                    .worktree_file_list()
                    .and_then(|files| files.scope().or_else(|| files.group()))
                    == Some(crate::files::Group::Staged) =>
            {
                OutOfScope(i18n::tr!(CommandNeedsUnstaged))
            }
            WorktreeFiles => Status::requiring(
                !selection.worktree_files().is_empty(),
                Unavailable(i18n::tr!(DiscardFileSelect)),
            ),
            Conflict => Status::requiring(
                self.conflict_view().is_some(),
                OutOfScope(i18n::tr!(ConflictFileRequired)),
            ),
            BranchList => Status::requiring(
                matches!(&self.mode, Mode::Branches(view)
                    if matches!(view.as_ref(), crate::branches::View::List(_))),
                Unavailable(i18n::tr!(BranchFilterRequiresView)),
            ),
            HistoryGraph => Status::requiring(
                self.history_graph_target().is_some(),
                Unavailable(i18n::tr!(CommandRequiresHistory)),
            ),
            HistorySelection => selection.history_selection_status(),
            CommitSelection => match selection.commit_selection_status() {
                Present if repository.is_none() => Unavailable(i18n::tr!(SelectARepositoryFirst)),
                status => status,
            },
            MergeCommit => Status::requiring(
                matches!(&self.mode, Mode::History(view)
                    if matches!(view.as_ref(), history::View::Commit(commit)
                        if matches!(&commit.inspect.detail, crate::inspection::Source::Commit(detail)
                            if detail.parents.len() > 1))),
                Unavailable(i18n::tr!(HistoryParentRequiresMerge)),
            ),
            StashDetail => Status::requiring(
                matches!(&self.mode, Mode::Stashes(view)
                    if matches!(view.as_ref(), crate::stashes::View::Detail { .. })),
                Unavailable(i18n::tr!(StashPreviewRequiresDetail)),
            ),
            Browser => Status::requiring(
                matches!(self.mode, Mode::Browser(_)),
                Unavailable(i18n::tr!(CommandRequiresBrowser)),
            ),
            PreviewableFile => Status::requiring(
                selection.opener().is_some(),
                OutOfScope(i18n::tr!(CommandRequiresPreviewableFile)),
            ),
            Composition => Status::requiring(
                matches!(&self.mode, Mode::Composition(session)
                    if matches!(session.state, crate::composition::State::Browsing))
                    || matches!(
                        (&self.mode, action),
                        (
                            Mode::CompositionWaiting(_),
                            Action::Compose(crate::composition::Action::Regenerate)
                        )
                    ),
                Unavailable(i18n::tr!(ComposeSelected)),
            ),
        }
    }

    pub(crate) fn history_graph_target(&self) -> Option<&history::Session> {
        match &self.mode {
            Mode::History(view) => match view.as_ref() {
                history::View::Graph(session) => Some(session),
                _ => None,
            },
            _ => None,
        }
    }
}
