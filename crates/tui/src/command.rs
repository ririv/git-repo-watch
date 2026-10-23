//! Explicit command catalog independent of terminal rendering.
mod argument;
pub(crate) mod path;
#[cfg(test)]
mod tests;
pub use argument::{ArgumentAction, argument_input};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Branches,
    Stage,
    Unstage,
    Diff,
    Log,
    LogMore,
    DiffLayoutMenu,
    DiffLayout(crate::diff::Preference),
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
        name: "branch",
        action: Action::Branches,
        scope: Repository,
        description: i18n::Message::BranchCommand,
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
        name: "repo",
        action: Repositories,
        scope: Global,
        description: i18n::Message::ManageRepositories,
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
    COMMANDS
        .iter()
        .filter(|c| c.name.starts_with(input) && (!input.is_empty() || !c.name.contains(' ')))
        .collect()
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
