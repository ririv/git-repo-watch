//! Explicit command catalog independent of terminal rendering.
mod argument;
pub(crate) mod availability;
pub(crate) mod completion;
pub(crate) mod context;
mod enter;
pub(crate) mod hierarchy;
pub(crate) mod history;
mod input;
pub(crate) mod loader;
pub(crate) mod named;
pub(crate) mod options;
pub(crate) mod path;
mod prefix;
pub(crate) mod repository;
pub(crate) mod scan;
mod scope;
mod summary;
pub(crate) mod syntax;
pub(crate) mod target;
#[cfg(test)]
mod tests;
pub(crate) mod values;
pub(crate) use argument::ArgumentSyntax;
pub use argument::{ArgumentAction, ConfigArgument, argument_input};
mod environment;
pub(crate) use context::Context;
pub use environment::Field as EnvironmentField;
pub(crate) use input::{Discovery, Input, NamespaceEvent};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionAction {
    Toggle,
    All,
    Clear,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Tasks,
    TaskCancel,
    TaskRetry,
    TaskFilterMenu,
    TaskFilter(crate::task_center::Filter),
    SearchMenu,
    Search(crate::search::Mode),
    Focus,
    Zoom,
    Branches,
    BranchFilterMenu,
    BranchFilter(crate::branches::Filter),
    Worktrees,
    WorktreeAdd,
    WorktreeOpen,
    WorktreeRemove,
    BranchCreate,
    BranchSwitch,
    Fetch,
    Pull,
    Sync,
    Stashes,
    StashPreviewMenu,
    StashPreview(engine::local::stash::preview::Scope),
    StashSave,
    StashSaveMode(crate::stash_save::Scope),
    StashApply,
    StashPop,
    StashDrop,
    Stage,
    Unstage,
    PartialSelectMenu,
    PartialSelect(crate::changes::edit::Granularity),
    Discard,
    DiscardFile,
    Diff,
    DiffGroup(crate::files::Group),
    ConflictMenu,
    ConflictAccept(engine::local::files::conflict::resolve::Choice),
    ConflictUndo,
    ConflictReset,
    ConflictView(crate::conflict::DisplayMode),
    BrowseFiles,
    BrowserDiff,
    BrowserLayoutMenu,
    BrowserLayout(crate::browser::opening::Layout),
    BrowserModeMenu,
    BrowserMode(crate::browser::opening::TextMode),
    Log,
    LogMore,
    LogParent,
    Select(SelectionAction),
    Replay(engine::operations::replay::Kind),
    ResetBranch(engine::operations::reset::Mode),
    ReplayControl(engine::operations::replay::Control),
    FileLayoutMenu,
    FileLayout(crate::files::Layout),
    DiffLayoutMenu,
    DiffHeader,
    DiffHeaderVisibility(crate::diff::HeaderVisibility),
    DiffContextMenu,
    DiffContext(crate::diff::Context),
    DiffLayout(crate::diff::Preference),
    FilterMenu,
    Filter(crate::repository_list::Filter),
    SortMenu,
    Sort(crate::repository_list::Order),
    Copy,
    CopyPath,
    Commit,
    CommitGenerate,
    Compose(crate::composition::Action),
    ComposeOrderMenu,
    AiAgent,
    AiModel,
    AiPrompt,
    Push,
    OpenMenu,
    Terminal,
    OpenFolder,
    OpenEditor,
    EditorSetting,
    FetchAll,
    CancelCheck,
    Refresh,
    Settings,
    AiSettings,
    Schedule,
    OpenRepository,
    Repositories,
    RepositorySettings,
    RepositoryEnv,
    RepositoryEnvEdit(EnvironmentField),
    Add,
    Remove,
    Scan,
    Interval,
    Anchor,
    Autopush,
    Concurrency,
    Workflow,
    AutoFetchMenu,
    AutoFetch(bool),
    AutoFetchInterval,
    StartupFetchMenu,
    StartupFetch(bool),
    WrapMenu,
    Wrap(bool),
    Apply,
    EnableSchedule,
    DisableSchedule,
    ScheduleEnabledMenu,
    Help,
    InteractionMenu,
    Interaction(engine::state::InteractionMode),
    Quit,
}
#[derive(Debug, Clone, Copy)]
pub struct Command {
    pub name: &'static str,
    pub action: Action,
    pub description: i18n::Message,
}
impl Command {
    pub(crate) fn common(&self) -> bool {
        matches!(
            self.action,
            Action::Copy
                | Action::CopyPath
                | Action::Select(_)
                | Action::Refresh
                | Action::OpenEditor
                | Action::OpenMenu
                | Action::Terminal
                | Action::OpenFolder
        )
    }
}
use Action::*;
// 父命令排在子命令之前，使前缀匹配默认选中入口，避免意外执行子操作。
pub const COMMANDS: &[Command] = &[
    Command {
        name: "repo",
        action: OpenRepository,
        description: i18n::Message::OpenRepositoryOverview,
    },
    Command {
        name: "repos",
        action: Repositories,
        description: i18n::Message::ManageRepositories,
    },
    Command {
        name: "repo branch",
        action: Action::Branches,
        description: i18n::Message::BranchCommand,
    },
    Command {
        name: "repo branch filter",
        action: BranchFilterMenu,
        description: i18n::Message::BranchFilterCommand,
    },
    Command {
        name: "repo branch create",
        action: BranchCreate,
        description: i18n::Message::BranchCreateCommand,
    },
    Command {
        name: "repo branch switch",
        action: BranchSwitch,
        description: i18n::Message::BranchSwitchCommand,
    },
    Command {
        name: "repo worktree",
        action: Worktrees,
        description: i18n::Message::WorktreesCommand,
    },
    Command {
        name: "repo worktree add",
        action: WorktreeAdd,
        description: i18n::Message::WorktreeAdd,
    },
    Command {
        name: "repo worktree open",
        action: WorktreeOpen,
        description: i18n::Message::WorktreeOpen,
    },
    Command {
        name: "repo worktree remove",
        action: WorktreeRemove,
        description: i18n::Message::WorktreeRemove,
    },
    Command {
        name: "repo log",
        action: Log,
        description: i18n::Message::HistoryCommand,
    },
    Command {
        name: "repo log cherry-pick",
        action: Replay(engine::operations::replay::Kind::CherryPick),
        description: i18n::Message::ReplayCherryPick,
    },
    Command {
        name: "repo log revert",
        action: Replay(engine::operations::replay::Kind::Revert),
        description: i18n::Message::ReplayRevert,
    },
    Command {
        name: "repo log reset",
        action: ResetBranch(engine::operations::reset::Mode::Soft),
        description: i18n::Message::ResetCommand,
    },
    Command {
        name: "repo continue",
        action: ReplayControl(engine::operations::replay::Control::Continue),
        description: i18n::Message::ReplayContinue,
    },
    Command {
        name: "repo skip",
        action: ReplayControl(engine::operations::replay::Control::Skip),
        description: i18n::Message::ReplaySkip,
    },
    Command {
        name: "repo abort",
        action: ReplayControl(engine::operations::replay::Control::Abort),
        description: i18n::Message::ReplayAbort,
    },
    Command {
        name: "repo log parent",
        action: LogParent,
        description: i18n::Message::HistoryParentCommand,
    },
    Command {
        name: "repo log more",
        action: LogMore,
        description: i18n::Message::HistoryMoreCommand,
    },
    Command {
        name: "repo diff",
        action: Diff,
        description: i18n::Message::ChangesOpen,
    },
    Command {
        name: "repo diff stage",
        action: Stage,
        description: i18n::Message::StageSelectedFiles,
    },
    Command {
        name: "repo diff unstage",
        action: Unstage,
        description: i18n::Message::UnstageSelectedFiles,
    },
    Command {
        name: "repo diff layout",
        action: DiffLayoutMenu,
        description: i18n::Message::ChangesLayout,
    },
    Command {
        name: "repo diff unstaged",
        action: DiffGroup(crate::files::Group::Unstaged),
        description: i18n::Message::ChangesOpenUnstaged,
    },
    Command {
        name: "repo diff staged",
        action: DiffGroup(crate::files::Group::Staged),
        description: i18n::Message::ChangesOpenStaged,
    },
    Command {
        name: "repo diff select",
        action: PartialSelectMenu,
        description: i18n::Message::PartialSelectMode,
    },
    Command {
        name: "repo diff context",
        action: DiffContextMenu,
        description: i18n::Message::DiffContextCommand,
    },
    Command {
        name: "repo diff header",
        action: DiffHeader,
        description: i18n::Message::DiffHeaderCommand,
    },
    Command {
        name: "repo diff discard",
        action: Discard,
        description: i18n::Message::PartialDiscard,
    },
    Command {
        name: "repo diff discard-file",
        action: DiscardFile,
        description: i18n::Message::DiscardFile,
    },
    Command {
        name: "repo diff view",
        action: FileLayoutMenu,
        description: i18n::Message::FilesLayout,
    },
    Command {
        name: "repo diff conflict",
        action: ConflictMenu,
        description: i18n::Message::ConflictControlsTitle,
    },
    Command {
        name: "repo diff conflict undo",
        action: ConflictUndo,
        description: i18n::Message::ConflictUndoCommand,
    },
    Command {
        name: "repo diff conflict reset",
        action: ConflictReset,
        description: i18n::Message::ConflictResetCommand,
    },
    Command {
        name: "repo files",
        action: BrowseFiles,
        description: i18n::Message::BrowserDescription,
    },
    Command {
        name: "repo files diff",
        action: BrowserDiff,
        description: i18n::Message::BrowserDiffDescription,
    },
    Command {
        name: "repo files layout",
        action: BrowserLayoutMenu,
        description: i18n::Message::FileLayoutCommand,
    },
    Command {
        name: "repo files mode",
        action: BrowserModeMenu,
        description: i18n::Message::FileModeCommand,
    },
    Command {
        name: "repo commit",
        action: Commit,
        description: i18n::Message::ReviewStagedContentsEnterAMessageAnd,
    },
    Command {
        name: "repo compose",
        action: Compose(crate::composition::Action::Open),
        description: i18n::Message::ComposeCommand,
    },
    Command {
        name: "repo compose commit",
        action: Compose(crate::composition::Action::Next),
        description: i18n::Message::ComposeNext,
    },
    Command {
        name: "repo compose regenerate",
        action: Compose(crate::composition::Action::Regenerate),
        description: i18n::Message::ComposeRegenerate,
    },
    Command {
        name: "repo compose message",
        action: Compose(crate::composition::Action::Edit),
        description: i18n::Message::ComposeEdit,
    },
    Command {
        name: "repo compose reorder",
        action: ComposeOrderMenu,
        description: i18n::Message::ComposeReorderCommand,
    },
    Command {
        name: "repo compose split",
        action: Compose(crate::composition::Action::Split),
        description: i18n::Message::ComposeSplit,
    },
    Command {
        name: "repo compose merge",
        action: Compose(crate::composition::Action::Merge),
        description: i18n::Message::ComposeMerge,
    },
    Command {
        name: "repo compose move",
        action: Compose(crate::composition::Action::Move),
        description: i18n::Message::ComposeMove,
    },
    Command {
        name: "repo push",
        action: Push,
        description: i18n::Message::ReviewCurrentBranchAndRemoteUpstreamThen,
    },
    Command {
        name: "repo commit generate",
        action: CommitGenerate,
        description: i18n::Message::AgentGenerate,
    },
    Command {
        name: "repo env",
        action: RepositoryEnv,
        description: i18n::Message::GitEnvironment,
    },
    Command {
        name: "repo env path",
        action: RepositoryEnvEdit(EnvironmentField::Path),
        description: i18n::Message::GitEnvironmentPath,
    },
    Command {
        name: "repo env set",
        action: RepositoryEnvEdit(EnvironmentField::Set),
        description: i18n::Message::GitEnvironmentSet,
    },
    Command {
        name: "repo env unset",
        action: RepositoryEnvEdit(EnvironmentField::Unset),
        description: i18n::Message::GitEnvironmentUnset,
    },
    Command {
        name: "repo env launcher",
        action: RepositoryEnvEdit(EnvironmentField::Launcher),
        description: i18n::Message::GitEnvironmentLauncher,
    },
    Command {
        name: "repo fetch",
        action: Fetch,
        description: i18n::Message::FetchCommand,
    },
    Command {
        name: "repo pull",
        action: Pull,
        description: i18n::Message::PullCommand,
    },
    Command {
        name: "repo sync",
        action: Sync,
        description: i18n::Message::SyncCommand,
    },
    Command {
        name: "repo stash",
        action: Stashes,
        description: i18n::Message::StashCommand,
    },
    Command {
        name: "repo stash scope",
        action: StashPreviewMenu,
        description: i18n::Message::StashPreviewCommand,
    },
    Command {
        name: "repo stash save",
        action: StashSave,
        description: i18n::Message::StashSaveCommand,
    },
    Command {
        name: "repo stash apply",
        action: StashApply,
        description: i18n::Message::StashApplyCommand,
    },
    Command {
        name: "repo stash pop",
        action: StashPop,
        description: i18n::Message::StashPopCommand,
    },
    Command {
        name: "repo stash drop",
        action: StashDrop,
        description: i18n::Message::StashDropCommand,
    },
    Command {
        name: "repos fetch",
        action: FetchAll,
        description: i18n::Message::FetchAllCommand,
    },
    Command {
        name: "cancel",
        action: CancelCheck,
        description: i18n::Message::CancelBackgroundCheck,
    },
    Command {
        name: "repos filter",
        action: FilterMenu,
        description: i18n::Message::RepositoryFilterCommand,
    },
    Command {
        name: "repos sort",
        action: SortMenu,
        description: i18n::Message::RepositorySortCommand,
    },
    Command {
        name: "repos manage",
        action: RepositorySettings,
        description: i18n::Message::RepositoryManagementSettings,
    },
    Command {
        name: "search",
        action: SearchMenu,
        description: i18n::Message::SearchModeCommand,
    },
    Command {
        name: "focus",
        action: Focus,
        description: i18n::Message::PaneFocusCommand,
    },
    Command {
        name: "zoom",
        action: Zoom,
        description: i18n::Message::PaneZoomCommand,
    },
    Command {
        name: "select",
        action: Select(SelectionAction::Toggle),
        description: i18n::Message::SelectionToggleItem,
    },
    Command {
        name: "select all",
        action: Select(SelectionAction::All),
        description: i18n::Message::SelectionSelectAll,
    },
    Command {
        name: "select clear",
        action: Select(SelectionAction::Clear),
        description: i18n::Message::SelectionClearMarks,
    },
    Command {
        name: "editor",
        action: OpenEditor,
        description: i18n::Message::ExternalEditorOpen,
    },
    Command {
        name: "open",
        action: OpenMenu,
        description: i18n::Message::OpenDirectoryMenu,
    },
    Command {
        name: "open terminal",
        action: Terminal,
        description: i18n::Message::OpenRepositoryTerminal,
    },
    Command {
        name: if cfg!(target_os = "macos") {
            "open finder"
        } else if cfg!(target_os = "windows") {
            "open explorer"
        } else {
            "open browse"
        },
        action: OpenFolder,
        description: i18n::Message::OpenInFileManager,
    },
    Command {
        name: "copy",
        action: Copy,
        description: i18n::Message::CopySelectionRequired,
    },
    Command {
        name: "copy path",
        action: CopyPath,
        description: i18n::Message::CopyFilePaths,
    },
    Command {
        name: "refresh",
        action: Refresh,
        description: i18n::Message::RefreshContextView,
    },
    Command {
        name: "tasks",
        action: Tasks,
        description: i18n::Message::Tasks,
    },
    Command {
        name: "tasks filter",
        action: TaskFilterMenu,
        description: i18n::Message::TasksFilterStatus,
    },
    Command {
        name: "tasks cancel",
        action: TaskCancel,
        description: i18n::Message::TaskCancel,
    },
    Command {
        name: "tasks retry",
        action: TaskRetry,
        description: i18n::Message::TaskRetry,
    },
    Command {
        name: "schedule",
        action: Schedule,
        description: i18n::Message::ConfigureSchedule,
    },
    Command {
        name: "settings",
        action: Settings,
        description: i18n::Message::OpenSettings,
    },
    Command {
        name: "repos add",
        action: Add,
        description: i18n::Message::AddARepositoryRoot,
    },
    Command {
        name: "repos remove",
        action: Remove,
        description: i18n::Message::UnregisterRepositoryKeepFilesOnDisk,
    },
    Command {
        name: "repos scan",
        action: Scan,
        description: i18n::Message::PreviewScanUsingGitIgnoreRules,
    },
    Command {
        name: "schedule interval",
        action: Interval,
        description: i18n::Message::SaveCheckInterval,
    },
    Command {
        name: "schedule anchor",
        action: Anchor,
        description: i18n::Message::SaveScheduleAnchor,
    },
    Command {
        name: "schedule autopush",
        action: Autopush,
        description: i18n::Message::SaveAutomaticPushPolicy,
    },
    Command {
        name: "settings concurrency",
        action: Concurrency,
        description: i18n::Message::SetCheckConcurrencyLimit132,
    },
    Command {
        name: "schedule actions",
        action: Workflow,
        description: i18n::Message::WorkflowScheduledActions,
    },
    Command {
        name: "settings startup-fetch",
        action: StartupFetchMenu,
        description: i18n::Message::StartupFetchHint,
    },
    Command {
        name: "settings auto-fetch",
        action: AutoFetchMenu,
        description: i18n::Message::AutoFetchHint,
    },
    Command {
        name: "settings auto-fetch-interval",
        action: AutoFetchInterval,
        description: i18n::Message::AutoFetchIntervalHint,
    },
    Command {
        name: "settings wrap",
        action: WrapMenu,
        description: i18n::Message::TextWrapHint,
    },
    Command {
        name: "settings interaction",
        action: InteractionMenu,
        description: i18n::Message::InteractionCommand,
    },
    Command {
        name: "settings ai",
        action: AiSettings,
        description: i18n::Message::AiSettings,
    },
    Command {
        name: "settings editor",
        action: EditorSetting,
        description: i18n::Message::ExternalEditorSetting,
    },
    Command {
        name: "settings ai agent",
        action: AiAgent,
        description: i18n::Message::AgentNameHint,
    },
    Command {
        name: "settings ai model",
        action: AiModel,
        description: i18n::Message::AgentModelHint,
    },
    Command {
        name: "settings ai prompt",
        action: AiPrompt,
        description: i18n::Message::AgentPromptHint,
    },
    Command {
        name: "schedule apply",
        action: Apply,
        description: i18n::Message::ApplySystemScheduleWithoutStartingACheck,
    },
    Command {
        name: "schedule enabled",
        action: ScheduleEnabledMenu,
        description: i18n::Message::ScheduleEnabledCommand,
    },
    Command {
        name: "help",
        action: Help,
        description: i18n::Message::ShowHelp,
    },
    Command {
        name: "quit",
        action: Quit,
        description: i18n::Message::Quit,
    },
];
pub fn matches(input: &str) -> Vec<&'static Command> {
    if let Some((argument, _)) = argument_input(input) {
        return vec![argument.command()];
    }
    let input = input.trim_start();
    if input.is_empty() {
        return COMMANDS
            .iter()
            .filter(|command| !command.name.contains(' '))
            .collect();
    }
    let query = MatchQuery::new(input);
    // 完整命令名按词边界限定组，避免 repo 同时匹配 repos。
    let exact = COMMANDS.iter().any(|command| command.name == query.text);
    let prefix = format!("{} ", query.text);
    // 父命令末尾的空格限定为子命令；普通说明短语仍搜索整个目录。
    let parent = input
        .strip_suffix(' ')
        .filter(|parent| {
            COMMANDS
                .iter()
                .any(|command| command.name.eq_ignore_ascii_case(parent))
        })
        .map(|parent| format!("{} ", parent.to_ascii_lowercase()));
    let mut matches = COMMANDS
        .iter()
        .filter(|command| {
            (!exact || command.name == query.text || command.name.starts_with(&prefix))
                && parent
                    .as_ref()
                    .is_none_or(|parent| command.name.starts_with(parent))
        })
        .filter_map(|command| command.match_kind(&query).map(|kind| (kind, command)))
        .collect::<Vec<_>>();
    let grouped = exact || parent.is_some();
    matches.sort_by_key(|(kind, command)| {
        (
            *kind,
            if grouped {
                command.name.split_whitespace().count()
            } else {
                0
            },
        )
    });
    matches.into_iter().map(|(_, command)| command).collect()
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum MatchKind {
    NamePrefix,
    NameWordPrefix,
    NameSubstring,
    NamePrefixWithOptionalSpaces,
    NameSubstringWithOptionalSpaces,
    Description,
    DescriptionWithOptionalSpaces,
}

