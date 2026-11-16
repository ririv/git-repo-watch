//! Explicit command catalog independent of terminal rendering.
mod argument;
pub(crate) mod availability;
pub(crate) mod completion;
pub(crate) mod context;
mod input;
pub(crate) mod loader;
pub(crate) mod options;
pub(crate) mod path;
mod prefix;
pub(crate) mod repository;
pub(crate) mod scan;
mod scope;
mod summary;
pub(crate) mod target;
#[cfg(test)]
mod tests;
pub(crate) mod values;
pub(crate) use argument::ArgumentSyntax;
pub use argument::{ArgumentAction, ConfigArgument, argument_input};
pub(crate) use context::Context;
pub(crate) use input::{Input, NamespaceEvent};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Tasks,
    SearchMenu,
    Search(crate::search::Mode),
    Focus,
    Zoom,
    Branches,
    BranchFilterMenu,
    BranchFilter(crate::branches::Filter),
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
    ConflictAcceptMenu,
    ConflictAccept(engine::local::files::conflict::resolve::Choice),
    ConflictBlockMenu,
    ConflictBlock(crate::conflict::resolution::Direction),
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
    LogSelect,
    LogSelection(crate::history::SelectionAction),
    Replay(engine::operations::replay::Kind),
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
    Terminal,
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
                | Action::Refresh
                | Action::OpenEditor
                | Action::Terminal
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
        name: "repo log select",
        action: LogSelect,
        description: i18n::Message::HistorySelect,
    },
    Command {
        name: "repo log revert",
        action: Replay(engine::operations::replay::Kind::Revert),
        description: i18n::Message::ReplayRevert,
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
        description: i18n::Message::ConflictViewCommand,
    },
    Command {
        name: "repo diff conflict accept",
        action: ConflictAcceptMenu,
        description: i18n::Message::ConflictAcceptCommand,
    },
    Command {
        name: "repo diff conflict block",
        action: ConflictBlockMenu,
        description: i18n::Message::ConflictBlockCommand,
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
        name: "editor",
        action: OpenEditor,
        description: i18n::Message::ExternalEditorOpen,
    },
    Command {
        name: "terminal",
        action: Terminal,
        description: i18n::Message::OpenRepositoryTerminal,
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
    let query = input.trim_end().to_lowercase();
    // 完整命令名按词边界限定组，避免 repo 同时匹配 repos。
    let exact = COMMANDS.iter().any(|command| command.name == query);
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
            (!exact || command.name == query || command.name.starts_with(&format!("{query} ")))
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
    Description,
}
impl Command {
    fn match_kind(&self, query: &str) -> Option<MatchKind> {
        self.match_name(self.name, query)
    }
    fn match_name(&self, name: &str, query: &str) -> Option<MatchKind> {
        if name.starts_with(query) {
            Some(MatchKind::NamePrefix)
        } else if name
            .match_indices(' ')
            .any(|(index, _)| name[index + 1..].starts_with(query))
        {
            Some(MatchKind::NameWordPrefix)
        } else if name.contains(query) {
            Some(MatchKind::NameSubstring)
        } else if self
            .action
            .argument()
            .and_then(|argument| argument.options())
            .is_some_and(|kind| {
                kind.options().any(|option| {
                    option.token.contains(query)
                        || [i18n::Language::English, i18n::Language::Chinese]
                            .iter()
                            .any(|language| {
                                option
                                    .description
                                    .in_language(*language)
                                    .to_lowercase()
                                    .contains(query)
                            })
                })
            })
            || [i18n::Language::English, i18n::Language::Chinese]
                .into_iter()
                .any(|language| {
                    [
                        self.description.in_language(language),
                        self.summary().in_language(language),
                    ]
                    .iter()
                    .any(|text| text.to_lowercase().contains(query))
                })
        {
            Some(MatchKind::Description)
        } else {
            None
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Configuration {
    General,
    Schedule,
    Repositories,
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
