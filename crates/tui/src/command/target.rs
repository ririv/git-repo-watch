//! Commands declare the target they consume. Views resolve it from their current selection.
mod conditions;
mod content;
mod context;
mod execution;
mod history_targets;
mod index_request;
mod resolved;
mod result;
mod selection;
mod worktree;
use super::Action;
use crate::{Mode, Page, Screen, changes, history};
pub(crate) use context::CommandContext;
use engine::local::history::Oid;
pub(crate) use execution::partial::PreparedPartial;
pub(crate) use execution::{Execution, Permit as ExecutionPermit};
use index_request::IndexRequest;
pub(crate) use resolved::Resolved;
pub(crate) use result::{Reason, Target};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy)]
pub(super) enum Requirement {
    Global,
    TaskView,
    Repository,
    EditorDestination,
    DirectoryDestination,
    Clipboard,
    FilePath,
    Search,
    LocalRefresh,
    IndexSelection,
    DiffSelection,
    WorktreeFiles,
    Conflict,
    BranchList,
    WorktreeEntry,
    HistoryGraph,
    ListSelection,
    CommitSelection,
    MergeCommit,
    StashDetail,
    Browser,
    PreviewableFile,
    Composition,
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

pub(super) enum IndexSelection<'a> {
    Files(&'a crate::FileSession),
    Diff(&'a changes::Session),
    Conflict(&'a crate::conflict::View),
}
impl<'a> IndexSelection<'a> {
    pub fn current(screen: &'a Screen) -> Option<Self> {
        match &screen.mode {
            Mode::Changes(session) if session.focus == changes::Focus::Preview => Some(
                screen
                    .conflict_view()
                    .map_or(Self::Diff(session), Self::Conflict),
            ),
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
            | Worktrees
            | WorktreeAdd
            | Submodules
            | SubmoduleAdd
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
            | RepositoryEnv
            | RepositoryEnvEdit(_)
            | Remove
            | ReplayControl(_) => Requirement::Repository,
            OpenEditor => Requirement::EditorDestination,
            OpenMenu | Terminal | OpenFolder => Requirement::DirectoryDestination,
            Copy => Requirement::Clipboard,
            CopyPath => Requirement::FilePath,
            SearchMenu | Search(_) => Requirement::Search,
            Refresh => Requirement::LocalRefresh,
            Stage | Unstage => Requirement::IndexSelection,
            Discard | PartialSelectMenu | PartialSelect(_) => Requirement::DiffSelection,
            DiscardFile => Requirement::WorktreeFiles,
            ConflictMenu | ConflictView(_) | ConflictAccept(_) | ConflictReset | ConflictUndo => {
                Requirement::Conflict
            }
            WorktreeOpen | WorktreeRemove => Requirement::WorktreeEntry,
            BranchFilterMenu | BranchFilter(_) => Requirement::BranchList,
            LogMore => Requirement::HistoryGraph,
            Select(_) => Requirement::ListSelection,
            Replay(_) | ResetMenu | ResetBranch(_) => Requirement::CommitSelection,
            LogParent => Requirement::MergeCommit,
            StashPreviewMenu | StashPreview(_) => Requirement::StashDetail,
            BrowserDiff => Requirement::Browser,
            BrowserLayoutMenu | BrowserLayout(_) | BrowserModeMenu | BrowserMode(_) => {
                Requirement::PreviewableFile
            }
            Compose(_) | ComposeOrderMenu => Requirement::Composition,
            TaskCancel | TaskRetry | TaskFilterMenu | TaskFilter(_) => Requirement::TaskView,
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
            | Repositories
            | RepositorySettings
            | Add
            | Scan
            | Concurrency
            | AutoFetchMenu
            | AutoFetch(_)
            | AutoFetchInterval
            | StartupFetchMenu
            | StartupFetch(_)
            | WrapMenu
            | Wrap(_)
            | Help
            | InteractionMenu
            | Interaction(_)
            | Quit => Requirement::Global,
            #[cfg(feature = "developer-scheduler")]
            Schedule | Interval | Anchor | Autopush | Workflow | Apply | EnableSchedule
            | DisableSchedule | ScheduleEnabledMenu => Requirement::Global,
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
        if self.tasks.session.open() {
            return None;
        }
        match &self.mode {
            Mode::Browsing => match self.page {
                Page::Repositories => self.selected_path(),
                Page::Detail => self.workspace.path().map(Path::to_owned),
                _ => None,
            },
            Mode::Panel(_) => self.workspace.path().map(Path::to_owned),
            Mode::Management(_)
            | Mode::RepositoryEnvironment(_)
            | Mode::Composition(_)
            | Mode::CompositionWaiting(_)
            | Mode::CommitMessage { .. }
            | Mode::Browser(_)
            | Mode::Stashes(_)
            | Mode::Branches(_)
            | Mode::Worktrees(_)
            | Mode::Submodules(_)
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