struct MatchQuery {
    text: String,
}
impl MatchQuery {
    fn new(input: &str) -> Self {
        Self {
            text: input
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .to_lowercase(),
        }
    }

    fn name_rank(&self, name: &str) -> Option<MatchKind> {
        if name.starts_with(&self.text) {
            return Some(MatchKind::NamePrefix);
        }
        if name
            .match_indices(' ')
            .any(|(index, _)| name[index + 1..].starts_with(&self.text))
        {
            return Some(MatchKind::NameWordPrefix);
        }
        if name.contains(&self.text) {
            return Some(MatchKind::NameSubstring);
        }
        if prefix_with_optional_spaces(name, &self.text) {
            Some(MatchKind::NamePrefixWithOptionalSpaces)
        } else if self.contains_with_optional_spaces(name) {
            Some(MatchKind::NameSubstringWithOptionalSpaces)
        } else {
            None
        }
    }

    fn description_rank(&self, text: &str) -> Option<MatchKind> {
        let text = text.to_lowercase();
        if text.contains(&self.text) {
            Some(MatchKind::Description)
        } else if self.contains_with_optional_spaces(&text) {
            Some(MatchKind::DescriptionWithOptionalSpaces)
        } else {
            None
        }
    }

    fn contains_with_optional_spaces(&self, text: &str) -> bool {
        let Some(first) = self.text.chars().next() else {
            return true;
        };
        text.match_indices(first)
            .any(|(index, _)| prefix_with_optional_spaces(&text[index..], &self.text))
    }
}

