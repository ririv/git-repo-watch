//! Every command action supplies a concise candidate description.
use super::{Action, Command};
use i18n::CommandSummary as Summary;
impl Command {
    pub fn summary(&self) -> Summary {
        match self.action {
            Action::PartialSelectMenu | Action::PartialSelect(_) => Summary::PartialSelect,
            Action::Discard => Summary::PartialDiscard,
            Action::SearchMenu => Summary::SearchMode,
            Action::Search(crate::search::Mode::Filter) => Summary::SearchFilter,
            Action::Search(crate::search::Mode::Locate) => Summary::SearchLocate,
            Action::Tasks => Summary::Tasks,
            Action::Focus => Summary::PaneFocus,
            Action::Zoom => Summary::PaneZoom,
            Action::Branches => Summary::Branches,
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
            Action::LogSelect | Action::LogSelection(_) => Summary::LogSelection,
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
            Action::Terminal => Summary::Terminal,
            Action::OpenEditor => Summary::ExternalEditorOpen,
            Action::EditorSetting => Summary::ExternalEditorSetting,
            Action::Check => Summary::Check,
            Action::CheckAll => Summary::CheckAll,
            Action::CancelCheck => Summary::CancelCheck,
            Action::Refresh => Summary::Refresh,
            Action::Settings => Summary::Settings,
            Action::AiSettings => Summary::AiSettings,
            Action::Schedule => Summary::Schedule,
            Action::OpenRepository => Summary::OpenRepository,
            Action::Repositories => Summary::Repositories,
            Action::RepositorySettings => Summary::RepositorySettings,
            Action::Add => Summary::Add,
            Action::Remove => Summary::Remove,
            Action::Scan => Summary::Scan,
            Action::Interval => Summary::Interval,
            Action::Anchor => Summary::Anchor,
            Action::Autopush => Summary::Autopush,
            Action::Concurrency => Summary::Concurrency,
            Action::StartupCheck => Summary::StartupCheck,
            Action::Workflow(engine::workflow::Trigger::Startup) => Summary::StartupActions,
            Action::Workflow(engine::workflow::Trigger::Scheduled) => Summary::ScheduledActions,
            Action::WrapMenu | Action::Wrap(_) => Summary::TextWrap,
            Action::Apply => Summary::Apply,
            Action::EnableSchedule => Summary::EnableSchedule,
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
        }
    }
}
