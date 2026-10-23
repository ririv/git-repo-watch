//! Explicit command catalog independent of terminal rendering.
mod argument;
pub(crate) mod context;
mod input;
pub(crate) mod path;
#[cfg(test)]
mod tests;
pub use argument::{ArgumentAction, ConfigArgument, argument_input};
pub(crate) use context::Context;
pub(crate) use input::Input;
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
        action: Repositories,
        scope: Global,
        description: i18n::Message::ManageRepositories,
    },
    Command {
        name: "branch",
        action: Action::Branches,
        scope: Repository,
        description: i18n::Message::BranchCommand,
    },
    Command {
        name: "branch refresh",
        action: Refresh,
        scope: View,
        description: i18n::Message::RefreshContextView,
    },
    Command {
        name: "branch copy",
        action: Copy(crate::clipboard::Scope::Branch),
        scope: View,
        description: i18n::Message::CopyBranchCommand,
    },
    Command {
        name: "branch create",
        action: BranchCreate,
        scope: Repository,
        description: i18n::Message::BranchCreateCommand,
    },
    Command {
        name: "branch switch",
        action: BranchSwitch,
        scope: Repository,
        description: i18n::Message::BranchSwitchCommand,
    },
    Command {
        name: "log",
        action: Log,
        scope: Repository,
        description: i18n::Message::HistoryCommand,
    },
    Command {
        name: "log more",
        action: LogMore,
        scope: View,
        description: i18n::Message::HistoryMoreCommand,
    },
    Command {
        name: "log refresh",
        action: Refresh,
        scope: View,
        description: i18n::Message::RefreshContextView,
    },
    Command {
        name: "log copy",
        action: Copy(crate::clipboard::Scope::Log),
        scope: View,
        description: i18n::Message::CopyLogCommand,
    },
    Command {
        name: "diff",
        action: Diff,
        scope: Repository,
        description: i18n::Message::ChangesOpen,
    },
    Command {
        name: "diff stage",
        action: Stage,
        scope: Repository,
        description: i18n::Message::StageSelectedFiles,
    },
    Command {
        name: "diff unstage",
        action: Unstage,
        scope: Repository,
        description: i18n::Message::UnstageSelectedFiles,
    },
    Command {
        name: "diff layout",
        action: DiffLayoutMenu,
        scope: View,
        description: i18n::Message::ChangesLayout,
    },
    Command {
        name: "diff layout auto",
        action: DiffLayout(crate::diff::Preference::Auto),
        scope: View,
        description: i18n::Message::DiffChooseAuto,
    },
    Command {
        name: "diff layout split",
        action: DiffLayout(crate::diff::Preference::Split),
        scope: View,
        description: i18n::Message::DiffChooseSplit,
    },
    Command {
        name: "diff layout unified",
        action: DiffLayout(crate::diff::Preference::Unified),
        scope: View,
        description: i18n::Message::DiffChooseUnified,
    },
    Command {
        name: "diff refresh",
        action: Refresh,
        scope: View,
        description: i18n::Message::RefreshContextView,
    },
    Command {
        name: "diff copy",
        action: Copy(crate::clipboard::Scope::Diff),
        scope: View,
        description: i18n::Message::CopyDiffCommand,
    },
    Command {
        name: "commit",
        action: Commit,
        scope: Repository,
        description: i18n::Message::ReviewStagedContentsEnterAMessageAnd,
    },
    Command {
        name: "push",
        action: Push,
        scope: Repository,
        description: i18n::Message::ReviewCurrentBranchAndRemoteUpstreamThen,
    },
    Command {
        name: "fetch",
        action: Fetch,
        scope: Repository,
        description: i18n::Message::FetchCommand,
    },
    Command {
        name: "pull",
        action: Pull,
        scope: Repository,
        description: i18n::Message::PullCommand,
    },
    Command {
        name: "stash",
        action: Stashes,
        scope: Repository,
        description: i18n::Message::StashCommand,
    },
    Command {
        name: "stash refresh",
        action: Refresh,
        scope: View,
        description: i18n::Message::RefreshContextView,
    },
    Command {
        name: "stash copy",
        action: Copy(crate::clipboard::Scope::Stash),
        scope: View,
        description: i18n::Message::StashCopyCommand,
    },
    Command {
        name: "stash save",
        action: StashSave,
        scope: Repository,
        description: i18n::Message::StashSaveCommand,
    },
    Command {
        name: "stash save-all",
        action: StashSaveAll,
        scope: Repository,
        description: i18n::Message::StashSaveAllCommand,
    },
    Command {
        name: "stash apply",
        action: StashApply,
        scope: Repository,
        description: i18n::Message::StashApplyCommand,
    },
    Command {
        name: "stash pop",
        action: StashPop,
        scope: Repository,
        description: i18n::Message::StashPopCommand,
    },
    Command {
        name: "stash drop",
        action: StashDrop,
        scope: Repository,
        description: i18n::Message::StashDropCommand,
    },
    Command {
        name: "terminal",
        action: Terminal,
        scope: Repository,
        description: i18n::Message::OpenRepositoryTerminal,
    },
    Command {
        name: "check",
        action: Check,
        scope: Repository,
        description: i18n::Message::CheckSelectedRepository,
    },
    Command {
        name: "check all",
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
        name: "refresh",
        action: Refresh,
        scope: View,
        description: i18n::Message::ReloadReportOrFileList,
    },
    Command {
        name: "repo filter",
        action: FilterMenu,
        scope: View,
        description: i18n::Message::RepositoryFilterCommand,
    },
    Command {
        name: "repo filter all",
        action: Filter(crate::repository_list::Filter::All),
        scope: View,
        description: i18n::Message::RepositoryFilterAll,
    },
    Command {
        name: "repo filter problems",
        action: Filter(crate::repository_list::Filter::Problems),
        scope: View,
        description: i18n::Message::RepositoryFilterProblems,
    },
    Command {
        name: "repo filter clean",
        action: Filter(crate::repository_list::Filter::Clean),
        scope: View,
        description: i18n::Message::RepositoryFilterClean,
    },
    Command {
        name: "repo filter failed",
        action: Filter(crate::repository_list::Filter::Failed),
        scope: View,
        description: i18n::Message::RepositoryFilterFailed,
    },
    Command {
        name: "repo filter unchecked",
        action: Filter(crate::repository_list::Filter::Unchecked),
        scope: View,
        description: i18n::Message::RepositoryFilterUnchecked,
    },
    Command {
        name: "repo filter no-upstream",
        action: Filter(crate::repository_list::Filter::WithoutUpstream),
        scope: View,
        description: i18n::Message::RepositoryFilterNoUpstream,
    },
    Command {
        name: "repo filter pushes",
        action: Filter(crate::repository_list::Filter::Pushes),
        scope: View,
        description: i18n::Message::RepositoryFilterPushes,
    },
    Command {
        name: "repo sort",
        action: SortMenu,
        scope: View,
        description: i18n::Message::RepositorySortCommand,
    },
    Command {
        name: "repo sort priority",
        action: Sort(crate::repository_list::Order::Priority),
        scope: View,
        description: i18n::Message::RepositorySortPriority,
    },
    Command {
        name: "repo sort name",
        action: Sort(crate::repository_list::Order::Name),
        scope: View,
        description: i18n::Message::RepositorySortName,
    },
    Command {
        name: "repo sort path",
        action: Sort(crate::repository_list::Order::Path),
        scope: View,
        description: i18n::Message::RepositorySortPath,
    },
    Command {
        name: "repo sort updated",
        action: Sort(crate::repository_list::Order::Updated),
        scope: View,
        description: i18n::Message::RepositorySortUpdated,
    },
    Command {
        name: "repo refresh",
        action: RepositoryRefresh,
        scope: View,
        description: i18n::Message::RefreshRepositoryReport,
    },
    Command {
        name: "repo manage",
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
        name: "repo add",
        action: Add,
        scope: Global,
        description: i18n::Message::AddARepositoryRoot,
    },
    Command {
        name: "repo remove",
        action: Remove,
        scope: Repository,
        description: i18n::Message::UnregisterRepositoryKeepFilesOnDisk,
    },
    Command {
        name: "repo exclude",
        action: Exclude,
        scope: Global,
        description: i18n::Message::ExcludeDirectoryAndUnregisterRepositoriesWithinIt,
    },
    Command {
        name: "repo unexclude",
        action: Unexclude,
        scope: Global,
        description: i18n::Message::RemoveDirectoryExclusionWithoutImporting,
    },
    Command {
        name: "repo scan",
        action: Scan,
        scope: Global,
        description: i18n::Message::PreviewScanUsingGitIgnoreRules,
    },
    Command {
        name: "repo import",
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
            parent
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