fn prefix_with_optional_spaces(candidate: &str, query: &str) -> bool {
    let mut candidate = candidate.chars().peekable();
    for character in query.chars() {
        // 可以省略候选中已有的词间空格，输入的空格必须对应已有词边界。
        let separated = candidate.peek().is_some_and(|next| next.is_whitespace());
        if character == ' ' && !separated {
            return false;
        }
        while candidate.peek().is_some_and(|next| next.is_whitespace()) {
            candidate.next();
        }
        if character != ' ' && candidate.next() != Some(character) {
            return false;
        }
    }
    true
}

impl Command {
    fn match_kind(&self, query: &MatchQuery) -> Option<MatchKind> {
        self.match_name(self.name, query)
    }

    fn match_name(&self, name: &str, query: &MatchQuery) -> Option<MatchKind> {
        if let Some(rank) = query.name_rank(name) {
            return Some(rank);
        }
        use i18n::Language::{Chinese, English};
        let descriptions = [English, Chinese].into_iter().flat_map(|language| {
            [
                self.description.in_language(language),
                self.summary().in_language(language),
            ]
        });
        let options = self
            .action
            .argument()
            .and_then(|argument| argument.options());
        let option_descriptions = options.into_iter().flat_map(|kind| {
            kind.options().flat_map(|option| {
                [
                    option.token,
                    option.description.in_language(English),
                    option.description.in_language(Chinese),
                ]
            })
        });
        descriptions
            .chain(option_descriptions)
            .filter_map(|text| query.description_rank(text))
            .min()
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Configuration {
    General,
    Schedule,
    Repositories,
    RepositoryEnvironment,
}
impl Configuration {
    pub fn actions(self, registration: engine::scheduler::Registration) -> &'static [Action] {
        match self {
            Self::General => &[
                Concurrency,
                StartupFetchMenu,
                AutoFetchMenu,
                AutoFetchInterval,
                WrapMenu,
            ],
            Self::Repositories => &[Add, Remove, Scan],
            Self::RepositoryEnvironment => &[
                RepositoryEnvEdit(EnvironmentField::Path),
                RepositoryEnvEdit(EnvironmentField::Set),
                RepositoryEnvEdit(EnvironmentField::Unset),
                RepositoryEnvEdit(EnvironmentField::Launcher),
            ],
            Self::Schedule => match registration {
                engine::scheduler::Registration::Loaded => {
                    &[DisableSchedule, Interval, Anchor, Workflow, Apply]
                }
                engine::scheduler::Registration::NotLoaded => {
                    &[EnableSchedule, Interval, Anchor, Workflow, Apply]
                }
            },
        }
    }
    pub fn title(self) -> &'static str {
        match self {
            Self::General => i18n::tr!(GeneralSettings),
            Self::Repositories => i18n::tr!(RepositoryManagement),
            Self::RepositoryEnvironment => i18n::tr!(GitEnvironment),
            Self::Schedule => i18n::tr!(ScheduleSettings),
        }
    }
}
pub fn description(action: Action) -> &'static str {
    if let Some(description) = options::description(action) {
        return description;
    }
    COMMANDS
        .iter()
        .find(|c| c.action == action)
        .expect("catalog covers every action")
        .description
        .text()
}

impl Action {
    pub fn experimental(self) -> bool {
        matches!(self, Self::Compose(_) | Self::ComposeOrderMenu)
    }
    pub fn available(self, config: &engine::state::Config) -> bool {
        (!self.experimental() || config.experimental_features)
            && (config.ai.enabled
                || !matches!(
                    self,
                    Self::AiSettings
                        | Self::Compose(_)
                        | Self::ComposeOrderMenu
                        | Self::CommitGenerate
                        | Self::AiAgent
                        | Self::AiModel
                        | Self::AiPrompt
                ))
    }
}
