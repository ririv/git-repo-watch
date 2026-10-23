//! Explicit command catalog independent of terminal rendering.
mod argument;
pub(crate) mod context;
mod input;
pub(crate) mod path;
#[cfg(test)]
mod tests;
pub use argument::{ArgumentAction, ConfigArgument, argument_input};
pub(crate) use context::Context;
pub(crate) use input::{Input, NamespaceEvent};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Branches,
    BranchCreate,
    BranchSwitch,
    Fetch,
    Pull,
    Stashes,
    StashSave,
    StashSaveAll,
    StashApply,
    StashPop,
    StashDrop,
    Stage,
    Unstage,
    Diff,
    Log,
    LogMore,
    FileLayoutMenu,
    FileLayout(crate::files::Layout),
    DiffLayoutMenu,
    DiffLayout(crate::diff::Preference),
    FilterMenu,
    Filter(crate::repository_list::Filter),
    SortMenu,
    Sort(crate::repository_list::Order),
    Copy(crate::clipboard::Scope),
    Commit,
    Push,
    Terminal,
    Check,
    CheckAll,
    CancelCheck,
    Refresh,
    Settings,
    Schedule,
    OpenRepository,
    Repositories,
    RepositorySettings,
    RepositoryRefresh,
    Add,
    Remove,
    Exclude,
    Unexclude,
    Scan,
    Import,
    Interval,
    Anchor,
    Autopush,
    Concurrency,
    StartupCheck,
    Apply,
    EnableSchedule,
    DisableSchedule,
    Help,
    InteractionMenu,
    Interaction(engine::state::InteractionMode),
    Quit,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Repository,
    Global,
    View,
}
#[derive(Debug, Clone, Copy)]
pub struct Command {
    pub name: &'static str,
    pub action: Action,
    pub scope: Scope,
    pub description: i18n::Message,
}
use Action::*;
use Scope::*;
// 父命令排在子命令之前，使前缀匹配默认选中入口，避免意外执行子操作。
pub const COMMANDS: &[Command] = &[
    Command {
        name: "repo",
        action: OpenRepository,
        scope: Repository,
        description: i18n::Message::OpenRepositoryOverview,
    },
    Command {
        name: "repos",
        action: Repositories,
        scope: Global,
        description: i18n::Message::ManageRepositories,
    },
    Command {
        name: "repo branch",
        action: Action::Branches,
        scope: Repository,
        description: i18n::Message::BranchCommand,
    },
    Command {
        name: "repo branch refresh",
        action: Refresh,
        scope: View,
        description: i18n::Message::RefreshContextView,
    },
    Command {
        name: "repo branch copy",
        action: Copy(crate::clipboard::Scope::Branch),
        scope: View,
        description: i18n::Message::CopyBranchCommand,
    },
    Command {
        name: "repo branch create",
        action: BranchCreate,
        scope: Repository,
        description: i18n::Message::BranchCreateCommand,
    },
    Command {
        name: "repo branch switch",
        action: BranchSwitch,
        scope: Repository,
        description: i18n::Message::BranchSwitchCommand,
    },
    Command {
        name: "repo log",
        action: Log,
        scope: Repository,
        description: i18n::Message::HistoryCommand,
    },
    Command {
        name: "repo log more",
        action: LogMore,
        scope: View,
        description: i18n::Message::HistoryMoreCommand,
    },
    Command {
        name: "repo log refresh",
        action: Refresh,
        scope: View,
        description: i18n::Message::RefreshContextView,
    },
    Command {
        name: "repo log copy",
        action: Copy(crate::clipboard::Scope::Log),
        scope: View,
        description: i18n::Message::CopyLogCommand,
    },
    Command {
        name: "repo diff",
        action: Diff,
        scope: Repository,
        description: i18n::Message::ChangesOpen,
    },
    Command {
        name: "repo diff stage",
        action: Stage,
        scope: Repository,
        description: i18n::Message::StageSelectedFiles,
    },
    Command {
        name: "repo diff unstage",
        action: Unstage,
        scope: Repository,
        description: i18n::Message::UnstageSelectedFiles,
    },
    Command {
        name: "repo diff layout",
        action: DiffLayoutMenu,
        scope: View,
        description: i18n::Message::ChangesLayout,
    },
    Command {
        name: "repo diff layout auto",
        action: DiffLayout(crate::diff::Preference::Auto),
        scope: View,
        description: i18n::Message::DiffChooseAuto,
    },
    Command {
        name: "repo diff layout split",
        action: DiffLayout(crate::diff::Preference::Split),
        scope: View,
        description: i18n::Message::DiffChooseSplit,
    },
    Command {
        name: "repo diff layout unified",
        action: DiffLayout(crate::diff::Preference::Unified),
        scope: View,
        description: i18n::Message::DiffChooseUnified,
    },
    Command {
        name: "repo diff refresh",
        action: Refresh,
        scope: View,
        description: i18n::Message::RefreshContextView,
    },
    Command {
        name: "repo diff copy",
        action: Copy(crate::clipboard::Scope::Diff),
        scope: View,
        description: i18n::Message::CopyDiffCommand,
    },
    Command {
        name: "repo diff view",
        action: FileLayoutMenu,
        scope: View,
        description: i18n::Message::FilesLayout,
    },
    Command {
        name: "repo diff view tree",
        action: FileLayout(crate::files::Layout::Tree),
        scope: View,
        description: i18n::Message::FilesTree,
    },
    Command {
        name: "repo diff view flat",
        action: FileLayout(crate::files::Layout::Flat),
        scope: View,
        description: i18n::Message::FilesFlat,
    },
    Command {
        name: "repo commit",
        action: Commit,
        scope: Repository,
        description: i18n::Message::ReviewStagedContentsEnterAMessageAnd,
    },
    Command {
        name: "repo push",
        action: Push,
        scope: Repository,
        description: i18n::Message::ReviewCurrentBranchAndRemoteUpstreamThen,
    },
    Command {
        name: "repo fetch",
        action: Fetch,
        scope: Repository,
        description: i18n::Message::FetchCommand,
    },
    Command {
        name: "repo pull",
        action: Pull,
        scope: Repository,
        description: i18n::Message::PullCommand,
    },
    Command {
        name: "repo stash",
        action: Stashes,
        scope: Repository,
        description: i18n::Message::StashCommand,
    },
    Command {
        name: "repo stash refresh",
        action: Refresh,
        scope: View,
        description: i18n::Message::RefreshContextView,
    },
    Command {
        name: "repo stash copy",
        action: Copy(crate::clipboard::Scope::Stash),
        scope: View,
        description: i18n::Message::StashCopyCommand,
    },
    Command {
        name: "repo stash save",
        action: StashSave,
        scope: Repository,
        description: i18n::Message::StashSaveCommand,
    },
    Command {
        name: "repo stash save-all",
        action: StashSaveAll,
        scope: Repository,
        description: i18n::Message::StashSaveAllCommand,
    },
    Command {
        name: "repo stash apply",
        action: StashApply,
        scope: Repository,
        description: i18n::Message::StashApplyCommand,
    },
    Command {
        name: "repo stash pop",
        action: StashPop,
        scope: Repository,
        description: i18n::Message::StashPopCommand,
    },
    Command {
        name: "repo stash drop",
        action: StashDrop,
        scope: Repository,
        description: i18n::Message::StashDropCommand,
    },
    Command {
        name: "repo terminal",
        action: Terminal,
        scope: Repository,
        description: i18n::Message::OpenRepositoryTerminal,
    },
    Command {
        name: "repo check",
        action: Check,
        scope: Repository,
        description: i18n::Message::CheckSelectedRepository,
    },
    Command {
        name: "repos check",
        action: CheckAll,
        scope: Global,
        description: i18n::Message::CheckAllRepositoriesInBackground,
    },
    Command {
        name: "cancel",
        action: CancelCheck,
        scope: Global,
        description: i18n::Message::CancelBackgroundCheck,
    },
    Command {
        name: "repo refresh",
        action: Refresh,
        scope: View,
        description: i18n::Message::ReloadReportOrFileList,
    },
    Command {
        name: "repos filter",
        action: FilterMenu,
        scope: View,
        description: i18n::Message::RepositoryFilterCommand,
    },
    Command {
        name: "repos filter all",
        action: Filter(crate::repository_list::Filter::All),
        scope: View,
        description: i18n::Message::RepositoryFilterAll,
    },
    Command {
        name: "repos filter problems",
        action: Filter(crate::repository_list::Filter::Problems),
        scope: View,
        description: i18n::Message::RepositoryFilterProblems,
    },
    Command {
        name: "repos filter clean",
        action: Filter(crate::repository_list::Filter::Clean),
        scope: View,
        description: i18n::Message::RepositoryFilterClean,
    },
    Command {
        name: "repos filter failed",
        action: Filter(crate::repository_list::Filter::Failed),
        scope: View,
        description: i18n::Message::RepositoryFilterFailed,
    },
    Command {
        name: "repos filter unchecked",
        action: Filter(crate::repository_list::Filter::Unchecked),
        scope: View,
        description: i18n::Message::RepositoryFilterUnchecked,
    },
    Command {
        name: "repos filter no-upstream",
        action: Filter(crate::repository_list::Filter::WithoutUpstream),
        scope: View,
        description: i18n::Message::RepositoryFilterNoUpstream,
    },
    Command {
        name: "repos filter pushes",
        action: Filter(crate::repository_list::Filter::Pushes),
        scope: View,
        description: i18n::Message::RepositoryFilterPushes,
    },
    Command {
        name: "repos sort",
        action: SortMenu,
        scope: View,
        description: i18n::Message::RepositorySortCommand,
    },
    Command {
        name: "repos sort priority",
        action: Sort(crate::repository_list::Order::Priority),
        scope: View,
        description: i18n::Message::RepositorySortPriority,
    },
    Command {
        name: "repos sort name",
        action: Sort(crate::repository_list::Order::Name),
        scope: View,
        description: i18n::Message::RepositorySortName,
    },
    Command {
        name: "repos sort path",
        action: Sort(crate::repository_list::Order::Path),
        scope: View,
        description: i18n::Message::RepositorySortPath,
    },
    Command {
        name: "repos sort updated",
        action: Sort(crate::repository_list::Order::Updated),
        scope: View,
        description: i18n::Message::RepositorySortUpdated,
    },
    Command {
        name: "repos refresh",
        action: RepositoryRefresh,
        scope: View,
        description: i18n::Message::RefreshRepositoryReport,
    },
    Command {
        name: "repos manage",
        action: RepositorySettings,
        scope: Global,
        description: i18n::Message::RepositoryManagementSettings,
    },
    Command {
        name: "repo copy",
        action: Copy(crate::clipboard::Scope::Repository),
        scope: Repository,
        description: i18n::Message::CopyRepositoryCommand,
    },
    Command {
        name: "schedule",
        action: Schedule,
        scope: Global,
        description: i18n::Message::ConfigureSchedule,
    },
    Command {
        name: "settings",
        action: Settings,
        scope: Global,
        description: i18n::Message::OpenSettings,
    },
    Command {
        name: "repos add",
        action: Add,
        scope: Global,
        description: i18n::Message::AddARepositoryRoot,
    },
    Command {
        name: "repos remove",
        action: Remove,
        scope: Repository,
        description: i18n::Message::UnregisterRepositoryKeepFilesOnDisk,
    },
    Command {
        name: "repos exclude",
        action: Exclude,
        scope: Global,
        description: i18n::Message::ExcludeDirectoryAndUnregisterRepositoriesWithinIt,
    },
    Command {
        name: "repos unexclude",
        action: Unexclude,
        scope: Global,
        description: i18n::Message::RemoveDirectoryExclusionWithoutImporting,
    },
    Command {
        name: "repos scan",
        action: Scan,
        scope: Global,
        description: i18n::Message::PreviewScanUsingGitIgnoreRules,
    },
    Command {
        name: "repos import",
        action: Import,
        scope: Global,
        description: i18n::Message::ScanAndImportRepositoryList,
    },
    Command {
        name: "schedule interval",
        action: Interval,
        scope: Global,
        description: i18n::Message::SaveCheckInterval,
    },
    Command {
        name: "schedule anchor",
        action: Anchor,
        scope: Global,
        description: i18n::Message::SaveScheduleAnchor,
    },
    Command {
        name: "schedule autopush",
        action: Autopush,
        scope: Global,
        description: i18n::Message::SaveAutomaticPushPolicy,
    },
    Command {
        name: "settings concurrency",
        action: Concurrency,
        scope: Global,
        description: i18n::Message::SetCheckConcurrencyLimit132,
    },
    Command {
        name: "settings startup-check",
        action: StartupCheck,
        scope: Global,
        description: i18n::Message::CheckAllOnStartup,
    },
    Command {
        name: "settings interaction",
        action: InteractionMenu,
        scope: Global,
        description: i18n::Message::InteractionCommand,
    },
    Command {
        name: "settings interaction interactive",
        action: Interaction(engine::state::InteractionMode::Application),
        scope: Global,
        description: i18n::Message::InteractionApplicationCommand,
    },
    Command {
        name: "settings interaction text",
        action: Interaction(engine::state::InteractionMode::TextSelection),
        scope: Global,
        description: i18n::Message::InteractionTextCommand,
    },
    Command {
        name: "schedule apply",
        action: Apply,
        scope: Global,
        description: i18n::Message::ApplySystemScheduleWithoutStartingACheck,
    },
    Command {
        name: "schedule enable",
        action: EnableSchedule,
        scope: Global,
        description: i18n::Message::EnableScheduledTask,
    },
    Command {
        name: "schedule disable",
        action: DisableSchedule,
        scope: Global,
        description: i18n::Message::DisableScheduledTask,
    },
    Command {
        name: "help",
        action: Help,
        scope: Global,
        description: i18n::Message::ShowHelp,
    },
    Command {
        name: "error copy",
        action: Copy(crate::clipboard::Scope::Error),
        scope: View,
        description: i18n::Message::CopyErrorCommand,
    },
    Command {
        name: "quit",
        action: Quit,
        scope: Global,
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
    matches.sort_by_key(|(kind, _)| *kind);
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
        } else if [i18n::Language::English, i18n::Language::Chinese]
            .into_iter()
            .any(|language| {
                self.description
                    .in_language(language)
                    .to_lowercase()
                    .contains(query)
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
            Self::General => &[Concurrency, StartupCheck],
            Self::Repositories => &[Add, Remove, Exclude, Unexclude, Scan, Import],
            Self::Schedule => match registration {
                engine::scheduler::Registration::Loaded => {
                    &[Interval, Anchor, Autopush, Apply, DisableSchedule]
                }
                engine::scheduler::Registration::NotLoaded => {
                    &[Interval, Anchor, Autopush, Apply, EnableSchedule]
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
    COMMANDS
        .iter()
        .find(|c| c.action == action)
        .expect("catalog covers every action")
        .description
        .text()
}
