//! Every command action supplies a concise candidate description.
use super::{Action, Command, SelectionAction};
use i18n::CommandSummary as Summary;
impl Command {
    pub fn summary(&self) -> Summary {
        match self.action {
            Action::ResetBranch(engine::operations::reset::Mode::Soft) => Summary::ResetBranch,
            Action::ResetBranch(engine::operations::reset::Mode::Hard) => Summary::ResetHard,
            Action::ConflictMenu => Summary::ConflictGroup,
            Action::ConflictView(_) => Summary::Conflict,
            Action::ConflictAccept(_) => Summary::ConflictAccept,
            Action::ConflictUndo => Summary::ConflictUndo,
            Action::ConflictReset => Summary::ConflictReset,
            Action::PartialSelectMenu | Action::PartialSelect(_) => Summary::PartialSelect,
            Action::Discard => Summary::PartialDiscard,
            Action::DiscardFile => Summary::DiscardFile,
            Action::SearchMenu => Summary::SearchMode,
            Action::Search(crate::search::Mode::Filter) => Summary::SearchFilter,
            Action::Search(crate::search::Mode::Locate) => Summary::SearchLocate,
            Action::Tasks => Summary::Tasks,
            Action::TaskCancel => Summary::TaskCancel,
            Action::TaskRetry => Summary::TaskRetry,
            Action::TaskFilterMenu | Action::TaskFilter(_) => Summary::TaskFilter,
            Action::Focus => Summary::PaneFocus,
            Action::Zoom => Summary::PaneZoom,
            Action::Branches => Summary::Branches,
            Action::Worktrees => Summary::Worktrees,
            Action::WorktreeAdd => Summary::WorktreeAdd,
            Action::WorktreeOpen => Summary::WorktreeOpen,
            Action::WorktreeRemove => Summary::WorktreeRemove,
            Action::BranchFilterMenu | Action::BranchFilter(_) => Summary::BranchFilter,
            Action::LogParent => Summary::HistoryParent,
            Action::BranchCreate => Summary::BranchCreate,
            Action::BranchSwitch => Summary::BranchSwitch,
            Action::Fetch => Summary::Fetch,
            Action::Pull => Summary::Pull,
            Action::Sync => Summary::Sync,
            Action::Stashes => Summary::Stashes,
            Action::StashPreviewMenu | Action::StashPreview(_) => Summary::StashPreview,
            Action::StashSave | Action::StashSaveMode(_) => Summary::StashSave,
            Action::StashApply => Summary::StashApply,
            Action::StashPop => Summary::StashPop,
            Action::StashDrop => Summary::StashDrop,
            Action::Stage => Summary::Stage,
            Action::Unstage => Summary::Unstage,
            Action::Diff => Summary::Diff,
            Action::DiffGroup(crate::files::Group::Unstaged) => Summary::DiffUnstaged,
            Action::DiffGroup(crate::files::Group::Staged) => Summary::DiffStaged,
            Action::BrowseFiles => Summary::BrowseFiles,
            Action::BrowserDiff => Summary::BrowserDiff,
            Action::BrowserLayoutMenu | Action::BrowserLayout(_) => Summary::BrowserLayout,
            Action::BrowserModeMenu => Summary::BrowserMode,
            Action::BrowserMode(crate::browser::opening::TextMode::Source) => {
                Summary::BrowserSource
            }
            Action::BrowserMode(crate::browser::opening::TextMode::Preview) => {
                Summary::BrowserPreview
            }
            Action::Log => Summary::Log,
            Action::LogMore => Summary::LogMore,
            Action::Select(action) => match action {
                SelectionAction::Toggle => Summary::SelectionToggle,
                SelectionAction::All => Summary::SelectionAll,
                SelectionAction::Clear => Summary::SelectionClear,
            },
            Action::Replay(engine::operations::replay::Kind::CherryPick) => {
                Summary::ReplayCherryPick
            }
            Action::Replay(engine::operations::replay::Kind::Revert) => Summary::ReplayRevert,
            Action::ReplayControl(engine::operations::replay::Control::Continue) => {
                Summary::ReplayContinue
            }
            Action::ReplayControl(engine::operations::replay::Control::Skip) => Summary::ReplaySkip,
            Action::ReplayControl(engine::operations::replay::Control::Abort) => {
                Summary::ReplayAbort
            }
            Action::FileLayoutMenu => Summary::FileLayoutMenu,
            Action::DiffLayoutMenu => Summary::DiffLayoutMenu,
            Action::DiffHeader | Action::DiffHeaderVisibility(_) => Summary::DiffHeader,
            Action::ComposeOrderMenu => Summary::ComposeReorder,
            #[cfg(feature = "developer-scheduler")]
            Action::ScheduleEnabledMenu => Summary::ScheduleEnabled,
            Action::DiffContextMenu | Action::DiffContext(_) => Summary::DiffContext,
            Action::FilterMenu => Summary::FilterMenu,
            Action::SortMenu => Summary::SortMenu,
            Action::Compose(action) => match action {
                crate::composition::Action::Open => Summary::Compose,
                crate::composition::Action::Next => Summary::ComposeNext,
                crate::composition::Action::Regenerate => Summary::ComposeRegenerate,
                crate::composition::Action::Edit => Summary::ComposeEdit,
                crate::composition::Action::Up => Summary::ComposeUp,
                crate::composition::Action::Down => Summary::ComposeDown,
                crate::composition::Action::Split => Summary::ComposeSplit,
                crate::composition::Action::Merge => Summary::ComposeMerge,
                crate::composition::Action::Move => Summary::ComposeMove,
            },
            Action::Commit => Summary::Commit,
            Action::CommitGenerate => Summary::CommitGenerate,
            Action::AiAgent => Summary::AiAgent,
            Action::AiModel => Summary::AiModel,
            Action::AiPrompt => Summary::AiPrompt,
            Action::Push => Summary::Push,
            Action::OpenMenu => Summary::OpenMenu,
            Action::Terminal => Summary::Terminal,
            Action::OpenFolder => Summary::OpenFolder,
            Action::OpenEditor => Summary::ExternalEditorOpen,
            Action::EditorSetting => Summary::ExternalEditorSetting,
            Action::FetchAll => Summary::FetchAll,
            Action::CancelCheck => Summary::CancelCheck,
            Action::Refresh => Summary::Refresh,
            Action::Settings => Summary::Settings,
            Action::AiSettings => Summary::AiSettings,
            #[cfg(feature = "developer-scheduler")]
            Action::Schedule => Summary::Schedule,
            Action::OpenRepository => Summary::OpenRepository,
            Action::Repositories => Summary::Repositories,
            Action::RepositorySettings => Summary::RepositorySettings,
            Action::RepositoryEnv => Summary::GitEnvironment,
            Action::RepositoryEnvEdit(field) => match field {
                super::EnvironmentField::Path => Summary::GitEnvironmentPath,
                super::EnvironmentField::Set => Summary::GitEnvironmentSet,
                super::EnvironmentField::Unset => Summary::GitEnvironmentUnset,
                super::EnvironmentField::Launcher => Summary::GitEnvironmentLauncher,
            },
            Action::Add => Summary::Add,
            Action::Remove => Summary::Remove,
            Action::Scan => Summary::Scan,
            #[cfg(feature = "developer-scheduler")]
            Action::Interval => Summary::Interval,
            #[cfg(feature = "developer-scheduler")]
            Action::Anchor => Summary::Anchor,
            #[cfg(feature = "developer-scheduler")]
            Action::Autopush => Summary::Autopush,
            Action::Concurrency => Summary::Concurrency,
            #[cfg(feature = "developer-scheduler")]
            Action::Workflow => Summary::ScheduledActions,
            Action::WrapMenu | Action::Wrap(_) => Summary::TextWrap,
            Action::AutoFetchMenu | Action::AutoFetch(_) => Summary::AutoFetch,
            Action::AutoFetchInterval => Summary::AutoFetchInterval,
            Action::StartupFetchMenu | Action::StartupFetch(_) => Summary::StartupFetch,
            #[cfg(feature = "developer-scheduler")]
            Action::Apply => Summary::Apply,
            #[cfg(feature = "developer-scheduler")]
            Action::EnableSchedule => Summary::EnableSchedule,
            #[cfg(feature = "developer-scheduler")]
            Action::DisableSchedule => Summary::DisableSchedule,
            Action::Help => Summary::Help,
            Action::InteractionMenu => Summary::InteractionMenu,
            Action::Quit => Summary::Quit,
            Action::FileLayout(_) => Summary::FileLayoutMenu,
            Action::DiffLayout(_) => Summary::DiffLayoutMenu,
            Action::Filter(_) => Summary::FilterMenu,
            Action::Sort(_) => Summary::SortMenu,
            Action::Interaction(_) => Summary::InteractionMenu,
            Action::Copy => Summary::Copy,
            Action::CopyPath => Summary::CopyPath,
        }
    }
}
