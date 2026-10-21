//! Explicit command catalog independent of terminal rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Stage,
    Unstage,
    Diff,
    Commit,
    Push,
    CheckAll,
    Refresh,
    Settings,
    Add,
    Remove,
    Exclude,
    Unexclude,
    Scan,
    Import,
    Interval,
    Autopush,
    Concurrency,
    Apply,
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
    pub description: gw_i18n::Message,
}
use Action::*;
use Scope::*;
pub const COMMANDS: &[Command] = &[
    Command {
        name: "stage",
        action: Stage,
        scope: Repository,
        description: gw_i18n::Message::SelectAndStageWorktreeFiles,
    },
    Command {
        name: "unstage",
        action: Unstage,
        scope: Repository,
        description: gw_i18n::Message::SelectAndUnstageIndexFiles,
    },
    Command {
        name: "diff",
        action: Diff,
        scope: Repository,
        description: gw_i18n::Message::ViewTheCurrentFileDiff,
    },
    Command {
        name: "commit",
        action: Commit,
        scope: Repository,
        description: gw_i18n::Message::ReviewStagedContentsEnterAMessageAnd,
    },
    Command {
        name: "push",
        action: Push,
        scope: Repository,
        description: gw_i18n::Message::ReviewCurrentBranchAndRemoteUpstreamThen,
    },
    Command {
        name: "check all",
        action: CheckAll,
        scope: Global,
        description: gw_i18n::Message::CheckAllRepositoriesAndPushAccordingTo,
    },
    Command {
        name: "refresh",
        action: Refresh,
        scope: View,
        description: gw_i18n::Message::ReloadReportOrFileList,
    },
    Command {
        name: "settings",
        action: Settings,
        scope: Global,
        description: gw_i18n::Message::OpenSettings,
    },
    Command {
        name: "add",
        action: Add,
        scope: Global,
        description: gw_i18n::Message::AddARepositoryRoot,
    },
    Command {
        name: "remove",
        action: Remove,
        scope: Repository,
        description: gw_i18n::Message::UnregisterRepositoryKeepFilesOnDisk,
    },
    Command {
        name: "exclude",
        action: Exclude,
        scope: Global,
        description: gw_i18n::Message::ExcludeDirectoryAndUnregisterRepositoriesWithinIt,
    },
    Command {
        name: "unexclude",
        action: Unexclude,
        scope: Global,
        description: gw_i18n::Message::RemoveDirectoryExclusionWithoutImporting,
    },
    Command {
        name: "scan",
        action: Scan,
        scope: Global,
        description: gw_i18n::Message::PreviewScanUsingGitIgnoreRules,
    },
    Command {
        name: "import",
        action: Import,
        scope: Global,
        description: gw_i18n::Message::ScanAndImportRepositoryList,
    },
    Command {
        name: "interval",
        action: Interval,
        scope: Global,
        description: gw_i18n::Message::SaveCheckInterval,
    },
    Command {
        name: "autopush",
        action: Autopush,
        scope: Global,
        description: gw_i18n::Message::SaveAutomaticPushPolicy,
    },
    Command {
        name: "concurrency",
        action: Concurrency,
        scope: Global,
        description: gw_i18n::Message::SetCheckConcurrencyLimit132,
    },
    Command {
        name: "apply",
        action: Apply,
        scope: Global,
        description: gw_i18n::Message::ApplySystemScheduleWithoutStartingACheck,
    },
    Command {
        name: "help",
        action: Help,
        scope: Global,
        description: gw_i18n::Message::ShowHelp,
    },
    Command {
        name: "quit",
        action: Quit,
        scope: Global,
        description: gw_i18n::Message::Quit,
    },
];
pub fn matches(input: &str) -> Vec<&'static Command> {
    COMMANDS
        .iter()
        .filter(|c| c.name.starts_with(input.trim()))
        .collect()
}
pub fn exact(input: &str) -> Option<Action> {
    COMMANDS
        .iter()
        .find(|c| c.name == input.trim())
        .map(|c| c.action)
}
pub const SETTINGS: &[Action] = &[
    Interval,
    Autopush,
    Concurrency,
    Apply,
    Add,
    Exclude,
    Unexclude,
    Scan,
    Import,
];
pub fn description(action: Action) -> &'static str {
    COMMANDS
        .iter()
        .find(|c| c.action == action)
        .expect("catalog covers every action")
        .description
        .text()
}
