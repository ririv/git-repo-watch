//! Complete bilingual catalog; each message has both languages.
use crate::{Language, language};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Message {
    NoUpstreamColumn,
    InstallationPathNotUtf8,
    CommandPathNotUtf8,
    FieldLabel,
    CliHelp,
    UnableToRegisterExitSignalE,
    OperationCancelled,
    ErrorE,
    ArgumentsMustBeUtf8,
    HomeRequiresAPath,
    CannotReadInstallationLocationRunInstallSh,
    HomeMustBeAnAbsolutePath,
    StatusOnlyAcceptsJson,
    NoCompleteReportYetWaitForThe,
    AddRequiresAtLeastOneRepositoryPath,
    AddedNRepositories,
    ConfigurationSavedRunGwInstallToApply,
    FoundRepositoriesFailedCouldNotBeRead,
    Checking,
    UnableToPrintResultE,
    SystemNotificationFailedN,
    CheckCompleteRepositories,
    InstalledNativeGwAndSystemScheduleChecking,
    ScheduleRemovedConfigurationReportsAndCommandRetained,
    UnknownCommandCommandRunGwHelp,
    ThisCommandDoesNotAcceptPositionalArguments,
    ThisCommandRequiresOneArgument,
    ReportFiltersAreMutuallyExclusive,
    UnknownReportArgumentValue,
    InvalidReportTimestampValue,
    SystemScheduleCheckProcessConfiguredIntervalEvery,
    Loaded,
    NotLoaded,
    Running,
    Idle,
    SystemScheduleFileMissing,
    ScheduledIntervalEveryHours,
    IntervalNotAppliedRunGwInstall,
    ExcludedDirectory,
    LatestReportNoCompleteReportYet,
    LastCheckLatestResultCleanNeedAttention,
    OtherRepositoriesFailedToCheckTheseCounts,
    SystemNotificationsFailedSeeGwReportPushes,
    LatestReportPredatesAutomaticPushTheNext,
    NoRepositoriesWereConfiguredForThisCheck,
    ReportScopeDiffersFromTheCurrentRepository,
    CheckedAt,
    ClassificationUpdatedAt,
    NoRemoteUpstream,
    SystemNotificationFailedError,
    PushedCommits,
    PushFailedPhaseError,
    CountedSeparately,
    UpstreamIsALocalBranch,
    UpstreamNotConfigured,
    DetailsGwReportPushes,
    SystemNotification,
    StagedUnstagedUntrackedFilesOrSubmoduleChanges,
    HeadIsDetached,
    NoLocalBranchCommitsToCheck,
    RemoteQueryStateIsMissing,
    RemoteUpstreamBranchDoesNotExist,
    RemoteChangedAfterFetchOrFetchRefspec,
    LocalBranchChangedAfterPushCheckAgain,
    AutomaticPushFailed,
    FailedPushHasNoError,
    AheadOfRemoteByCommitsBehindBy,
    InvalidRemoteBranchRecord,
    ServerReturnedADuplicateBranchReference,
    RemoteUpstreamBranchDoesNotExistAlt,
    ScanEncounteredANonUtf8Tracked,
    RepositoryHasNoWorktree,
    ScanRootIsNotADirectory,
    UnknownAutomaticPushPolicyS,
    AnAttentionResultRequiresAtLeastOne,
    CheckFailed,
    NeedsAttention,
    Clean,
    UnsupportedReportVersion,
    All,
    Problems,
    NoUpstream,
    Pushes,
    CheckIntervalMustBeAtLeast60,
    ConcurrencyMustBeBetween1And32,
    PathMustBeANormalizedAbsolutePath,
    DuplicatePath,
    RepositoryIsUnderAnExcludedDirectory,
    InvalidReportFormat,
    ACheckOrRepositoryOperationIsAlready,
    SavePathHasNoParentDirectory,
    CannotRead,
    InvalidJson,
    EnterAnAbsolutePath,
    EnterANormalizedPath,
    IntervalRequiresAUnitSMH,
    CheckIntervalMustBeAnIntegerOf,
    TaskQueueLockFailed,
    ProgressReceiverClosed,
    ResultReceiverClosed,
    CheckWorkerPanicked,
    CannotReadUserHomeDirectory,
    UserHomeDirectoryIsNotAnAbsolute,
    InstallationLocationIsNotAnAbsolutePath,
    QuerySystemSchedule,
    SchedulePathIsNotUtf8,
    ReadScheduledInterval,
    InvalidScheduledInterval,
    InvalidInstallationArguments,
    ExistingGwDoesNotMatchTheInstallation,
    InstallationPathHasNoParentDirectory,
    SchedulePathHasNoParentDirectory,
    TemporaryPathIsNotUtf8,
    ValidateSystemSchedule,
    UnloadOldSystemSchedule,
    LoadSystemSchedule,
    UnloadSystemSchedule,
    LabelFailed,
    UnableToStart,
    CannotAttachStdout,
    CannotAttachStderr,
    TimedOutSeconds,
    StdoutReaderPanicked,
    StderrReaderPanicked,
    CommitMessageMustNotBeEmptyOr,
    CommitCommandCompletedButTheRepositoryCould,
    CommitCommandCompletedButHeadCouldNot,
    CurrentBranchHasNoCommits,
    CurrentLocalBranchDoesNotExist,
    CurrentBranchHasNoRemoteUpstreamExplicitly,
    RemoteUpstreamReferenceIsInconsistentCheckFetch,
    CurrentBranchIsAheadCommitsAheadAnd,
    CurrentBranchMatchesTheRemoteNothingTo,
    BranchOrHeadChangedDuringPreparationStart,
    BranchOrHeadChangedReviewThePush,
    RemoteUrlChangedReviewThePushAgain,
    InvalidPushTarget,
    LocalBranchChangedAfterCheckingPushCancelled,
    RemoteBranchChangedAfterCheckingPushCancelled,
    LocalCommitIsNotADescendantOf,
    RemoteUrlChangedDuringValidationPushCancelled,
    LocalBranchChangedDuringValidationPushCancelled,
    PushCommandReturnedButTheRemoteCommit,
    NoStagedChangesUseStageToSelect,
    FilesInsertionsDeletions,
    BranchOrHeadChangedReviewTheCommit,
    StagedContentsChangedReviewTheCommitAgain,
    BranchReferenceIsNotUtf8,
    BranchNameIsNotUtf8,
    BranchHasNoCommit,
    RemoteNameIsNotUtf8,
    UpstreamReferenceIsNotUtf8,
    ReferenceHasNoObjectId,
    RemoteUrlIsNotUtf8,
    RemoteMustHaveExactlyOneFetchUrl,
    PushUrlIsNotUtf8,
    PushUrlDiffersFromFetchUrlOr,
    PushUrlDiffersFromTheCheckedUrl,
    BareRepositoriesAreNotSupported,
    ConfiguredPathIsNotTheRepositoryRoot,
    RepositoryHasAnUnfinishedGitOperation,
    RepositoryHasUnresolvedConflicts,
    HeadIsDetachedSwitchToALocal,
    HeadHasNoSymbolicBranch,
    HeadIsNotAttachedToALocal,
    GitPathIsNotUtf8And,
    CheckResultsAreIncompleteHistoricalReportWas,
    RepositoryIsUnderAnExcludedDirectoryAlt,
    NotInTheRepositoryList,
    DirectoryIsNotInTheExclusionList,
    NoGitRepositoriesFoundUnderThisDirectory,
    RepositoryIsNotCurrentlyRegistered,
    CommitCreatedButTheOperationRecordCould,
    PushCommandExecutedButTheOperationRecord,
    SelectAFileFirst,
    FilesBelongToDifferentSnapshotsRefreshAnd,
    FileAreaDoesNotMatchTheOperation,
    DirectoriesOnlySupportStagingSubmoduleCommits,
    SubmoduleHasNoCommit,
    StagingCompletedButRefreshFailed,
    FilePathUsesExternalFilterNameSuch,
    FilePathUsesWorkingTreeEncodingWhich,
    ChangeHasNoPath,
    RenameHasNoOriginalPath,
    CopyHasNoOriginalPath,
    StagedDiffIsMissing,
    WorktreeDiffIsMissing,
    BranchOrHeadChangedRefreshTheFile,
    IndexChangedRefreshTheFileList,
    SubmoduleNameIsNotUtf8,
    FileDoesNotBelongToThisSnapshot,
    NewSymlink,
    DirectoryOrSpecialFileTextDiffIs,
    LargeUntrackedFileBytesViewWithAn,
    DiffIsNotUtf8ViewWith,
    DiffExceeds128KibOnlyTheBeginning,
    NoTextDiffAvailablePossiblyBinaryOr,
    SelectAndStageWorktreeFiles,
    SelectAndUnstageIndexFiles,
    ViewTheCurrentFileDiff,
    ReviewStagedContentsEnterAMessageAnd,
    ReviewCurrentBranchAndRemoteUpstreamThen,
    CheckAllRepositoriesAndPushAccordingTo,
    ReloadReportOrFileList,
    OpenSettings,
    AddARepositoryRoot,
    UnregisterRepositoryKeepFilesOnDisk,
    ExcludeDirectoryAndUnregisterRepositoriesWithinIt,
    RemoveDirectoryExclusionWithoutImporting,
    PreviewScanUsingGitIgnoreRules,
    ScanAndImportRepositoryList,
    SaveCheckInterval,
    SaveAutomaticPushPolicy,
    SetCheckConcurrencyLimit132,
    ApplySystemScheduleWithoutStartingACheck,
    ShowHelp,
    Quit,
    GwTuiRequiresAnInteractiveTerminalUse,
    UnableToRestoreTerminalCursor,
    SystemNotificationFailedNotice,
    BackgroundWorkerTerminatedUnexpectedly,
    BackgroundWorkerPanicked,
    OperationInterruptedSomeWritesMayHaveCompleted,
    CommittedTheListStillShowsTheLast,
    ContentsChangedVerifyTheCommit,
    PushedTheListStillShowsTheLast,
    PushFailed,
    PhasePhaseError,
    RefreshFailedE,
    CheckCompleteCleanAttentionFailed,
    UnknownCommandUseTabToCompleteBefore,
    Cancelled,
    EnterACommitMessage,
    CommitStagedContents,
    PushCurrentBranch,
    RefreshFailed,
    SelectARepositoryFirst,
    ReadStagedContents,
    ValidateCurrentBranchAndRemoteUpstream,
    ReadRepositoryFiles,
    NoFilesInThisArea,
    ReadFileDiff,
    NoActionableFilesInThisArea,
    StageSelectedFiles,
    UnstageSelectedFiles,
    CheckAllRepositories,
    ApplySystemSchedule,
    SystemScheduleAppliedNoCheckStarted,
    UnregisterRepository,
    RepositoryUnregisteredFilesRetained,
    AddedRepositories,
    FoundRepositoriesSeeVarImportReportJson,
    ThisCommandDoesNotAcceptConfigurationInput,
    ConfigurationSavedApplyScheduleAfterChangingThe,
    RepositoryBranchScopeStagedContentsOnlyStaged,
    FilesCurrentState,
    Repositories,
    RepositoryDetails,
    Settings,
    Help,
    CheckingAlt,
    ScheduleNotLoaded,
    NoCompleteReportYet,
    CleanAttentionFailed,
    LastCheck,
    ReportScopeChanged,
    FilesSelected,
    Index,
    Worktree,
    NoChangesInThisArea,
    FileDiff,
    CommitPreview,
    CommitMessage,
    ConfirmCommit,
    RepositoryBranchRemoteUpstreamUrlPendingCommits,
    ConfirmPush,
    OperationResult,
    ConfigurationInput,
    CheckAllRegisteredRepositoriesConcurrencyLimitAutomatic,
    ApplySavedIntervalAndReloadTheSystem,
    UnregisterRepositoryFilesOnDiskAreRetained,
    ConfirmOperation,
    CurrentRepositoryIsNotInTheFiltered,
    Instructions,
    GwResizeTerminalToAtLeast60,
    CurrentRepo,
    Global,
    CurrentView,
    Commands,
    NoRepositorySelected,
    NoMatchingCommands,
    TuiHelp,
    Repos,
    Search,
    WorktreeChangesUnknownPushSuccessFailureCounts,
    NoRepositorySelectedAlt,
    NoRepositories,
    CleanAlt,
    Passed,
    Changes,
    OutOfSync,
    MissingUpstream,
    NoCommits,
    TlsFailed,
    RemoteChangedAfterFetchOrFetchRefspecAlt,
    RefMismatch,
    Remote,
    ListProblemsFirst,
    ListPathOrder,
    NoRepositoriesMatchAddToAddOne,
    Path,
    Status,
    Unchecked,
    NotIncludedInTheLastReportWait,
    RemoteTlsConnectionFailedSslErrorSyscall,
    UpstreamReferenceMismatchRemoteChangedOrFetch,
    CheckFailedSeeOriginalError,
    Reason,
    Unknown,
    RemoteCheck,
    Incomplete,
    BranchCounts,
    OriginalError,
    HeadState,
    CommitState,
    RemoteUpstream,
    RemoteSync,
    Branches,
    TracksALocalBranch,
    PushHistory,
    Branch,
    PushTarget,
    PushResult,
    PushedCommitsAlt,
    Validation,
    Transfer,
    Verification,
    PhasePhaseFailed,
    PushError,
    TabCompleteCandidatesEnterRunEscClear,
    SelectSpaceInsMarkCtrlAAll,
    ScrollEscFilesCommands,
    SelectEnterDetailsCtrlFFilterTab,
    SelectEnterEditTabReposCommands,
    PgdnScrollEscBackCommands,
    CommandsAlt,
    SearchFiles,
    SearchRepositories,
    TypeToSearchOrForCommands,
    ConfigurationInputAlt,
    EnterAConfigurationValue,
    EnterNextEscCancel,
    CommitMessageAlt,
    DescribeThisCommit,
    EnterConfirmEscCancelPgupPgdnReview,
    ConfirmOperationAlt,
    AwaitingConfirmation,
    YConfirmNEscCancel,
    ConfirmCommitAlt,
    AwaitingCommitConfirmation,
    YCommitNEscCancelPgupPgdn,
    ConfirmPushAlt,
    AwaitingPushConfirmation,
    YPushNEscCancelPgupPgdn,
    OperationIncomplete,
    SeeErrorDetails,
    PgdnFullErrorEnterEscBack,
    RunningAlt,
    Cancelling,
    EscCtrlCCancelAndWaitFor,
    ChecksAndPushes,
    EveryHours,
    SystemSchedule,
    CheckingAltAlt,
    AppliedIntervalHoursConfigurationNotYetApplied,
    ExcludedDirectories,
    AutoPushFailureTitle,
}
impl Message {
    pub fn text(self) -> &'static str {
        self.in_language(language())
    }
    pub const fn in_language(self, language: Language) -> &'static str {
        match (self, language) {
            (Self::NoUpstreamColumn, Language::English) => "No up",
            (Self::NoUpstreamColumn, Language::Chinese) => "无上游",
            (Self::InstallationPathNotUtf8, Language::English) => "Installation path is not UTF-8",
            (Self::InstallationPathNotUtf8, Language::Chinese) => "安装路径不是 UTF-8",
            (Self::CommandPathNotUtf8, Language::English) => "Command path is not UTF-8",
            (Self::CommandPathNotUtf8, Language::Chinese) => "命令路径不是 UTF-8",
            (Self::FieldLabel, Language::English) => "{label}: ",
            (Self::FieldLabel, Language::Chinese) => "{label}：",
            (Self::CliHelp, Language::English) => {
                "gw: Git repository status and configuration\n\nUsage: gw [--home ABSOLUTE_PATH] [COMMAND]\nWithout a command, opens the TUI. Use gw status for status queries.\n\n  tui                              Open the TUI without checking\n  status [--json]                   Show schedule, configuration and last result\n  report [--json] [FILTER]          Read the complete report\n    --problems                     Only attention or failed repositories\n    --no-upstream                  Branches without remote upstream\n    --pushes                       Automatic push results\n  config                           Show configuration JSON\n  list                             List repositories\n  exclusions                       List excluded directories\n  add PATH...                      Add repository roots\n  remove PATH                      Unregister a repository\n  exclude PATH                     Exclude directory and unregister its repositories\n  unexclude PATH                   Remove exclusion without importing\n  scan PATH                        Scan using Git ignore rules; save report\n  import-tree PATH                 Scan and import repositories\n  interval 6h                      Save interval (s, m, h, d)\n  autopush disabled|fast_forward    Save automatic push policy\n  concurrency 4                    Set concurrency limit (1–32)\n  check                            Check concurrently and push according to policy\n  install                          Install command and schedule without checking\n  uninstall                        Remove schedule; keep data and command\n\nLanguage: follows system preferences; unsupported languages use English.\nSet GW_LANG=en or GW_LANG=zh to select a language explicitly.\n"
            }
            (Self::CliHelp, Language::Chinese) => {
                "gw：Git 仓库状态查询与配置管理\n\n用法：gw [--home 绝对路径] [命令]\n不传命令时进入 TUI；状态查询使用 gw status。\n\n  tui                              打开交互界面，不自动检查\n  status [--json]                   查看任务、配置和上次结果\n  report [--json] [筛选]             读取完整报告\n    --problems                      只看需关注或检查失败\n    --no-upstream                   单独查看无远程上游的分支\n    --pushes                        只看自动推送结果\n  config                            查看配置 JSON\n  list                              查看仓库列表\n  exclusions                        查看排除目录\n  add 路径...                       添加仓库根目录\n  remove 路径                       移除仓库注册项\n  exclude 路径                      排除目录并移除其下注册项\n  unexclude 路径                    取消排除，不自动添加仓库\n  scan 路径                         按 Git 忽略规则扫描，写报告\n  import-tree 路径                  扫描并导入列表\n  interval 6h                       保存频率（支持 s、m、h、d）\n  autopush disabled|fast_forward     保存自动推送策略\n  concurrency 4                     设置检查并发上限（1 至 32）\n  check                             并行检查与配置的自动推送\n  install                           安装命令并应用调度，不立即检查\n  uninstall                         卸载定时任务，保留数据与命令\n"
            }
            (Self::UnableToRegisterExitSignalE, Language::English) => {
                "Unable to register exit signal: {e}"
            }
            (Self::UnableToRegisterExitSignalE, Language::Chinese) => "无法注册退出信号：{e}",
            (Self::OperationCancelled, Language::English) => "Operation cancelled",
            (Self::OperationCancelled, Language::Chinese) => "操作已取消",
            (Self::ErrorE, Language::English) => "Error: {e:#}",
            (Self::ErrorE, Language::Chinese) => "错误：{e:#}",
            (Self::ArgumentsMustBeUtf8, Language::English) => "Arguments must be UTF-8",
            (Self::ArgumentsMustBeUtf8, Language::Chinese) => "参数必须为 UTF-8",
            (Self::HomeRequiresAPath, Language::English) => "--home requires a path",
            (Self::HomeRequiresAPath, Language::Chinese) => "--home 缺少路径",
            (Self::CannotReadInstallationLocationRunInstallSh, Language::English) => {
                "Cannot read installation location; run ./install.sh or specify --home"
            }
            (Self::CannotReadInstallationLocationRunInstallSh, Language::Chinese) => {
                "无法读取安装位置，请运行 ./install.sh 或显式传入 --home"
            }
            (Self::HomeMustBeAnAbsolutePath, Language::English) => {
                "--home must be an absolute path"
            }
            (Self::HomeMustBeAnAbsolutePath, Language::Chinese) => "--home 必须是绝对路径",
            (Self::StatusOnlyAcceptsJson, Language::English) => "status only accepts --json",
            (Self::StatusOnlyAcceptsJson, Language::Chinese) => "status 只接受 --json",
            (Self::NoCompleteReportYetWaitForThe, Language::English) => {
                "No complete report yet; wait for the scheduled run or explicitly run check"
            }
            (Self::NoCompleteReportYetWaitForThe, Language::Chinese) => {
                "尚未生成完整报告；等待定时执行或显式运行 check"
            }
            (Self::AddRequiresAtLeastOneRepositoryPath, Language::English) => {
                "add requires at least one repository path"
            }
            (Self::AddRequiresAtLeastOneRepositoryPath, Language::Chinese) => {
                "add 至少需要一个仓库路径"
            }
            (Self::AddedNRepositories, Language::English) => "Added {n} repositories",
            (Self::AddedNRepositories, Language::Chinese) => "已添加 {n} 个仓库",
            (Self::ConfigurationSavedRunGwInstallToApply, Language::English) => {
                "Configuration saved; run gw install to apply interval changes without starting a check"
            }
            (Self::ConfigurationSavedRunGwInstallToApply, Language::Chinese) => {
                "配置已保存；间隔修改需运行 gw install 应用调度，不立即检查"
            }
            (Self::FoundRepositoriesFailedCouldNotBeRead, Language::English) => {
                "Found {} repositories; {failed} could not be read. See var/import-report.json"
            }
            (Self::FoundRepositoriesFailedCouldNotBeRead, Language::Chinese) => {
                "发现 {} 个仓库，其中 {failed} 个无法读取；详见 var/import-report.json"
            }
            (Self::Checking, Language::English) => "Checking: {}",
            (Self::Checking, Language::Chinese) => "正在检查：{}",
            (Self::UnableToPrintResultE, Language::English) => "Unable to print result: {e}",
            (Self::UnableToPrintResultE, Language::Chinese) => "无法输出结果：{e}",
            (Self::SystemNotificationFailedN, Language::English) => {
                "System notification failed: {n:?}"
            }
            (Self::SystemNotificationFailedN, Language::Chinese) => "系统通知发送失败：{n:?}",
            (Self::CheckCompleteRepositories, Language::English) => {
                "Check complete: {:?}, {} repositories"
            }
            (Self::CheckCompleteRepositories, Language::Chinese) => "检查完成：{:?}，共 {} 个仓库",
            (Self::InstalledNativeGwAndSystemScheduleChecking, Language::English) => {
                "Installed native gw and system schedule; checking every {} hours. No check started"
            }
            (Self::InstalledNativeGwAndSystemScheduleChecking, Language::Chinese) => {
                "已安装原生 gw 命令与系统任务，每 {} 小时检查一次；未触发检查"
            }
            (Self::ScheduleRemovedConfigurationReportsAndCommandRetained, Language::English) => {
                "Schedule removed; configuration, reports and command retained"
            }
            (Self::ScheduleRemovedConfigurationReportsAndCommandRetained, Language::Chinese) => {
                "已卸载定时任务，保留配置、报告和管理命令"
            }
            (Self::UnknownCommandCommandRunGwHelp, Language::English) => {
                "Unknown command {command:?}; run gw --help"
            }
            (Self::UnknownCommandCommandRunGwHelp, Language::Chinese) => {
                "未知命令 {command:?}；运行 gw --help 查看帮助"
            }
            (Self::ThisCommandDoesNotAcceptPositionalArguments, Language::English) => {
                "This command does not accept positional arguments"
            }
            (Self::ThisCommandDoesNotAcceptPositionalArguments, Language::Chinese) => {
                "该命令不接受位置参数"
            }
            (Self::ThisCommandRequiresOneArgument, Language::English) => {
                "This command requires one argument"
            }
            (Self::ThisCommandRequiresOneArgument, Language::Chinese) => "该命令要求一个参数",
            (Self::ReportFiltersAreMutuallyExclusive, Language::English) => {
                "Report filters are mutually exclusive"
            }
            (Self::ReportFiltersAreMutuallyExclusive, Language::Chinese) => "报告筛选参数互斥",
            (Self::UnknownReportArgumentValue, Language::English) => {
                "Unknown report argument: {value}"
            }
            (Self::UnknownReportArgumentValue, Language::Chinese) => "未知报告参数：{value}",
            (Self::InvalidReportTimestampValue, Language::English) => {
                "Invalid report timestamp: {value}"
            }
            (Self::InvalidReportTimestampValue, Language::Chinese) => "报告时间格式无效：{value}",
            (Self::SystemScheduleCheckProcessConfiguredIntervalEvery, Language::English) => {
                "System schedule: {}\nCheck process: {}\nConfigured interval: every {} hours\nRepositories: {}\nConcurrency: {}\nAutomatic push: {}\n"
            }
            (Self::SystemScheduleCheckProcessConfiguredIntervalEvery, Language::Chinese) => {
                "系统任务：{}\n检查进程：{}\n配置频率：每 {} 小时\n仓库数量：{}\n检查并发：{}\n自动推送：{}\n"
            }
            (Self::Loaded, Language::English) => "Loaded",
            (Self::Loaded, Language::Chinese) => "已加载",
            (Self::NotLoaded, Language::English) => "Not loaded",
            (Self::NotLoaded, Language::Chinese) => "未加载",
            (Self::Running, Language::English) => "Running",
            (Self::Running, Language::Chinese) => "运行中",
            (Self::Idle, Language::English) => "Idle",
            (Self::Idle, Language::Chinese) => "空闲",
            (Self::SystemScheduleFileMissing, Language::English) => {
                "System schedule file: missing\n"
            }
            (Self::SystemScheduleFileMissing, Language::Chinese) => "系统任务配置文件：不存在\n",
            (Self::ScheduledIntervalEveryHours, Language::English) => {
                "Scheduled interval: every {} hours\n"
            }
            (Self::ScheduledIntervalEveryHours, Language::Chinese) => "系统任务频率：每 {} 小时\n",
            (Self::IntervalNotAppliedRunGwInstall, Language::English) => {
                "Interval not applied; run gw install\n"
            }
            (Self::IntervalNotAppliedRunGwInstall, Language::Chinese) => {
                "频率尚未应用，请运行 gw install\n"
            }
            (Self::ExcludedDirectory, Language::English) => "Excluded directory: {}\n",
            (Self::ExcludedDirectory, Language::Chinese) => "排除目录：{}\n",
            (Self::LatestReportNoCompleteReportYet, Language::English) => {
                "Latest report: no complete report yet\n"
            }
            (Self::LatestReportNoCompleteReportYet, Language::Chinese) => {
                "最近报告：尚未生成完整报告\n"
            }
            (Self::LastCheckLatestResultCleanNeedAttention, Language::English) => {
                "Last check: {}\nLatest result: {} clean, {} need attention, {} failed\nLatest automatic pushes: {} succeeded, {} failed\nNo remote upstream (counted separately): {} branches across {} repositories\n"
            }
            (Self::LastCheckLatestResultCleanNeedAttention, Language::Chinese) => {
                "最近检查：{}\n最近结果：{} 个正常，{} 个需关注，{} 个检查失败\n最近自动推送：{} 个分支成功，{} 个分支失败\n未配置远程上游（单独统计）：{} 个分支，涉及 {} 个仓库\n"
            }
            (Self::OtherRepositoriesFailedToCheckTheseCounts, Language::English) => {
                "{} other repositories failed to check; these counts are incomplete\n"
            }
            (Self::OtherRepositoriesFailedToCheckTheseCounts, Language::Chinese) => {
                "另有 {} 个仓库检查失败，此项统计不完整\n"
            }
            (Self::SystemNotificationsFailedSeeGwReportPushes, Language::English) => {
                "{} system notifications failed; see gw report --pushes\n"
            }
            (Self::SystemNotificationsFailedSeeGwReportPushes, Language::Chinese) => {
                "系统通知发送失败：{} 条，详情见 gw report --pushes\n"
            }
            (Self::LatestReportPredatesAutomaticPushTheNext, Language::English) => {
                "Latest report predates automatic push; the next check will use the current policy\n"
            }
            (Self::LatestReportPredatesAutomaticPushTheNext, Language::Chinese) => {
                "最近报告来自自动推送启用前；将在下次检查时应用\n"
            }
            (Self::NoRepositoriesWereConfiguredForThisCheck, Language::English) => {
                "No repositories were configured for this check\n"
            }
            (Self::NoRepositoriesWereConfiguredForThisCheck, Language::Chinese) => {
                "该轮检查未配置仓库\n"
            }
            (Self::ReportScopeDiffersFromTheCurrentRepository, Language::English) => {
                "Report scope differs from the current repository list; wait for the next run or explicitly run check\n"
            }
            (Self::ReportScopeDiffersFromTheCurrentRepository, Language::Chinese) => {
                "报告范围与当前仓库列表不同，等待下一轮检查或显式运行 check\n"
            }
            (Self::CheckedAt, Language::English) => "Checked at: {}\n",
            (Self::CheckedAt, Language::Chinese) => "检查数据时间：{}\n",
            (Self::ClassificationUpdatedAt, Language::English) => "Classification updated at: {}\n",
            (Self::ClassificationUpdatedAt, Language::Chinese) => "分类规则更新时间：{}\n",
            (Self::NoRemoteUpstream, Language::English) => "No remote upstream",
            (Self::NoRemoteUpstream, Language::Chinese) => "未配置远程上游",
            (Self::SystemNotificationFailedError, Language::English) => {
                "  - System notification failed: {error}\n"
            }
            (Self::SystemNotificationFailedError, Language::Chinese) => {
                "  - 系统通知发送失败：{error}\n"
            }
            (Self::PushedCommits, Language::English) => "  - Pushed {} → {}/{}, {} commits\n",
            (Self::PushedCommits, Language::Chinese) => "  - 已推送 {} → {}/{}，{} 个提交\n",
            (Self::PushFailedPhaseError, Language::English) => {
                "  - Push failed {} ({phase:?}): {error}\n"
            }
            (Self::PushFailedPhaseError, Language::Chinese) => {
                "  - 推送失败 {}（{phase:?}）：{error}\n"
            }
            (Self::CountedSeparately, Language::English) => "  - {}: {} (counted separately)\n",
            (Self::CountedSeparately, Language::Chinese) => "  - {}：{}（单独统计）\n",
            (Self::UpstreamIsALocalBranch, Language::English) => "Upstream is a local branch",
            (Self::UpstreamIsALocalBranch, Language::Chinese) => "上游是本地分支",
            (Self::UpstreamNotConfigured, Language::English) => "Upstream not configured",
            (Self::UpstreamNotConfigured, Language::Chinese) => "未配置上游",
            (Self::DetailsGwReportPushes, Language::English) => "{}\nDetails: gw report --pushes",
            (Self::DetailsGwReportPushes, Language::Chinese) => "{}\n详情：gw report --pushes",
            (Self::SystemNotification, Language::English) => "System notification",
            (Self::SystemNotification, Language::Chinese) => "系统通知",
            (Self::StagedUnstagedUntrackedFilesOrSubmoduleChanges, Language::English) => {
                "Staged, unstaged, untracked files or submodule changes exist"
            }
            (Self::StagedUnstagedUntrackedFilesOrSubmoduleChanges, Language::Chinese) => {
                "存在暂存、未暂存、未跟踪文件或子模块改动"
            }
            (Self::HeadIsDetached, Language::English) => "HEAD is detached",
            (Self::HeadIsDetached, Language::Chinese) => "HEAD 处于游离状态",
            (Self::NoLocalBranchCommitsToCheck, Language::English) => {
                "No local branch commits to check"
            }
            (Self::NoLocalBranchCommitsToCheck, Language::Chinese) => "没有可检查的本地分支提交",
            (Self::RemoteQueryStateIsMissing, Language::English) => "Remote query state is missing",
            (Self::RemoteQueryStateIsMissing, Language::Chinese) => "远程查询状态丢失",
            (Self::RemoteUpstreamBranchDoesNotExist, Language::English) => {
                "{}: remote upstream branch does not exist"
            }
            (Self::RemoteUpstreamBranchDoesNotExist, Language::Chinese) => "{}：远程上游分支不存在",
            (Self::RemoteChangedAfterFetchOrFetchRefspec, Language::English) => {
                "{}: remote changed after fetch, or fetch refspec did not update upstream"
            }
            (Self::RemoteChangedAfterFetchOrFetchRefspec, Language::Chinese) => {
                "{}：远程在 fetch 后发生变化或 fetch refspec 未更新上游"
            }
            (Self::LocalBranchChangedAfterPushCheckAgain, Language::English) => {
                "{}: local branch changed after push; check again next time"
            }
            (Self::LocalBranchChangedAfterPushCheckAgain, Language::Chinese) => {
                "{}：推送后本地分支发生变化，待下次重新检查"
            }
            (Self::AutomaticPushFailed, Language::English) => "{}: automatic push failed: {}",
            (Self::AutomaticPushFailed, Language::Chinese) => "{}：自动推送失败：{}",
            (Self::FailedPushHasNoError, Language::English) => "Failed push has no error",
            (Self::FailedPushHasNoError, Language::Chinese) => "失败推送缺少错误",
            (Self::AheadOfRemoteByCommitsBehindBy, Language::English) => {
                "{}: ahead of remote by {} commits, behind by {} commits"
            }
            (Self::AheadOfRemoteByCommitsBehindBy, Language::Chinese) => {
                "{}：领先远程 {} 个提交，落后远程 {} 个提交"
            }
            (Self::InvalidRemoteBranchRecord, Language::English) => "Invalid remote branch record",
            (Self::InvalidRemoteBranchRecord, Language::Chinese) => "远程分支记录格式错误",
            (Self::ServerReturnedADuplicateBranchReference, Language::English) => {
                "Server returned a duplicate branch reference"
            }
            (Self::ServerReturnedADuplicateBranchReference, Language::Chinese) => {
                "服务器返回重复的分支引用"
            }
            (Self::RemoteUpstreamBranchDoesNotExistAlt, Language::English) => {
                "Remote upstream branch does not exist"
            }
            (Self::RemoteUpstreamBranchDoesNotExistAlt, Language::Chinese) => "远程上游分支不存在",
            (Self::ScanEncounteredANonUtf8Tracked, Language::English) => {
                "Scan encountered a non-UTF-8 tracked path"
            }
            (Self::ScanEncounteredANonUtf8Tracked, Language::Chinese) => {
                "扫描遇到非 UTF-8 的已跟踪路径"
            }
            (Self::RepositoryHasNoWorktree, Language::English) => "Repository has no worktree",
            (Self::RepositoryHasNoWorktree, Language::Chinese) => "仓库没有工作区",
            (Self::ScanRootIsNotADirectory, Language::English) => "Scan root is not a directory",
            (Self::ScanRootIsNotADirectory, Language::Chinese) => "扫描起点不是目录",
            (Self::UnknownAutomaticPushPolicyS, Language::English) => {
                "Unknown automatic push policy: {s}"
            }
            (Self::UnknownAutomaticPushPolicyS, Language::Chinese) => "未知自动推送策略：{s}",
            (Self::AnAttentionResultRequiresAtLeastOne, Language::English) => {
                "An attention result requires at least one issue"
            }
            (Self::AnAttentionResultRequiresAtLeastOne, Language::Chinese) => {
                "需关注仓库缺少有效问题列表"
            }
            (Self::CheckFailed, Language::English) => "Check failed",
            (Self::CheckFailed, Language::Chinese) => "检查失败",
            (Self::NeedsAttention, Language::English) => "Needs attention",
            (Self::NeedsAttention, Language::Chinese) => "需关注",
            (Self::Clean, Language::English) => "Clean",
            (Self::Clean, Language::Chinese) => "正常",
            (Self::UnsupportedReportVersion, Language::English) => "Unsupported report version: {}",
            (Self::UnsupportedReportVersion, Language::Chinese) => "不支持的报告版本：{}",
            (Self::All, Language::English) => "All",
            (Self::All, Language::Chinese) => "全部",
            (Self::Problems, Language::English) => "Problems",
            (Self::Problems, Language::Chinese) => "问题",
            (Self::NoUpstream, Language::English) => "No upstream",
            (Self::NoUpstream, Language::Chinese) => "无上游",
            (Self::Pushes, Language::English) => "Pushes",
            (Self::Pushes, Language::Chinese) => "推送",
            (Self::CheckIntervalMustBeAtLeast60, Language::English) => {
                "Check interval must be at least 60 seconds"
            }
            (Self::CheckIntervalMustBeAtLeast60, Language::Chinese) => "检查间隔至少为 60 秒",
            (Self::ConcurrencyMustBeBetween1And32, Language::English) => {
                "Concurrency must be between 1 and 32"
            }
            (Self::ConcurrencyMustBeBetween1And32, Language::Chinese) => {
                "并发数必须在 1 至 32 之间"
            }
            (Self::PathMustBeANormalizedAbsolutePath, Language::English) => {
                "Path must be a normalized absolute path: {}"
            }
            (Self::PathMustBeANormalizedAbsolutePath, Language::Chinese) => {
                "路径必须为规范绝对路径：{}"
            }
            (Self::DuplicatePath, Language::English) => "Duplicate path: {}",
            (Self::DuplicatePath, Language::Chinese) => "重复路径：{}",
            (Self::RepositoryIsUnderAnExcludedDirectory, Language::English) => {
                "Repository is under an excluded directory: {}"
            }
            (Self::RepositoryIsUnderAnExcludedDirectory, Language::Chinese) => {
                "仓库处于已排除目录：{}"
            }
            (Self::InvalidReportFormat, Language::English) => "Invalid report format",
            (Self::InvalidReportFormat, Language::Chinese) => "报告格式无效",
            (Self::ACheckOrRepositoryOperationIsAlready, Language::English) => {
                "A check or repository operation is already running"
            }
            (Self::ACheckOrRepositoryOperationIsAlready, Language::Chinese) => {
                "已有检查或仓库操作正在运行"
            }
            (Self::SavePathHasNoParentDirectory, Language::English) => {
                "Save path has no parent directory"
            }
            (Self::SavePathHasNoParentDirectory, Language::Chinese) => "保存路径缺少父目录",
            (Self::CannotRead, Language::English) => "Cannot read {}",
            (Self::CannotRead, Language::Chinese) => "无法读取 {}",
            (Self::InvalidJson, Language::English) => "Invalid JSON",
            (Self::InvalidJson, Language::Chinese) => "JSON 格式无效",
            (Self::EnterAnAbsolutePath, Language::English) => "Enter an absolute path",
            (Self::EnterAnAbsolutePath, Language::Chinese) => "请输入绝对路径",
            (Self::EnterANormalizedPath, Language::English) => "Enter a normalized path",
            (Self::EnterANormalizedPath, Language::Chinese) => "请输入规范路径",
            (Self::IntervalRequiresAUnitSMH, Language::English) => {
                "Interval requires a unit: s, m, h or d"
            }
            (Self::IntervalRequiresAUnitSMH, Language::Chinese) => "间隔必须带 s、m、h 或 d 单位",
            (Self::CheckIntervalMustBeAnIntegerOf, Language::English) => {
                "Check interval must be an integer of at least 60 seconds"
            }
            (Self::CheckIntervalMustBeAnIntegerOf, Language::Chinese) => {
                "检查间隔必须为至少 60 秒的整数"
            }
            (Self::TaskQueueLockFailed, Language::English) => "Task queue lock failed",
            (Self::TaskQueueLockFailed, Language::Chinese) => "任务队列锁异常",
            (Self::ProgressReceiverClosed, Language::English) => "Progress receiver closed",
            (Self::ProgressReceiverClosed, Language::Chinese) => "进度接收端已关闭",
            (Self::ResultReceiverClosed, Language::English) => "Result receiver closed",
            (Self::ResultReceiverClosed, Language::Chinese) => "结果接收端已关闭",
            (Self::CheckWorkerPanicked, Language::English) => "Check worker panicked",
            (Self::CheckWorkerPanicked, Language::Chinese) => "检查工作线程异常",
            (Self::CannotReadUserHomeDirectory, Language::English) => {
                "Cannot read user home directory"
            }
            (Self::CannotReadUserHomeDirectory, Language::Chinese) => "无法读取用户目录",
            (Self::UserHomeDirectoryIsNotAnAbsolute, Language::English) => {
                "User home directory is not an absolute path"
            }
            (Self::UserHomeDirectoryIsNotAnAbsolute, Language::Chinese) => "用户目录不是绝对路径",
            (Self::InstallationLocationIsNotAnAbsolutePath, Language::English) => {
                "Installation location is not an absolute path"
            }
            (Self::InstallationLocationIsNotAnAbsolutePath, Language::Chinese) => {
                "安装位置不是绝对路径"
            }
            (Self::QuerySystemSchedule, Language::English) => "Query system schedule",
            (Self::QuerySystemSchedule, Language::Chinese) => "查询系统任务",
            (Self::SchedulePathIsNotUtf8, Language::English) => "Schedule path is not UTF-8",
            (Self::SchedulePathIsNotUtf8, Language::Chinese) => "调度路径不是 UTF-8",
            (Self::ReadScheduledInterval, Language::English) => "Read scheduled interval",
            (Self::ReadScheduledInterval, Language::Chinese) => "读取系统任务间隔",
            (Self::InvalidScheduledInterval, Language::English) => "Invalid scheduled interval",
            (Self::InvalidScheduledInterval, Language::Chinese) => "系统任务间隔无效",
            (Self::InvalidInstallationArguments, Language::English) => {
                "Invalid installation arguments"
            }
            (Self::InvalidInstallationArguments, Language::Chinese) => "安装参数无效",
            (Self::ExistingGwDoesNotMatchTheInstallation, Language::English) => {
                "Existing gw does not match the installation manifest; ownership cannot be verified"
            }
            (Self::ExistingGwDoesNotMatchTheInstallation, Language::Chinese) => {
                "现有 gw 命令与安装清单不匹配，无法确认归属"
            }
            (Self::InstallationPathHasNoParentDirectory, Language::English) => {
                "Installation path has no parent directory"
            }
            (Self::InstallationPathHasNoParentDirectory, Language::Chinese) => "安装路径缺少父目录",
            (Self::SchedulePathHasNoParentDirectory, Language::English) => {
                "Schedule path has no parent directory"
            }
            (Self::SchedulePathHasNoParentDirectory, Language::Chinese) => "调度路径缺少父目录",
            (Self::TemporaryPathIsNotUtf8, Language::English) => "Temporary path is not UTF-8",
            (Self::TemporaryPathIsNotUtf8, Language::Chinese) => "临时路径不是 UTF-8",
            (Self::ValidateSystemSchedule, Language::English) => "Validate system schedule",
            (Self::ValidateSystemSchedule, Language::Chinese) => "验证系统任务配置",
            (Self::UnloadOldSystemSchedule, Language::English) => "Unload old system schedule",
            (Self::UnloadOldSystemSchedule, Language::Chinese) => "卸载旧系统任务",
            (Self::LoadSystemSchedule, Language::English) => "Load system schedule",
            (Self::LoadSystemSchedule, Language::Chinese) => "加载系统任务",
            (Self::UnloadSystemSchedule, Language::English) => "Unload system schedule",
            (Self::UnloadSystemSchedule, Language::Chinese) => "卸载系统任务",
            (Self::LabelFailed, Language::English) => "{label} failed ({}):\n{}\n{}",
            (Self::LabelFailed, Language::Chinese) => "{label} 失败（{}）：\n{}\n{}",
            (Self::UnableToStart, Language::English) => "Unable to start {}",
            (Self::UnableToStart, Language::Chinese) => "无法启动 {}",
            (Self::CannotAttachStdout, Language::English) => "Cannot attach stdout",
            (Self::CannotAttachStdout, Language::Chinese) => "无法连接 stdout",
            (Self::CannotAttachStderr, Language::English) => "Cannot attach stderr",
            (Self::CannotAttachStderr, Language::Chinese) => "无法连接 stderr",
            (Self::TimedOutSeconds, Language::English) => "{} timed out ({} seconds)",
            (Self::TimedOutSeconds, Language::Chinese) => "{} 超时（{} 秒）",
            (Self::StdoutReaderPanicked, Language::English) => "stdout reader panicked",
            (Self::StdoutReaderPanicked, Language::Chinese) => "stdout 读取线程异常",
            (Self::StderrReaderPanicked, Language::English) => "stderr reader panicked",
            (Self::StderrReaderPanicked, Language::Chinese) => "stderr 读取线程异常",
            (Self::CommitMessageMustNotBeEmptyOr, Language::English) => {
                "Commit message must not be empty or contain NUL"
            }
            (Self::CommitMessageMustNotBeEmptyOr, Language::Chinese) => {
                "提交说明不能为空且不能包含 NUL"
            }
            (Self::CommitCommandCompletedButTheRepositoryCould, Language::English) => {
                "Commit command completed, but the repository could not be reopened; verify the actual commit"
            }
            (Self::CommitCommandCompletedButTheRepositoryCould, Language::Chinese) => {
                "提交命令已完成，但无法重新打开仓库，请核对实际提交"
            }
            (Self::CommitCommandCompletedButHeadCouldNot, Language::English) => {
                "Commit command completed, but HEAD could not be verified; verify the actual commit"
            }
            (Self::CommitCommandCompletedButHeadCouldNot, Language::Chinese) => {
                "提交命令已完成，但无法核验 HEAD，请核对实际提交"
            }
            (Self::CurrentBranchHasNoCommits, Language::English) => "Current branch has no commits",
            (Self::CurrentBranchHasNoCommits, Language::Chinese) => "当前分支还没有提交",
            (Self::CurrentLocalBranchDoesNotExist, Language::English) => {
                "Current local branch does not exist"
            }
            (Self::CurrentLocalBranchDoesNotExist, Language::Chinese) => "当前本地分支不存在",
            (Self::CurrentBranchHasNoRemoteUpstreamExplicitly, Language::English) => {
                "Current branch has no remote upstream; explicitly configure tracking first"
            }
            (Self::CurrentBranchHasNoRemoteUpstreamExplicitly, Language::Chinese) => {
                "当前分支没有配置远程上游，请先显式配置跟踪关系"
            }
            (Self::RemoteUpstreamReferenceIsInconsistentCheckFetch, Language::English) => {
                "Remote upstream reference is inconsistent; check fetch configuration or try again later"
            }
            (Self::RemoteUpstreamReferenceIsInconsistentCheckFetch, Language::Chinese) => {
                "远程上游引用不一致，请检查 fetch 配置或稍后重试"
            }
            (Self::CurrentBranchIsAheadCommitsAheadAnd, Language::English) => {
                "Current branch is {ahead} commits ahead and {behind} behind; handle remote updates manually first"
            }
            (Self::CurrentBranchIsAheadCommitsAheadAnd, Language::Chinese) => {
                "当前分支领先 {ahead}、落后 {behind} 个提交；请先手动处理远程更新"
            }
            (Self::CurrentBranchMatchesTheRemoteNothingTo, Language::English) => {
                "Current branch matches the remote; nothing to push"
            }
            (Self::CurrentBranchMatchesTheRemoteNothingTo, Language::Chinese) => {
                "当前分支已与远程一致，没有待推送提交"
            }
            (Self::BranchOrHeadChangedDuringPreparationStart, Language::English) => {
                "Branch or HEAD changed during preparation; start again"
            }
            (Self::BranchOrHeadChangedDuringPreparationStart, Language::Chinese) => {
                "准备期间分支或 HEAD 已变化，请重新操作"
            }
            (Self::BranchOrHeadChangedReviewThePush, Language::English) => {
                "Branch or HEAD changed; review the push again"
            }
            (Self::BranchOrHeadChangedReviewThePush, Language::Chinese) => {
                "分支或 HEAD 已变化，请重新确认推送"
            }
            (Self::RemoteUrlChangedReviewThePushAgain, Language::English) => {
                "Remote URL changed; review the push again"
            }
            (Self::RemoteUrlChangedReviewThePushAgain, Language::Chinese) => {
                "远程地址已变化，请重新确认推送"
            }
            (Self::InvalidPushTarget, Language::English) => "Invalid push target",
            (Self::InvalidPushTarget, Language::Chinese) => "无效的推送目标",
            (Self::LocalBranchChangedAfterCheckingPushCancelled, Language::English) => {
                "Local branch changed after checking; push cancelled"
            }
            (Self::LocalBranchChangedAfterCheckingPushCancelled, Language::Chinese) => {
                "本地分支在检查后发生变化，取消推送"
            }
            (Self::RemoteBranchChangedAfterCheckingPushCancelled, Language::English) => {
                "Remote branch changed after checking; push cancelled"
            }
            (Self::RemoteBranchChangedAfterCheckingPushCancelled, Language::Chinese) => {
                "远程分支在检查后发生变化，取消推送"
            }
            (Self::LocalCommitIsNotADescendantOf, Language::English) => {
                "Local commit is not a descendant of the remote upstream"
            }
            (Self::LocalCommitIsNotADescendantOf, Language::Chinese) => {
                "本地提交不是远程上游的后代"
            }
            (Self::RemoteUrlChangedDuringValidationPushCancelled, Language::English) => {
                "Remote URL changed during validation; push cancelled"
            }
            (Self::RemoteUrlChangedDuringValidationPushCancelled, Language::Chinese) => {
                "远程地址在核验期间发生变化，取消推送"
            }
            (Self::LocalBranchChangedDuringValidationPushCancelled, Language::English) => {
                "Local branch changed during validation; push cancelled"
            }
            (Self::LocalBranchChangedDuringValidationPushCancelled, Language::Chinese) => {
                "本地分支在核验期间发生变化，取消推送"
            }
            (Self::PushCommandReturnedButTheRemoteCommit, Language::English) => {
                "Push command returned, but the remote commit does not match the expected commit"
            }
            (Self::PushCommandReturnedButTheRemoteCommit, Language::Chinese) => {
                "推送命令已返回，但远程提交与预期不一致"
            }
            (Self::NoStagedChangesUseStageToSelect, Language::English) => {
                "No staged changes; use stage to select commit contents"
            }
            (Self::NoStagedChangesUseStageToSelect, Language::Chinese) => {
                "没有已暂存的改动；请通过 stage 选择提交内容"
            }
            (Self::FilesInsertionsDeletions, Language::English) => {
                "{} files, {} insertions, {} deletions"
            }
            (Self::FilesInsertionsDeletions, Language::Chinese) => {
                "{} 个文件，新增 {} 行，删除 {} 行"
            }
            (Self::BranchOrHeadChangedReviewTheCommit, Language::English) => {
                "Branch or HEAD changed; review the commit again"
            }
            (Self::BranchOrHeadChangedReviewTheCommit, Language::Chinese) => {
                "分支或 HEAD 已变化，请重新查看提交内容"
            }
            (Self::StagedContentsChangedReviewTheCommitAgain, Language::English) => {
                "Staged contents changed; review the commit again"
            }
            (Self::StagedContentsChangedReviewTheCommitAgain, Language::Chinese) => {
                "暂存内容已变化，请重新查看提交内容"
            }
            (Self::BranchReferenceIsNotUtf8, Language::English) => "Branch reference is not UTF-8",
            (Self::BranchReferenceIsNotUtf8, Language::Chinese) => "分支引用不是 UTF-8",
            (Self::BranchNameIsNotUtf8, Language::English) => "Branch name is not UTF-8",
            (Self::BranchNameIsNotUtf8, Language::Chinese) => "分支名不是 UTF-8",
            (Self::BranchHasNoCommit, Language::English) => "Branch has no commit",
            (Self::BranchHasNoCommit, Language::Chinese) => "分支缺少提交",
            (Self::RemoteNameIsNotUtf8, Language::English) => "Remote name is not UTF-8",
            (Self::RemoteNameIsNotUtf8, Language::Chinese) => "远程名不是 UTF-8",
            (Self::UpstreamReferenceIsNotUtf8, Language::English) => {
                "Upstream reference is not UTF-8"
            }
            (Self::UpstreamReferenceIsNotUtf8, Language::Chinese) => "上游引用不是 UTF-8",
            (Self::ReferenceHasNoObjectId, Language::English) => "Reference has no object ID",
            (Self::ReferenceHasNoObjectId, Language::Chinese) => "引用缺少对象 ID",
            (Self::RemoteUrlIsNotUtf8, Language::English) => "Remote URL is not UTF-8",
            (Self::RemoteUrlIsNotUtf8, Language::Chinese) => "远程地址不是 UTF-8",
            (Self::RemoteMustHaveExactlyOneFetchUrl, Language::English) => {
                "Remote must have exactly one fetch URL"
            }
            (Self::RemoteMustHaveExactlyOneFetchUrl, Language::Chinese) => {
                "远程必须有且仅有一个获取地址"
            }
            (Self::PushUrlIsNotUtf8, Language::English) => "Push URL is not UTF-8",
            (Self::PushUrlIsNotUtf8, Language::Chinese) => "推送地址不是 UTF-8",
            (Self::PushUrlDiffersFromFetchUrlOr, Language::English) => {
                "Push URL differs from fetch URL, or multiple push URLs exist"
            }
            (Self::PushUrlDiffersFromFetchUrlOr, Language::Chinese) => {
                "推送地址与获取地址不同，或存在多个推送地址"
            }
            (Self::PushUrlDiffersFromTheCheckedUrl, Language::English) => {
                "Push URL differs from the checked URL"
            }
            (Self::PushUrlDiffersFromTheCheckedUrl, Language::Chinese) => "推送地址与检查地址不同",
            (Self::BareRepositoriesAreNotSupported, Language::English) => {
                "Bare repositories are not supported"
            }
            (Self::BareRepositoriesAreNotSupported, Language::Chinese) => "不支持裸仓库",
            (Self::ConfiguredPathIsNotTheRepositoryRoot, Language::English) => {
                "Configured path is not the repository root: {}"
            }
            (Self::ConfiguredPathIsNotTheRepositoryRoot, Language::Chinese) => {
                "配置路径不是仓库根目录：{}"
            }
            (Self::RepositoryHasAnUnfinishedGitOperation, Language::English) => {
                "Repository has an unfinished Git operation: {:?}"
            }
            (Self::RepositoryHasAnUnfinishedGitOperation, Language::Chinese) => {
                "仓库存在未完成的 Git 操作：{:?}"
            }
            (Self::RepositoryHasUnresolvedConflicts, Language::English) => {
                "Repository has unresolved conflicts"
            }
            (Self::RepositoryHasUnresolvedConflicts, Language::Chinese) => "仓库存在未解决的冲突",
            (Self::HeadIsDetachedSwitchToALocal, Language::English) => {
                "HEAD is detached; switch to a local branch first"
            }
            (Self::HeadIsDetachedSwitchToALocal, Language::Chinese) => {
                "HEAD 处于游离状态，请先切换到本地分支"
            }
            (Self::HeadHasNoSymbolicBranch, Language::English) => "HEAD has no symbolic branch",
            (Self::HeadHasNoSymbolicBranch, Language::Chinese) => "HEAD 缺少符号分支",
            (Self::HeadIsNotAttachedToALocal, Language::English) => {
                "HEAD is not attached to a local branch"
            }
            (Self::HeadIsNotAttachedToALocal, Language::Chinese) => "HEAD 未关联本地分支",
            (Self::GitPathIsNotUtf8And, Language::English) => {
                "Git path is not UTF-8 and cannot be displayed; handle this path with Git"
            }
            (Self::GitPathIsNotUtf8And, Language::Chinese) => {
                "Git 路径不是 UTF-8，当前界面无法表示；请使用 Git 处理该路径"
            }
            (Self::CheckResultsAreIncompleteHistoricalReportWas, Language::English) => {
                "Check results are incomplete; historical report was not overwritten"
            }
            (Self::CheckResultsAreIncompleteHistoricalReportWas, Language::Chinese) => {
                "检查结果不完整，不覆盖历史报告"
            }
            (Self::RepositoryIsUnderAnExcludedDirectoryAlt, Language::English) => {
                "Repository is under an excluded directory: {}"
            }
            (Self::RepositoryIsUnderAnExcludedDirectoryAlt, Language::Chinese) => {
                "仓库位于排除目录中：{}"
            }
            (Self::NotInTheRepositoryList, Language::English) => "Not in the repository list: {}",
            (Self::NotInTheRepositoryList, Language::Chinese) => "列表中不存在：{}",
            (Self::DirectoryIsNotInTheExclusionList, Language::English) => {
                "Directory is not in the exclusion list"
            }
            (Self::DirectoryIsNotInTheExclusionList, Language::Chinese) => "排除列表中不存在该目录",
            (Self::NoGitRepositoriesFoundUnderThisDirectory, Language::English) => {
                "No Git repositories found under this directory"
            }
            (Self::NoGitRepositoriesFoundUnderThisDirectory, Language::Chinese) => {
                "指定目录下未发现 Git 仓库"
            }
            (Self::RepositoryIsNotCurrentlyRegistered, Language::English) => {
                "Repository is not currently registered: {}"
            }
            (Self::RepositoryIsNotCurrentlyRegistered, Language::Chinese) => {
                "仓库不在当前注册列表中：{}"
            }
            (Self::CommitCreatedButTheOperationRecordCould, Language::English) => {
                "Commit created, but the operation record could not be saved; verify the actual commit"
            }
            (Self::CommitCreatedButTheOperationRecordCould, Language::Chinese) => {
                "提交已生成，但保存操作记录失败，请核对实际提交"
            }
            (Self::PushCommandExecutedButTheOperationRecord, Language::English) => {
                "Push command executed, but the operation record could not be saved; verify the remote state"
            }
            (Self::PushCommandExecutedButTheOperationRecord, Language::Chinese) => {
                "推送命令已执行，但保存操作记录失败，请核对远程状态"
            }
            (Self::SelectAFileFirst, Language::English) => "Select a file first",
            (Self::SelectAFileFirst, Language::Chinese) => "请先选择文件",
            (Self::FilesBelongToDifferentSnapshotsRefreshAnd, Language::English) => {
                "Files belong to different snapshots; refresh and select again"
            }
            (Self::FilesBelongToDifferentSnapshotsRefreshAnd, Language::Chinese) => {
                "文件来自不同的状态快照，请刷新后重新选择"
            }
            (Self::FileAreaDoesNotMatchTheOperation, Language::English) => {
                "File area does not match the operation"
            }
            (Self::FileAreaDoesNotMatchTheOperation, Language::Chinese) => "文件区域与操作不匹配",
            (Self::DirectoriesOnlySupportStagingSubmoduleCommits, Language::English) => {
                "Directories only support staging submodule commits"
            }
            (Self::DirectoriesOnlySupportStagingSubmoduleCommits, Language::Chinese) => {
                "目录只支持暂存子模块提交"
            }
            (Self::SubmoduleHasNoCommit, Language::English) => "Submodule has no commit",
            (Self::SubmoduleHasNoCommit, Language::Chinese) => "子模块没有提交",
            (Self::StagingCompletedButRefreshFailed, Language::English) => {
                "Staging completed, but refresh failed"
            }
            (Self::StagingCompletedButRefreshFailed, Language::Chinese) => {
                "暂存操作已执行，但刷新失败"
            }
            (Self::FilePathUsesExternalFilterNameSuch, Language::English) => {
                "File {path:?} uses external filter {name:?} (such as Git LFS), which is not registered in this libgit2 index backend; staging refused. Handle it explicitly with Git"
            }
            (Self::FilePathUsesExternalFilterNameSuch, Language::Chinese) => {
                "文件 {path:?} 使用外部 filter {name:?}（例如 Git LFS），当前 libgit2 索引后端未注册该过滤器，已拒绝暂存；请使用 Git 显式处理"
            }
            (Self::FilePathUsesWorkingTreeEncodingWhich, Language::English) => {
                "File {path:?} uses working-tree-encoding, which this backend does not support; staging refused"
            }
            (Self::FilePathUsesWorkingTreeEncodingWhich, Language::Chinese) => {
                "文件 {path:?} 使用 working-tree-encoding，当前后端不支持该转换，已拒绝暂存"
            }
            (Self::ChangeHasNoPath, Language::English) => "Change has no path",
            (Self::ChangeHasNoPath, Language::Chinese) => "变更缺少路径",
            (Self::RenameHasNoOriginalPath, Language::English) => "Rename has no original path",
            (Self::RenameHasNoOriginalPath, Language::Chinese) => "重命名缺少原路径",
            (Self::CopyHasNoOriginalPath, Language::English) => "Copy has no original path",
            (Self::CopyHasNoOriginalPath, Language::Chinese) => "复制缺少原路径",
            (Self::StagedDiffIsMissing, Language::English) => "Staged diff is missing",
            (Self::StagedDiffIsMissing, Language::Chinese) => "缺少暂存差异",
            (Self::WorktreeDiffIsMissing, Language::English) => "Worktree diff is missing",
            (Self::WorktreeDiffIsMissing, Language::Chinese) => "缺少工作区差异",
            (Self::BranchOrHeadChangedRefreshTheFile, Language::English) => {
                "Branch or HEAD changed; refresh the file list"
            }
            (Self::BranchOrHeadChangedRefreshTheFile, Language::Chinese) => {
                "分支或 HEAD 已变化，请刷新文件列表"
            }
            (Self::IndexChangedRefreshTheFileList, Language::English) => {
                "Index changed; refresh the file list"
            }
            (Self::IndexChangedRefreshTheFileList, Language::Chinese) => {
                "暂存区已变化，请刷新文件列表"
            }
            (Self::SubmoduleNameIsNotUtf8, Language::English) => "Submodule name is not UTF-8",
            (Self::SubmoduleNameIsNotUtf8, Language::Chinese) => "子模块名称不是 UTF-8",
            (Self::FileDoesNotBelongToThisSnapshot, Language::English) => {
                "File does not belong to this snapshot"
            }
            (Self::FileDoesNotBelongToThisSnapshot, Language::Chinese) => "文件不属于该快照",
            (Self::NewSymlink, Language::English) => "New symlink {:?} → {:?}",
            (Self::NewSymlink, Language::Chinese) => "新符号链接 {:?} → {:?}",
            (Self::DirectoryOrSpecialFileTextDiffIs, Language::English) => {
                "Directory or special file; text diff is unavailable"
            }
            (Self::DirectoryOrSpecialFileTextDiffIs, Language::Chinese) => {
                "目录或特殊文件，无法显示文本 diff"
            }
            (Self::LargeUntrackedFileBytesViewWithAn, Language::English) => {
                "Large untracked file ({} bytes); view with an external tool"
            }
            (Self::LargeUntrackedFileBytesViewWithAn, Language::Chinese) => {
                "未跟踪大文件（{} 字节），请使用外部工具查看"
            }
            (Self::DiffIsNotUtf8ViewWith, Language::English) => {
                "Diff is not UTF-8; view with an external tool"
            }
            (Self::DiffIsNotUtf8ViewWith, Language::Chinese) => {
                "差异内容不是 UTF-8，请使用外部工具查看"
            }
            (Self::DiffExceeds128KibOnlyTheBeginning, Language::English) => {
                "\n… Diff exceeds 128 KiB; only the beginning is shown."
            }
            (Self::DiffExceeds128KibOnlyTheBeginning, Language::Chinese) => {
                "\n… diff 超过 128 KiB，仅显示前段。"
            }
            (Self::NoTextDiffAvailablePossiblyBinaryOr, Language::English) => {
                "No text diff available (possibly binary or submodule changes)"
            }
            (Self::NoTextDiffAvailablePossiblyBinaryOr, Language::Chinese) => {
                "当前文件没有可显示的文本差异（可能为二进制或子模块状态变化）"
            }
            (Self::SelectAndStageWorktreeFiles, Language::English) => {
                "Select and stage worktree files"
            }
            (Self::SelectAndStageWorktreeFiles, Language::Chinese) => "选择工作区文件并暂存",
            (Self::SelectAndUnstageIndexFiles, Language::English) => {
                "Select and unstage index files"
            }
            (Self::SelectAndUnstageIndexFiles, Language::Chinese) => "选择暂存区文件并取消暂存",
            (Self::ViewTheCurrentFileDiff, Language::English) => "View the current file diff",
            (Self::ViewTheCurrentFileDiff, Language::Chinese) => "查看光标文件的差异",
            (Self::ReviewStagedContentsEnterAMessageAnd, Language::English) => {
                "Review staged contents, enter a message and confirm commit"
            }
            (Self::ReviewStagedContentsEnterAMessageAnd, Language::Chinese) => {
                "预览暂存内容，输入说明并确认提交"
            }
            (Self::ReviewCurrentBranchAndRemoteUpstreamThen, Language::English) => {
                "Review current branch and remote upstream, then confirm push"
            }
            (Self::ReviewCurrentBranchAndRemoteUpstreamThen, Language::Chinese) => {
                "核对当前分支和远程上游后确认推送"
            }
            (Self::CheckAllRepositoriesAndPushAccordingTo, Language::English) => {
                "Check all repositories and push according to policy"
            }
            (Self::CheckAllRepositoriesAndPushAccordingTo, Language::Chinese) => {
                "检查全部仓库，按当前配置自动推送"
            }
            (Self::ReloadReportOrFileList, Language::English) => "Reload report or file list",
            (Self::ReloadReportOrFileList, Language::Chinese) => "重新读取报告或文件列表",
            (Self::OpenSettings, Language::English) => "Open settings",
            (Self::OpenSettings, Language::Chinese) => "打开设置",
            (Self::AddARepositoryRoot, Language::English) => "Add a repository root",
            (Self::AddARepositoryRoot, Language::Chinese) => "添加仓库根目录",
            (Self::UnregisterRepositoryKeepFilesOnDisk, Language::English) => {
                "Unregister repository; keep files on disk"
            }
            (Self::UnregisterRepositoryKeepFilesOnDisk, Language::Chinese) => {
                "移除注册项，保留磁盘文件"
            }
            (Self::ExcludeDirectoryAndUnregisterRepositoriesWithinIt, Language::English) => {
                "Exclude directory and unregister repositories within it"
            }
            (Self::ExcludeDirectoryAndUnregisterRepositoriesWithinIt, Language::Chinese) => {
                "排除目录并移除其下注册项"
            }
            (Self::RemoveDirectoryExclusionWithoutImporting, Language::English) => {
                "Remove directory exclusion without importing"
            }
            (Self::RemoveDirectoryExclusionWithoutImporting, Language::Chinese) => {
                "取消目录排除，不自动导入"
            }
            (Self::PreviewScanUsingGitIgnoreRules, Language::English) => {
                "Preview scan using Git ignore rules"
            }
            (Self::PreviewScanUsingGitIgnoreRules, Language::Chinese) => "按 Git 忽略规则扫描预览",
            (Self::ScanAndImportRepositoryList, Language::English) => {
                "Scan and import repository list"
            }
            (Self::ScanAndImportRepositoryList, Language::Chinese) => "扫描并导入仓库列表",
            (Self::SaveCheckInterval, Language::English) => "Save check interval",
            (Self::SaveCheckInterval, Language::Chinese) => "保存检查频率",
            (Self::SaveAutomaticPushPolicy, Language::English) => "Save automatic push policy",
            (Self::SaveAutomaticPushPolicy, Language::Chinese) => "保存自动推送策略",
            (Self::SetCheckConcurrencyLimit132, Language::English) => {
                "Set check concurrency limit (1–32)"
            }
            (Self::SetCheckConcurrencyLimit132, Language::Chinese) => "设置并发检查上限（1 至 32）",
            (Self::ApplySystemScheduleWithoutStartingACheck, Language::English) => {
                "Apply system schedule without starting a check"
            }
            (Self::ApplySystemScheduleWithoutStartingACheck, Language::Chinese) => {
                "应用系统调度，不立即检查"
            }
            (Self::ShowHelp, Language::English) => "Show help",
            (Self::ShowHelp, Language::Chinese) => "查看帮助",
            (Self::Quit, Language::English) => "Quit",
            (Self::Quit, Language::Chinese) => "退出",
            (Self::GwTuiRequiresAnInteractiveTerminalUse, Language::English) => {
                "gw tui requires an interactive terminal; use gw status or gw report"
            }
            (Self::GwTuiRequiresAnInteractiveTerminalUse, Language::Chinese) => {
                "gw tui 需要交互式终端；请使用 gw status 或 gw report 查询"
            }
            (Self::UnableToRestoreTerminalCursor, Language::English) => {
                "Unable to restore terminal cursor"
            }
            (Self::UnableToRestoreTerminalCursor, Language::Chinese) => "无法恢复终端光标",
            (Self::SystemNotificationFailedNotice, Language::English) => {
                "System notification failed: {notice:?}"
            }
            (Self::SystemNotificationFailedNotice, Language::Chinese) => {
                "系统通知发送失败：{notice:?}"
            }
            (Self::BackgroundWorkerTerminatedUnexpectedly, Language::English) => {
                "Background worker terminated unexpectedly"
            }
            (Self::BackgroundWorkerTerminatedUnexpectedly, Language::Chinese) => {
                "后台工作线程异常退出"
            }
            (Self::BackgroundWorkerPanicked, Language::English) => "Background worker panicked",
            (Self::BackgroundWorkerPanicked, Language::Chinese) => "后台工作线程发生异常",
            (Self::OperationInterruptedSomeWritesMayHaveCompleted, Language::English) => {
                "Operation interrupted; some writes may have completed. Refresh and verify actual state.\n{e:#}"
            }
            (Self::OperationInterruptedSomeWritesMayHaveCompleted, Language::Chinese) => {
                "操作被中断，可能已完成部分写入，请刷新并核对实际状态。\n{e:#}"
            }
            (Self::CommittedTheListStillShowsTheLast, Language::English) => {
                "Committed {}{}; the list still shows the last check"
            }
            (Self::CommittedTheListStillShowsTheLast, Language::Chinese) => {
                "已提交 {}{}；列表仍为上次检查结果"
            }
            (Self::ContentsChangedVerifyTheCommit, Language::English) => {
                " (contents changed; verify the commit)"
            }
            (Self::ContentsChangedVerifyTheCommit, Language::Chinese) => {
                "（实际内容发生变化，请核对提交）"
            }
            (Self::PushedTheListStillShowsTheLast, Language::English) => {
                "Pushed {} → {}/{}; the list still shows the last check"
            }
            (Self::PushedTheListStillShowsTheLast, Language::Chinese) => {
                "已推送 {} → {}/{}；列表仍为上次检查结果"
            }
            (Self::PushFailed, Language::English) => "Push failed",
            (Self::PushFailed, Language::Chinese) => "推送失败",
            (Self::PhasePhaseError, Language::English) => "{phase:?} phase: {error}",
            (Self::PhasePhaseError, Language::Chinese) => "{phase:?} 阶段：{error}",
            (Self::RefreshFailedE, Language::English) => "; refresh failed: {e:#}",
            (Self::RefreshFailedE, Language::Chinese) => "；刷新失败：{e:#}",
            (Self::CheckCompleteCleanAttentionFailed, Language::English) => {
                "Check complete: {} clean · {} attention · {} failed"
            }
            (Self::CheckCompleteCleanAttentionFailed, Language::Chinese) => {
                "检查完成：{} 正常 · {} 需关注 · {} 失败"
            }
            (Self::UnknownCommandUseTabToCompleteBefore, Language::English) => {
                "Unknown command; use Tab to complete before executing"
            }
            (Self::UnknownCommandUseTabToCompleteBefore, Language::Chinese) => {
                "未知命令；按 Tab 补全后再执行"
            }
            (Self::Cancelled, Language::English) => "Cancelled",
            (Self::Cancelled, Language::Chinese) => "已取消",
            (Self::EnterACommitMessage, Language::English) => "Enter a commit message",
            (Self::EnterACommitMessage, Language::Chinese) => "请输入提交说明",
            (Self::CommitStagedContents, Language::English) => "Commit staged contents",
            (Self::CommitStagedContents, Language::Chinese) => "提交暂存内容",
            (Self::PushCurrentBranch, Language::English) => "Push current branch",
            (Self::PushCurrentBranch, Language::Chinese) => "推送当前分支",
            (Self::RefreshFailed, Language::English) => "Refresh failed",
            (Self::RefreshFailed, Language::Chinese) => "刷新失败",
            (Self::SelectARepositoryFirst, Language::English) => "Select a repository first",
            (Self::SelectARepositoryFirst, Language::Chinese) => "请先选择仓库",
            (Self::ReadStagedContents, Language::English) => "Read staged contents",
            (Self::ReadStagedContents, Language::Chinese) => "读取暂存内容",
            (Self::ValidateCurrentBranchAndRemoteUpstream, Language::English) => {
                "Validate current branch and remote upstream"
            }
            (Self::ValidateCurrentBranchAndRemoteUpstream, Language::Chinese) => {
                "核对当前分支与远程上游"
            }
            (Self::ReadRepositoryFiles, Language::English) => "Read repository files",
            (Self::ReadRepositoryFiles, Language::Chinese) => "读取仓库文件",
            (Self::NoFilesInThisArea, Language::English) => "No files in this area",
            (Self::NoFilesInThisArea, Language::Chinese) => "当前区域没有文件",
            (Self::ReadFileDiff, Language::English) => "Read file diff",
            (Self::ReadFileDiff, Language::Chinese) => "读取文件差异",
            (Self::NoActionableFilesInThisArea, Language::English) => {
                "No actionable files in this area"
            }
            (Self::NoActionableFilesInThisArea, Language::Chinese) => "当前区域没有可操作文件",
            (Self::StageSelectedFiles, Language::English) => "Stage selected files",
            (Self::StageSelectedFiles, Language::Chinese) => "暂存所选文件",
            (Self::UnstageSelectedFiles, Language::English) => "Unstage selected files",
            (Self::UnstageSelectedFiles, Language::Chinese) => "取消暂存所选文件",
            (Self::CheckAllRepositories, Language::English) => "Check all repositories",
            (Self::CheckAllRepositories, Language::Chinese) => "检查全部仓库",
            (Self::ApplySystemSchedule, Language::English) => "Apply system schedule",
            (Self::ApplySystemSchedule, Language::Chinese) => "应用系统调度",
            (Self::SystemScheduleAppliedNoCheckStarted, Language::English) => {
                "System schedule applied; no check started"
            }
            (Self::SystemScheduleAppliedNoCheckStarted, Language::Chinese) => {
                "系统调度已应用；未触发检查"
            }
            (Self::UnregisterRepository, Language::English) => "Unregister repository",
            (Self::UnregisterRepository, Language::Chinese) => "移除注册项",
            (Self::RepositoryUnregisteredFilesRetained, Language::English) => {
                "Repository unregistered; files retained"
            }
            (Self::RepositoryUnregisteredFilesRetained, Language::Chinese) => {
                "已移除注册项，保留磁盘文件"
            }
            (Self::AddedRepositories, Language::English) => "Added {} repositories",
            (Self::AddedRepositories, Language::Chinese) => "已添加 {} 个仓库",
            (Self::FoundRepositoriesSeeVarImportReportJson, Language::English) => {
                "Found {} repositories; see var/import-report.json"
            }
            (Self::FoundRepositoriesSeeVarImportReportJson, Language::Chinese) => {
                "发现 {} 个仓库；详见 var/import-report.json"
            }
            (Self::ThisCommandDoesNotAcceptConfigurationInput, Language::English) => {
                "This command does not accept configuration input"
            }
            (Self::ThisCommandDoesNotAcceptConfigurationInput, Language::Chinese) => {
                "此命令不接受配置输入"
            }
            (Self::ConfigurationSavedApplyScheduleAfterChangingThe, Language::English) => {
                "Configuration saved; apply schedule after changing the interval"
            }
            (Self::ConfigurationSavedApplyScheduleAfterChangingThe, Language::Chinese) => {
                "配置已保存；间隔修改后需应用调度"
            }
            (Self::RepositoryBranchScopeStagedContentsOnlyStaged, Language::English) => {
                "Repository: {}\nBranch: {}\nScope: staged contents only\n\nStaged files\n"
            }
            (Self::RepositoryBranchScopeStagedContentsOnlyStaged, Language::Chinese) => {
                "仓库：{}\n分支：{}\n范围：仅已暂存内容\n\n暂存文件\n"
            }
            (Self::FilesCurrentState, Language::English) => "Files · current state",
            (Self::FilesCurrentState, Language::Chinese) => "文件 · 当前状态",
            (Self::Repositories, Language::English) => "Repositories",
            (Self::Repositories, Language::Chinese) => "仓库",
            (Self::RepositoryDetails, Language::English) => "Repository details",
            (Self::RepositoryDetails, Language::Chinese) => "仓库详情",
            (Self::Settings, Language::English) => "Settings",
            (Self::Settings, Language::Chinese) => "设置",
            (Self::Help, Language::English) => "Help",
            (Self::Help, Language::Chinese) => "帮助",
            (Self::CheckingAlt, Language::English) => " · Checking",
            (Self::CheckingAlt, Language::Chinese) => " · 正在检查",
            (Self::ScheduleNotLoaded, Language::English) => " · Schedule not loaded",
            (Self::ScheduleNotLoaded, Language::Chinese) => " · 定时任务未加载",
            (Self::NoCompleteReportYet, Language::English) => " No complete report yet",
            (Self::NoCompleteReportYet, Language::Chinese) => " 尚无完整报告",
            (Self::CleanAttentionFailed, Language::English) => {
                " Clean {} · Attention {} · Failed {}"
            }
            (Self::CleanAttentionFailed, Language::Chinese) => " 正常 {} · 需关注 {} · 失败 {}",
            (Self::LastCheck, Language::English) => " Last check: {}{}",
            (Self::LastCheck, Language::Chinese) => " 上次检查：{}{}",
            (Self::ReportScopeChanged, Language::English) => " · Report scope changed",
            (Self::ReportScopeChanged, Language::Chinese) => " · 报告范围已变化",
            (Self::FilesSelected, Language::English) => "{} · {} files · {} selected",
            (Self::FilesSelected, Language::Chinese) => "{} · {} 个文件 · 已选 {}",
            (Self::Index, Language::English) => "Index",
            (Self::Index, Language::Chinese) => "暂存区",
            (Self::Worktree, Language::English) => "Worktree",
            (Self::Worktree, Language::Chinese) => "工作区",
            (Self::NoChangesInThisArea, Language::English) => "No changes in this area",
            (Self::NoChangesInThisArea, Language::Chinese) => "此区域没有变更",
            (Self::FileDiff, Language::English) => "File diff",
            (Self::FileDiff, Language::Chinese) => "文件差异",
            (Self::CommitPreview, Language::English) => "Commit preview",
            (Self::CommitPreview, Language::Chinese) => "提交预览",
            (Self::CommitMessage, Language::English) => "Commit message: {}\n\n{}",
            (Self::CommitMessage, Language::Chinese) => "提交说明：{}\n\n{}",
            (Self::ConfirmCommit, Language::English) => "Confirm commit",
            (Self::ConfirmCommit, Language::Chinese) => "确认提交",
            (Self::RepositoryBranchRemoteUpstreamUrlPendingCommits, Language::English) => {
                "Repository: {}\nBranch: {}\nRemote: {}\nUpstream: {}\nURL: {}\nPending: {} commits\n\nOnly push the reviewed commit snapshot; normal push with pre-push hooks preserved."
            }
            (Self::RepositoryBranchRemoteUpstreamUrlPendingCommits, Language::Chinese) => {
                "仓库：{}\n分支：{}\n远程：{}\n上游：{}\n地址：{}\n待推送：{} 个提交\n\n仅发布预览时的提交快照；普通推送，保留 pre-push 钩子。"
            }
            (Self::ConfirmPush, Language::English) => "Confirm push",
            (Self::ConfirmPush, Language::Chinese) => "确认推送",
            (Self::OperationResult, Language::English) => "Operation result",
            (Self::OperationResult, Language::Chinese) => "操作结果",
            (Self::ConfigurationInput, Language::English) => "Configuration input",
            (Self::ConfigurationInput, Language::Chinese) => "输入配置",
            (Self::CheckAllRegisteredRepositoriesConcurrencyLimitAutomatic, Language::English) => {
                "Check all {} registered repositories\nConcurrency limit: {}\nAutomatic push policy: {}\n\nWorktrees sharing Git storage are checked serially. Only committed changes that are ahead and not behind may be pushed automatically."
            }
            (Self::CheckAllRegisteredRepositoriesConcurrencyLimitAutomatic, Language::Chinese) => {
                "检查全部 {} 个注册仓库\n并发上限：{}\n自动推送策略：{}\n\n共享 Git 存储的工作区串行检查；仅领先且不落后的已提交内容可自动推送。"
            }
            (Self::ApplySavedIntervalAndReloadTheSystem, Language::English) => {
                "Apply saved interval and reload the system schedule. No check or push starts immediately."
            }
            (Self::ApplySavedIntervalAndReloadTheSystem, Language::Chinese) => {
                "应用已保存的检查间隔，重新加载系统任务。不会立即检查或推送。"
            }
            (Self::UnregisterRepositoryFilesOnDiskAreRetained, Language::English) => {
                "Unregister repository: {}\n\nFiles on disk are retained."
            }
            (Self::UnregisterRepositoryFilesOnDiskAreRetained, Language::Chinese) => {
                "移除仓库注册项：{}\n\n保留磁盘文件。"
            }
            (Self::ConfirmOperation, Language::English) => "Confirm operation",
            (Self::ConfirmOperation, Language::Chinese) => "确认操作",
            (Self::CurrentRepositoryIsNotInTheFiltered, Language::English) => {
                "Current repository is not in the filtered list; Esc to return."
            }
            (Self::CurrentRepositoryIsNotInTheFiltered, Language::Chinese) => {
                "当前仓库不在筛选结果中；Esc 返回。"
            }
            (Self::Instructions, Language::English) => "Instructions",
            (Self::Instructions, Language::Chinese) => "操作说明",
            (Self::GwResizeTerminalToAtLeast60, Language::English) => {
                "gw: resize terminal to at least 60 columns × 18 rows.\nCtrl+C exits; Esc cancels an operation in progress."
            }
            (Self::GwResizeTerminalToAtLeast60, Language::Chinese) => {
                "gw：请将终端调整到至少 60 列 × 18 行。\nCtrl+C 退出；进行中的操作可按 Esc 取消。"
            }
            (Self::CurrentRepo, Language::English) => "Current repo",
            (Self::CurrentRepo, Language::Chinese) => "当前仓库",
            (Self::Global, Language::English) => "Global",
            (Self::Global, Language::Chinese) => "全局",
            (Self::CurrentView, Language::English) => "Current view",
            (Self::CurrentView, Language::Chinese) => "当前视图",
            (Self::Commands, Language::English) => "Commands · {}",
            (Self::Commands, Language::Chinese) => "命令 · {}",
            (Self::NoRepositorySelected, Language::English) => "No repository selected",
            (Self::NoRepositorySelected, Language::Chinese) => "未选择仓库",
            (Self::NoMatchingCommands, Language::English) => "No matching commands",
            (Self::NoMatchingCommands, Language::Chinese) => "没有匹配命令",
            (Self::TuiHelp, Language::English) => {
                "Persistent input\nType to search repositories, or files in the file panel.\nType / to show command candidates; input is always active.\nCandidates span the full width above input; list and input positions stay fixed.\nWhen searching: ↑↓ selects, Enter opens repository details or file diff.\nIn commands: ↑↓ selects a candidate, Tab completes, Enter runs a complete command.\nEsc clears input; with empty input, Esc returns to the previous page.\nCtrl+U clears text before the cursor; Ctrl+A/E moves to the start/end.\nIn the file panel, Ctrl+A selects all visible files.\n\nNavigation\n↑↓ selects; PgUp/PgDn pages; Tab switches repositories and settings.\nCtrl+F cycles All, Problems, No upstream and Pushes.\nAlt+O switches priority/path order; F1 opens help.\nCtrl+R refreshes the view; Ctrl+C exits.\nLetters, numbers and spaces are input, not single-letter shortcuts.\n\nCommands\n/stage opens worktree files; /unstage opens the index.\nFiles: Space marks with empty input; Insert always marks; Ctrl+A marks visible files.\nTab switches areas; Enter shows diff. Filtering retains hidden selections.\nIn files, /stage and /unstage act on marked files, or the cursor file if none are marked.\nUnstaging retains worktree contents; Git-ignored files are not listed.\n/commit reviews staged contents, takes a message and confirms; hooks and signing are preserved.\n/push fetches and validates upstream; confirmation pushes only the reviewed branch snapshot.\n/check all checks all registered repositories after confirmation, with configured automatic push.\n/refresh reloads report or files; /settings opens settings; /help shows help; /quit exits.\n/add, /remove, /exclude, /unexclude manage registration.\n/scan previews discovery; /import scans and imports using Git ignore rules.\n/interval, /autopush, /concurrency, /apply manage configuration and system schedule.\nCommand scope is fixed when / is entered. The bar does not execute a shell.\n\nConfirmation\nConfiguration, checks, commits and pushes require y; n/Esc cancels.\nCommit messages and configuration use the bottom input; confirmation/running states lock input.\nEsc/Ctrl+C cancels operations and waits for workers and Git processes to stop.\nOpening the TUI does not check, scan or push. Exiting restores the terminal.\n\nStatus legend\n✓ clean, ! attention, × check failed, · unchecked.\n*Changes includes uncommitted/untracked files and submodule changes.\nThe repository list uses the last report; the file panel reads current state explicitly.\nNo-upstream branches are counted separately, are not problems and are never published automatically.\n↑n !m counts successful/failed branch pushes in the last check; ? means unknown.\nFull errors appear in preview/details. Manual operations do not overwrite historical reports.\n\nLanguage\nSystem Chinese uses Chinese; all other languages use English.\nSet GW_LANG=en or GW_LANG=zh before starting gw to override.\n"
            }
            (Self::TuiHelp, Language::Chinese) => {
                "常驻输入框\n直接打字搜索仓库；文件面板中搜索文件。\n输入 / 显示命令候选，不需要先激活输入框。\n命令候选在输入框上方以全宽面板显示，仓库列表与输入框位置保持稳定。\n搜索时 ↑↓ 操作列表，Enter 查看选中仓库或文件差异。\n命令时 ↑↓ 选择候选，Tab 补全，Enter 执行完整命令。\nEsc 清空输入；输入为空时 Esc 返回上一页。\nCtrl+U 清空光标前的文字，Ctrl+A/E 跳到开头或末尾。\n文件面板中 Ctrl+A 用于全选当前可见文件。\n\n仓库导航\n↑↓ 选择，PgUp/PgDn 翻页；Tab 切换仓库和设置。\nCtrl+F 循环切换全部、问题、无上游、推送。\nAlt+O 切换问题优先和路径顺序；F1 打开帮助。\nCtrl+R 刷新当前视图；Ctrl+C 退出。\n普通字母、数字和空格用于输入，不触发单字母快捷操作。\n\n命令\n/stage 打开工作区，/unstage 打开暂存区。\n文件面板：空输入时 Space 勾选，Insert 始终勾选，Ctrl+A 全选可见项。\nTab 切换区域，Enter 查看 diff；筛选隐藏文件时保留已勾选项。\n在文件面板执行 /stage 或 /unstage 操作勾选项；未勾选时操作光标项。\n取消暂存保留工作区内容；Git 忽略的文件不列出。\n/commit 预览暂存内容，输入说明并确认；保留 Git 钩子和签名。\n/push 获取当前远程并核验上游，确认后仅推送当前分支快照。\n/check all 确认后检查全部注册仓库，按当前策略自动推送。\n/refresh 刷新报告或文件列表，/settings 设置，/help 帮助，/quit 退出。\n/add、/remove、/exclude、/unexclude 管理注册列表。\n/scan 扫描预览，/import 扫描并导入，遵守 Git 忽略规则。\n/interval、/autopush、/concurrency、/apply 管理配置与系统调度。\n命令范围在输入前缀 / 时固定，命令栏不执行 Shell。\n\n操作确认\n配置、检查、提交和推送经 y 确认，n/Esc 取消。\n提交说明和配置输入会使用底部输入框，确认或执行期间禁止普通输入。\n耗时操作 Esc/Ctrl+C 取消，等待线程与 Git 子进程收尾。\n进入界面不会检查、扫描或推送；退出恢复终端。\n\n状态图例\n✓ 正常，! 需关注，× 检查失败，· 未检查。\n工作区 *改动 包括未提交、未跟踪和子模块改动。\n列表来自上次报告；文件面板为显式读取的当前状态。\n无上游分支单独统计，不作为问题，也不会自动发布。\n↑n !m 是上次检查成功、失败的分支数，? 表示该项未知。\n完整失败原因在预览和详情中查看，手动操作不改写历史检查报告。\n"
            }
            (Self::Repos, Language::English) => " {} repos",
            (Self::Repos, Language::Chinese) => " {}库",
            (Self::Search, Language::English) => " Search: {}",
            (Self::Search, Language::Chinese) => " 搜索：{}",
            (Self::WorktreeChangesUnknownPushSuccessFailureCounts, Language::English) => {
                "* Worktree changes · ? Unknown · ↑/! Push success/failure counts"
            }
            (Self::WorktreeChangesUnknownPushSuccessFailureCounts, Language::Chinese) => {
                "* 工作区改动 · ? 未知 · ↑/! 推送成功/失败分支数"
            }
            (Self::NoRepositorySelectedAlt, Language::English) => "No repository selected",
            (Self::NoRepositorySelectedAlt, Language::Chinese) => "无选中仓库",
            (Self::NoRepositories, Language::English) => "No repositories",
            (Self::NoRepositories, Language::Chinese) => "当前没有仓库",
            (Self::CleanAlt, Language::English) => "Clean",
            (Self::CleanAlt, Language::Chinese) => "干净",
            (Self::Passed, Language::English) => "Passed",
            (Self::Passed, Language::Chinese) => "通过",
            (Self::Changes, Language::English) => "*Changes",
            (Self::Changes, Language::Chinese) => "*改动",
            (Self::OutOfSync, Language::English) => "Out of sync",
            (Self::OutOfSync, Language::Chinese) => "待同步",
            (Self::MissingUpstream, Language::English) => "No upstream",
            (Self::MissingUpstream, Language::Chinese) => "上游缺失",
            (Self::NoCommits, Language::English) => "No commits",
            (Self::NoCommits, Language::Chinese) => "无提交",
            (Self::TlsFailed, Language::English) => "TLS failed",
            (Self::TlsFailed, Language::Chinese) => "TLS失败",
            (Self::RemoteChangedAfterFetchOrFetchRefspecAlt, Language::English) => {
                "remote changed after fetch, or fetch refspec did not update upstream"
            }
            (Self::RemoteChangedAfterFetchOrFetchRefspecAlt, Language::Chinese) => {
                "远程在 fetch 后发生变化或 fetch refspec 未更新上游"
            }
            (Self::RefMismatch, Language::English) => "Ref mismatch",
            (Self::RefMismatch, Language::Chinese) => "引用不符",
            (Self::Remote, Language::English) => "Remote",
            (Self::Remote, Language::Chinese) => "远程",
            (Self::ListProblemsFirst, Language::English) => "List · problems first",
            (Self::ListProblemsFirst, Language::Chinese) => "列表 · 问题优先",
            (Self::ListPathOrder, Language::English) => "List · path order",
            (Self::ListPathOrder, Language::Chinese) => "列表 · 路径顺序",
            (Self::NoRepositoriesMatchAddToAddOne, Language::English) => {
                "No repositories match; /add to add one"
            }
            (Self::NoRepositoriesMatchAddToAddOne, Language::Chinese) => {
                "当前筛选没有仓库；/add 添加仓库"
            }
            (Self::Path, Language::English) => "Path",
            (Self::Path, Language::Chinese) => "路径",
            (Self::Status, Language::English) => "Status",
            (Self::Status, Language::Chinese) => "状态",
            (Self::Unchecked, Language::English) => "Unchecked",
            (Self::Unchecked, Language::Chinese) => "未检查",
            (Self::NotIncludedInTheLastReportWait, Language::English) => {
                "Not included in the last report; wait for a scheduled or explicit check."
            }
            (Self::NotIncludedInTheLastReportWait, Language::Chinese) => {
                "上次报告未包含此仓库，等待定时或显式检查。"
            }
            (Self::RemoteTlsConnectionFailedSslErrorSyscall, Language::English) => {
                "Remote TLS connection failed (SSL_ERROR_SYSCALL)"
            }
            (Self::RemoteTlsConnectionFailedSslErrorSyscall, Language::Chinese) => {
                "远程 TLS 连接失败（SSL_ERROR_SYSCALL）"
            }
            (Self::UpstreamReferenceMismatchRemoteChangedOrFetch, Language::English) => {
                "Upstream reference mismatch: remote changed or fetch configuration did not update upstream"
            }
            (Self::UpstreamReferenceMismatchRemoteChangedOrFetch, Language::Chinese) => {
                "上游引用不一致：远程更新或 fetch 配置未更新该上游"
            }
            (Self::CheckFailedSeeOriginalError, Language::English) => {
                "Check failed; see original error"
            }
            (Self::CheckFailedSeeOriginalError, Language::Chinese) => "检查失败，详情见原始错误",
            (Self::Reason, Language::English) => "Reason",
            (Self::Reason, Language::Chinese) => "失败原因",
            (Self::Unknown, Language::English) => "Unknown",
            (Self::Unknown, Language::Chinese) => "未知",
            (Self::RemoteCheck, Language::English) => "Remote check",
            (Self::RemoteCheck, Language::Chinese) => "远程检查",
            (Self::Incomplete, Language::English) => "Incomplete",
            (Self::Incomplete, Language::Chinese) => "未完成",
            (Self::BranchCounts, Language::English) => "Branch counts",
            (Self::BranchCounts, Language::Chinese) => "分支统计",
            (Self::OriginalError, Language::English) => "Original error",
            (Self::OriginalError, Language::Chinese) => "原始错误",
            (Self::HeadState, Language::English) => "HEAD state",
            (Self::HeadState, Language::Chinese) => "HEAD 状态",
            (Self::CommitState, Language::English) => "Commit state",
            (Self::CommitState, Language::Chinese) => "提交状态",
            (Self::RemoteUpstream, Language::English) => "Remote upstream",
            (Self::RemoteUpstream, Language::Chinese) => "远程上游",
            (Self::RemoteSync, Language::English) => "Remote sync",
            (Self::RemoteSync, Language::Chinese) => "远程同步",
            (Self::Branches, Language::English) => "{} branches",
            (Self::Branches, Language::Chinese) => "{} 个分支",
            (Self::TracksALocalBranch, Language::English) => " (tracks a local branch)",
            (Self::TracksALocalBranch, Language::Chinese) => "（跟踪本地分支）",
            (Self::PushHistory, Language::English) => "Push history",
            (Self::PushHistory, Language::Chinese) => "推送记录",
            (Self::Branch, Language::English) => "Branch",
            (Self::Branch, Language::Chinese) => "分支",
            (Self::PushTarget, Language::English) => "Push target",
            (Self::PushTarget, Language::Chinese) => "推送目标",
            (Self::PushResult, Language::English) => "Push result",
            (Self::PushResult, Language::Chinese) => "推送结果",
            (Self::PushedCommitsAlt, Language::English) => "Pushed ({} commits)",
            (Self::PushedCommitsAlt, Language::Chinese) => "已推送（{} 个提交）",
            (Self::Validation, Language::English) => "Validation",
            (Self::Validation, Language::Chinese) => "校验",
            (Self::Transfer, Language::English) => "Transfer",
            (Self::Transfer, Language::Chinese) => "传输",
            (Self::Verification, Language::English) => "Verification",
            (Self::Verification, Language::Chinese) => "核验",
            (Self::PhasePhaseFailed, Language::English) => "{phase} phase failed",
            (Self::PhasePhaseFailed, Language::Chinese) => "{phase}阶段失败",
            (Self::PushError, Language::English) => "Push error",
            (Self::PushError, Language::Chinese) => "推送错误",
            (Self::TabCompleteCandidatesEnterRunEscClear, Language::English) => {
                "Tab complete · ↑↓ candidates · Enter run · Esc clear"
            }
            (Self::TabCompleteCandidatesEnterRunEscClear, Language::Chinese) => {
                "Tab 补全 · ↑↓候选 · Enter 执行完整命令 · Esc 清空"
            }
            (Self::SelectSpaceInsMarkCtrlAAll, Language::English) => {
                "↑↓ select · Space/Ins mark · Ctrl+A all · Tab area"
            }
            (Self::SelectSpaceInsMarkCtrlAAll, Language::Chinese) => {
                "↑↓选择 · Space/Ins勾选 · Ctrl+A全选 · Tab切换区域"
            }
            (Self::ScrollEscFilesCommands, Language::English) => {
                "↑↓ scroll · Esc files · /commands"
            }
            (Self::ScrollEscFilesCommands, Language::Chinese) => "↑↓滚动 · Esc返回文件 · /命令",
            (Self::SelectEnterDetailsCtrlFFilterTab, Language::English) => {
                "↑↓ select · Enter open · Ctrl+F filter · Tab/F1 settings/help"
            }
            (Self::SelectEnterDetailsCtrlFFilterTab, Language::Chinese) => {
                "↑↓选择 · Enter详情 · Ctrl+F筛选 · Tab设置 · F1帮助"
            }
            (Self::SelectEnterEditTabReposCommands, Language::English) => {
                "↑↓ select · Enter edit · Tab repos · /commands"
            }
            (Self::SelectEnterEditTabReposCommands, Language::Chinese) => {
                "↑↓选择 · Enter修改 · Tab仓库 · /命令"
            }
            (Self::PgdnScrollEscBackCommands, Language::English) => {
                "↑↓/PgDn scroll · Esc back · /commands"
            }
            (Self::PgdnScrollEscBackCommands, Language::Chinese) => "↑↓/PgDn滚动 · Esc返回 · /命令",
            (Self::CommandsAlt, Language::English) => " Commands ",
            (Self::CommandsAlt, Language::Chinese) => " 命令 ",
            (Self::SearchFiles, Language::English) => " Search files ",
            (Self::SearchFiles, Language::Chinese) => " 搜索文件 ",
            (Self::SearchRepositories, Language::English) => " Search repositories ",
            (Self::SearchRepositories, Language::Chinese) => " 搜索仓库 ",
            (Self::TypeToSearchOrForCommands, Language::English) => {
                "Type to search, or / for commands"
            }
            (Self::TypeToSearchOrForCommands, Language::Chinese) => {
                "输入关键词搜索，或输入 / 查看命令"
            }
            (Self::ConfigurationInputAlt, Language::English) => " Configuration input ",
            (Self::ConfigurationInputAlt, Language::Chinese) => " 配置输入 ",
            (Self::EnterAConfigurationValue, Language::English) => "Enter a configuration value",
            (Self::EnterAConfigurationValue, Language::Chinese) => "请输入配置值",
            (Self::EnterNextEscCancel, Language::English) => "Enter next · Esc cancel",
            (Self::EnterNextEscCancel, Language::Chinese) => "Enter 下一步 · Esc 取消",
            (Self::CommitMessageAlt, Language::English) => " Commit message ",
            (Self::CommitMessageAlt, Language::Chinese) => " 提交说明 ",
            (Self::DescribeThisCommit, Language::English) => "Describe this commit",
            (Self::DescribeThisCommit, Language::Chinese) => "请输入本次提交的说明",
            (Self::EnterConfirmEscCancelPgupPgdnReview, Language::English) => {
                "Enter confirm · Esc cancel · PgUp/PgDn review"
            }
            (Self::EnterConfirmEscCancelPgupPgdnReview, Language::Chinese) => {
                "Enter 确认 · Esc 取消 · PgUp/PgDn 查看"
            }
            (Self::ConfirmOperationAlt, Language::English) => " Confirm operation ",
            (Self::ConfirmOperationAlt, Language::Chinese) => " 确认操作 ",
            (Self::AwaitingConfirmation, Language::English) => "Awaiting confirmation",
            (Self::AwaitingConfirmation, Language::Chinese) => "等待确认",
            (Self::YConfirmNEscCancel, Language::English) => "y confirm · n/Esc cancel",
            (Self::YConfirmNEscCancel, Language::Chinese) => "y 确认执行 · n/Esc 取消",
            (Self::ConfirmCommitAlt, Language::English) => " Confirm commit ",
            (Self::ConfirmCommitAlt, Language::Chinese) => " 确认提交 ",
            (Self::AwaitingCommitConfirmation, Language::English) => "Awaiting commit confirmation",
            (Self::AwaitingCommitConfirmation, Language::Chinese) => "等待确认提交",
            (Self::YCommitNEscCancelPgupPgdn, Language::English) => {
                "y commit · n/Esc cancel · PgUp/PgDn review"
            }
            (Self::YCommitNEscCancelPgupPgdn, Language::Chinese) => {
                "y 确认提交 · n/Esc 取消 · PgUp/PgDn 查看"
            }
            (Self::ConfirmPushAlt, Language::English) => " Confirm push ",
            (Self::ConfirmPushAlt, Language::Chinese) => " 确认推送 ",
            (Self::AwaitingPushConfirmation, Language::English) => "Awaiting push confirmation",
            (Self::AwaitingPushConfirmation, Language::Chinese) => "等待确认推送",
            (Self::YPushNEscCancelPgupPgdn, Language::English) => {
                "y push · n/Esc cancel · PgUp/PgDn review"
            }
            (Self::YPushNEscCancelPgupPgdn, Language::Chinese) => {
                "y 确认推送 · n/Esc 取消 · PgUp/PgDn 查看"
            }
            (Self::OperationIncomplete, Language::English) => " Operation incomplete ",
            (Self::OperationIncomplete, Language::Chinese) => " 操作未完成 ",
            (Self::SeeErrorDetails, Language::English) => "See error details",
            (Self::SeeErrorDetails, Language::Chinese) => "请查看错误详情",
            (Self::PgdnFullErrorEnterEscBack, Language::English) => {
                "↑↓/PgDn full error · Enter/Esc back"
            }
            (Self::PgdnFullErrorEnterEscBack, Language::Chinese) => {
                "↑↓/PgDn 查看完整错误 · Enter/Esc 返回"
            }
            (Self::RunningAlt, Language::English) => " Running ",
            (Self::RunningAlt, Language::Chinese) => " 执行中 ",
            (Self::Cancelling, Language::English) => "Cancelling…",
            (Self::Cancelling, Language::Chinese) => "正在取消…",
            (Self::EscCtrlCCancelAndWaitFor, Language::English) => {
                "Esc/Ctrl+C cancel and wait for child processes"
            }
            (Self::EscCtrlCCancelAndWaitFor, Language::Chinese) => {
                "Esc/Ctrl+C 取消并等待子进程退出"
            }
            (Self::ChecksAndPushes, Language::English) => "Checks and pushes",
            (Self::ChecksAndPushes, Language::Chinese) => "检查与推送",
            (Self::EveryHours, Language::English) => "Every {} hours",
            (Self::EveryHours, Language::Chinese) => "每 {} 小时",
            (Self::SystemSchedule, Language::English) => "System schedule: {} · {}",
            (Self::SystemSchedule, Language::Chinese) => "系统任务：{} · {}",
            (Self::CheckingAltAlt, Language::English) => "Checking",
            (Self::CheckingAltAlt, Language::Chinese) => "检查运行中",
            (Self::AppliedIntervalHoursConfigurationNotYetApplied, Language::English) => {
                "Applied interval: {} hours; configuration not yet applied"
            }
            (Self::AppliedIntervalHoursConfigurationNotYetApplied, Language::Chinese) => {
                "已应用间隔 {} 小时，配置尚未应用"
            }
            (Self::ExcludedDirectories, Language::English) => "Excluded directories",
            (Self::ExcludedDirectories, Language::Chinese) => "排除目录",
            (Self::AutoPushFailureTitle, Language::English) => "GW automatic push failed",
            (Self::AutoPushFailureTitle, Language::Chinese) => "GW 自动推送失败",
        }
    }
}
impl std::fmt::Display for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.text())
    }
}
#[macro_export]
macro_rules! tf {
    (FieldLabel $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("{label}: " $(, $($args)*)?),
            $crate::Language::Chinese => format!("{label}：" $(, $($args)*)?),
        }
    };
    (UnableToRegisterExitSignalE $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Unable to register exit signal: {e}" $(, $($args)*)?),
            $crate::Language::Chinese => format!("无法注册退出信号：{e}" $(, $($args)*)?),
        }
    };
    (ErrorE $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Error: {e:#}" $(, $($args)*)?),
            $crate::Language::Chinese => format!("错误：{e:#}" $(, $($args)*)?),
        }
    };
    (AddedNRepositories $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Added {n} repositories" $(, $($args)*)?),
            $crate::Language::Chinese => format!("已添加 {n} 个仓库" $(, $($args)*)?),
        }
    };
    (FoundRepositoriesFailedCouldNotBeRead $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Found {} repositories; {failed} could not be read. See var/import-report.json" $(, $($args)*)?),
            $crate::Language::Chinese => format!("发现 {} 个仓库，其中 {failed} 个无法读取；详见 var/import-report.json" $(, $($args)*)?),
        }
    };
    (Checking $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Checking: {}" $(, $($args)*)?),
            $crate::Language::Chinese => format!("正在检查：{}" $(, $($args)*)?),
        }
    };
    (UnableToPrintResultE $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Unable to print result: {e}" $(, $($args)*)?),
            $crate::Language::Chinese => format!("无法输出结果：{e}" $(, $($args)*)?),
        }
    };
    (SystemNotificationFailedN $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("System notification failed: {n:?}" $(, $($args)*)?),
            $crate::Language::Chinese => format!("系统通知发送失败：{n:?}" $(, $($args)*)?),
        }
    };
    (CheckCompleteRepositories $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Check complete: {:?}, {} repositories" $(, $($args)*)?),
            $crate::Language::Chinese => format!("检查完成：{:?}，共 {} 个仓库" $(, $($args)*)?),
        }
    };
    (InstalledNativeGwAndSystemScheduleChecking $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Installed native gw and system schedule; checking every {} hours. No check started" $(, $($args)*)?),
            $crate::Language::Chinese => format!("已安装原生 gw 命令与系统任务，每 {} 小时检查一次；未触发检查" $(, $($args)*)?),
        }
    };
    (UnknownCommandCommandRunGwHelp $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Unknown command {command:?}; run gw --help" $(, $($args)*)?),
            $crate::Language::Chinese => format!("未知命令 {command:?}；运行 gw --help 查看帮助" $(, $($args)*)?),
        }
    };
    (UnknownReportArgumentValue $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Unknown report argument: {value}" $(, $($args)*)?),
            $crate::Language::Chinese => format!("未知报告参数：{value}" $(, $($args)*)?),
        }
    };
    (InvalidReportTimestampValue $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Invalid report timestamp: {value}" $(, $($args)*)?),
            $crate::Language::Chinese => format!("报告时间格式无效：{value}" $(, $($args)*)?),
        }
    };
    (SystemScheduleCheckProcessConfiguredIntervalEvery $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("System schedule: {}\nCheck process: {}\nConfigured interval: every {} hours\nRepositories: {}\nConcurrency: {}\nAutomatic push: {}\n" $(, $($args)*)?),
            $crate::Language::Chinese => format!("系统任务：{}\n检查进程：{}\n配置频率：每 {} 小时\n仓库数量：{}\n检查并发：{}\n自动推送：{}\n" $(, $($args)*)?),
        }
    };
    (ScheduledIntervalEveryHours $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Scheduled interval: every {} hours\n" $(, $($args)*)?),
            $crate::Language::Chinese => format!("系统任务频率：每 {} 小时\n" $(, $($args)*)?),
        }
    };
    (ExcludedDirectory $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Excluded directory: {}\n" $(, $($args)*)?),
            $crate::Language::Chinese => format!("排除目录：{}\n" $(, $($args)*)?),
        }
    };
    (LastCheckLatestResultCleanNeedAttention $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Last check: {}\nLatest result: {} clean, {} need attention, {} failed\nLatest automatic pushes: {} succeeded, {} failed\nNo remote upstream (counted separately): {} branches across {} repositories\n" $(, $($args)*)?),
            $crate::Language::Chinese => format!("最近检查：{}\n最近结果：{} 个正常，{} 个需关注，{} 个检查失败\n最近自动推送：{} 个分支成功，{} 个分支失败\n未配置远程上游（单独统计）：{} 个分支，涉及 {} 个仓库\n" $(, $($args)*)?),
        }
    };
    (OtherRepositoriesFailedToCheckTheseCounts $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("{} other repositories failed to check; these counts are incomplete\n" $(, $($args)*)?),
            $crate::Language::Chinese => format!("另有 {} 个仓库检查失败，此项统计不完整\n" $(, $($args)*)?),
        }
    };
    (SystemNotificationsFailedSeeGwReportPushes $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("{} system notifications failed; see gw report --pushes\n" $(, $($args)*)?),
            $crate::Language::Chinese => format!("系统通知发送失败：{} 条，详情见 gw report --pushes\n" $(, $($args)*)?),
        }
    };
    (CheckedAt $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Checked at: {}\n" $(, $($args)*)?),
            $crate::Language::Chinese => format!("检查数据时间：{}\n" $(, $($args)*)?),
        }
    };
    (ClassificationUpdatedAt $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Classification updated at: {}\n" $(, $($args)*)?),
            $crate::Language::Chinese => format!("分类规则更新时间：{}\n" $(, $($args)*)?),
        }
    };
    (SystemNotificationFailedError $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("  - System notification failed: {error}\n" $(, $($args)*)?),
            $crate::Language::Chinese => format!("  - 系统通知发送失败：{error}\n" $(, $($args)*)?),
        }
    };
    (PushedCommits $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("  - Pushed {} → {}/{}, {} commits\n" $(, $($args)*)?),
            $crate::Language::Chinese => format!("  - 已推送 {} → {}/{}，{} 个提交\n" $(, $($args)*)?),
        }
    };
    (PushFailedPhaseError $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("  - Push failed {} ({phase:?}): {error}\n" $(, $($args)*)?),
            $crate::Language::Chinese => format!("  - 推送失败 {}（{phase:?}）：{error}\n" $(, $($args)*)?),
        }
    };
    (CountedSeparately $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("  - {}: {} (counted separately)\n" $(, $($args)*)?),
            $crate::Language::Chinese => format!("  - {}：{}（单独统计）\n" $(, $($args)*)?),
        }
    };
    (DetailsGwReportPushes $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("{}\nDetails: gw report --pushes" $(, $($args)*)?),
            $crate::Language::Chinese => format!("{}\n详情：gw report --pushes" $(, $($args)*)?),
        }
    };
    (RemoteUpstreamBranchDoesNotExist $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("{}: remote upstream branch does not exist" $(, $($args)*)?),
            $crate::Language::Chinese => format!("{}：远程上游分支不存在" $(, $($args)*)?),
        }
    };
    (RemoteChangedAfterFetchOrFetchRefspec $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("{}: remote changed after fetch, or fetch refspec did not update upstream" $(, $($args)*)?),
            $crate::Language::Chinese => format!("{}：远程在 fetch 后发生变化或 fetch refspec 未更新上游" $(, $($args)*)?),
        }
    };
    (LocalBranchChangedAfterPushCheckAgain $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("{}: local branch changed after push; check again next time" $(, $($args)*)?),
            $crate::Language::Chinese => format!("{}：推送后本地分支发生变化，待下次重新检查" $(, $($args)*)?),
        }
    };
    (AutomaticPushFailed $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("{}: automatic push failed: {}" $(, $($args)*)?),
            $crate::Language::Chinese => format!("{}：自动推送失败：{}" $(, $($args)*)?),
        }
    };
    (AheadOfRemoteByCommitsBehindBy $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("{}: ahead of remote by {} commits, behind by {} commits" $(, $($args)*)?),
            $crate::Language::Chinese => format!("{}：领先远程 {} 个提交，落后远程 {} 个提交" $(, $($args)*)?),
        }
    };
    (UnknownAutomaticPushPolicyS $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Unknown automatic push policy: {s}" $(, $($args)*)?),
            $crate::Language::Chinese => format!("未知自动推送策略：{s}" $(, $($args)*)?),
        }
    };
    (UnsupportedReportVersion $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Unsupported report version: {}" $(, $($args)*)?),
            $crate::Language::Chinese => format!("不支持的报告版本：{}" $(, $($args)*)?),
        }
    };
    (PathMustBeANormalizedAbsolutePath $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Path must be a normalized absolute path: {}" $(, $($args)*)?),
            $crate::Language::Chinese => format!("路径必须为规范绝对路径：{}" $(, $($args)*)?),
        }
    };
    (DuplicatePath $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Duplicate path: {}" $(, $($args)*)?),
            $crate::Language::Chinese => format!("重复路径：{}" $(, $($args)*)?),
        }
    };
    (RepositoryIsUnderAnExcludedDirectory $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Repository is under an excluded directory: {}" $(, $($args)*)?),
            $crate::Language::Chinese => format!("仓库处于已排除目录：{}" $(, $($args)*)?),
        }
    };
    (CannotRead $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Cannot read {}" $(, $($args)*)?),
            $crate::Language::Chinese => format!("无法读取 {}" $(, $($args)*)?),
        }
    };
    (LabelFailed $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("{label} failed ({}):\n{}\n{}" $(, $($args)*)?),
            $crate::Language::Chinese => format!("{label} 失败（{}）：\n{}\n{}" $(, $($args)*)?),
        }
    };
    (UnableToStart $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Unable to start {}" $(, $($args)*)?),
            $crate::Language::Chinese => format!("无法启动 {}" $(, $($args)*)?),
        }
    };
    (TimedOutSeconds $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("{} timed out ({} seconds)" $(, $($args)*)?),
            $crate::Language::Chinese => format!("{} 超时（{} 秒）" $(, $($args)*)?),
        }
    };
    (CurrentBranchIsAheadCommitsAheadAnd $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Current branch is {ahead} commits ahead and {behind} behind; handle remote updates manually first" $(, $($args)*)?),
            $crate::Language::Chinese => format!("当前分支领先 {ahead}、落后 {behind} 个提交；请先手动处理远程更新" $(, $($args)*)?),
        }
    };
    (FilesInsertionsDeletions $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("{} files, {} insertions, {} deletions" $(, $($args)*)?),
            $crate::Language::Chinese => format!("{} 个文件，新增 {} 行，删除 {} 行" $(, $($args)*)?),
        }
    };
    (ConfiguredPathIsNotTheRepositoryRoot $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Configured path is not the repository root: {}" $(, $($args)*)?),
            $crate::Language::Chinese => format!("配置路径不是仓库根目录：{}" $(, $($args)*)?),
        }
    };
    (RepositoryHasAnUnfinishedGitOperation $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Repository has an unfinished Git operation: {:?}" $(, $($args)*)?),
            $crate::Language::Chinese => format!("仓库存在未完成的 Git 操作：{:?}" $(, $($args)*)?),
        }
    };
    (RepositoryIsUnderAnExcludedDirectoryAlt $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Repository is under an excluded directory: {}" $(, $($args)*)?),
            $crate::Language::Chinese => format!("仓库位于排除目录中：{}" $(, $($args)*)?),
        }
    };
    (NotInTheRepositoryList $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Not in the repository list: {}" $(, $($args)*)?),
            $crate::Language::Chinese => format!("列表中不存在：{}" $(, $($args)*)?),
        }
    };
    (RepositoryIsNotCurrentlyRegistered $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Repository is not currently registered: {}" $(, $($args)*)?),
            $crate::Language::Chinese => format!("仓库不在当前注册列表中：{}" $(, $($args)*)?),
        }
    };
    (FilePathUsesExternalFilterNameSuch $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("File {path:?} uses external filter {name:?} (such as Git LFS), which is not registered in this libgit2 index backend; staging refused. Handle it explicitly with Git" $(, $($args)*)?),
            $crate::Language::Chinese => format!("文件 {path:?} 使用外部 filter {name:?}（例如 Git LFS），当前 libgit2 索引后端未注册该过滤器，已拒绝暂存；请使用 Git 显式处理" $(, $($args)*)?),
        }
    };
    (FilePathUsesWorkingTreeEncodingWhich $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("File {path:?} uses working-tree-encoding, which this backend does not support; staging refused" $(, $($args)*)?),
            $crate::Language::Chinese => format!("文件 {path:?} 使用 working-tree-encoding，当前后端不支持该转换，已拒绝暂存" $(, $($args)*)?),
        }
    };
    (NewSymlink $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("New symlink {:?} → {:?}" $(, $($args)*)?),
            $crate::Language::Chinese => format!("新符号链接 {:?} → {:?}" $(, $($args)*)?),
        }
    };
    (LargeUntrackedFileBytesViewWithAn $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Large untracked file ({} bytes); view with an external tool" $(, $($args)*)?),
            $crate::Language::Chinese => format!("未跟踪大文件（{} 字节），请使用外部工具查看" $(, $($args)*)?),
        }
    };
    (SystemNotificationFailedNotice $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("System notification failed: {notice:?}" $(, $($args)*)?),
            $crate::Language::Chinese => format!("系统通知发送失败：{notice:?}" $(, $($args)*)?),
        }
    };
    (OperationInterruptedSomeWritesMayHaveCompleted $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Operation interrupted; some writes may have completed. Refresh and verify actual state.\n{e:#}" $(, $($args)*)?),
            $crate::Language::Chinese => format!("操作被中断，可能已完成部分写入，请刷新并核对实际状态。\n{e:#}" $(, $($args)*)?),
        }
    };
    (CommittedTheListStillShowsTheLast $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Committed {}{}; the list still shows the last check" $(, $($args)*)?),
            $crate::Language::Chinese => format!("已提交 {}{}；列表仍为上次检查结果" $(, $($args)*)?),
        }
    };
    (PushedTheListStillShowsTheLast $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Pushed {} → {}/{}; the list still shows the last check" $(, $($args)*)?),
            $crate::Language::Chinese => format!("已推送 {} → {}/{}；列表仍为上次检查结果" $(, $($args)*)?),
        }
    };
    (PhasePhaseError $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("{phase:?} phase: {error}" $(, $($args)*)?),
            $crate::Language::Chinese => format!("{phase:?} 阶段：{error}" $(, $($args)*)?),
        }
    };
    (RefreshFailedE $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("; refresh failed: {e:#}" $(, $($args)*)?),
            $crate::Language::Chinese => format!("；刷新失败：{e:#}" $(, $($args)*)?),
        }
    };
    (CheckCompleteCleanAttentionFailed $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Check complete: {} clean · {} attention · {} failed" $(, $($args)*)?),
            $crate::Language::Chinese => format!("检查完成：{} 正常 · {} 需关注 · {} 失败" $(, $($args)*)?),
        }
    };
    (AddedRepositories $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Added {} repositories" $(, $($args)*)?),
            $crate::Language::Chinese => format!("已添加 {} 个仓库" $(, $($args)*)?),
        }
    };
    (FoundRepositoriesSeeVarImportReportJson $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Found {} repositories; see var/import-report.json" $(, $($args)*)?),
            $crate::Language::Chinese => format!("发现 {} 个仓库；详见 var/import-report.json" $(, $($args)*)?),
        }
    };
    (RepositoryBranchScopeStagedContentsOnlyStaged $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Repository: {}\nBranch: {}\nScope: staged contents only\n\nStaged files\n" $(, $($args)*)?),
            $crate::Language::Chinese => format!("仓库：{}\n分支：{}\n范围：仅已暂存内容\n\n暂存文件\n" $(, $($args)*)?),
        }
    };
    (CleanAttentionFailed $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!(" Clean {} · Attention {} · Failed {}" $(, $($args)*)?),
            $crate::Language::Chinese => format!(" 正常 {} · 需关注 {} · 失败 {}" $(, $($args)*)?),
        }
    };
    (LastCheck $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!(" Last check: {}{}" $(, $($args)*)?),
            $crate::Language::Chinese => format!(" 上次检查：{}{}" $(, $($args)*)?),
        }
    };
    (FilesSelected $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("{} · {} files · {} selected" $(, $($args)*)?),
            $crate::Language::Chinese => format!("{} · {} 个文件 · 已选 {}" $(, $($args)*)?),
        }
    };
    (CommitMessage $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Commit message: {}\n\n{}" $(, $($args)*)?),
            $crate::Language::Chinese => format!("提交说明：{}\n\n{}" $(, $($args)*)?),
        }
    };
    (RepositoryBranchRemoteUpstreamUrlPendingCommits $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Repository: {}\nBranch: {}\nRemote: {}\nUpstream: {}\nURL: {}\nPending: {} commits\n\nOnly push the reviewed commit snapshot; normal push with pre-push hooks preserved." $(, $($args)*)?),
            $crate::Language::Chinese => format!("仓库：{}\n分支：{}\n远程：{}\n上游：{}\n地址：{}\n待推送：{} 个提交\n\n仅发布预览时的提交快照；普通推送，保留 pre-push 钩子。" $(, $($args)*)?),
        }
    };
    (CheckAllRegisteredRepositoriesConcurrencyLimitAutomatic $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Check all {} registered repositories\nConcurrency limit: {}\nAutomatic push policy: {}\n\nWorktrees sharing Git storage are checked serially. Only committed changes that are ahead and not behind may be pushed automatically." $(, $($args)*)?),
            $crate::Language::Chinese => format!("检查全部 {} 个注册仓库\n并发上限：{}\n自动推送策略：{}\n\n共享 Git 存储的工作区串行检查；仅领先且不落后的已提交内容可自动推送。" $(, $($args)*)?),
        }
    };
    (UnregisterRepositoryFilesOnDiskAreRetained $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Unregister repository: {}\n\nFiles on disk are retained." $(, $($args)*)?),
            $crate::Language::Chinese => format!("移除仓库注册项：{}\n\n保留磁盘文件。" $(, $($args)*)?),
        }
    };
    (Commands $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Commands · {}" $(, $($args)*)?),
            $crate::Language::Chinese => format!("命令 · {}" $(, $($args)*)?),
        }
    };
    (Repos $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!(" {} repos" $(, $($args)*)?),
            $crate::Language::Chinese => format!(" {}库" $(, $($args)*)?),
        }
    };
    (Search $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!(" Search: {}" $(, $($args)*)?),
            $crate::Language::Chinese => format!(" 搜索：{}" $(, $($args)*)?),
        }
    };
    (Branches $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("{} branches" $(, $($args)*)?),
            $crate::Language::Chinese => format!("{} 个分支" $(, $($args)*)?),
        }
    };
    (PushedCommitsAlt $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Pushed ({} commits)" $(, $($args)*)?),
            $crate::Language::Chinese => format!("已推送（{} 个提交）" $(, $($args)*)?),
        }
    };
    (PhasePhaseFailed $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("{phase} phase failed" $(, $($args)*)?),
            $crate::Language::Chinese => format!("{phase}阶段失败" $(, $($args)*)?),
        }
    };
    (EveryHours $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Every {} hours" $(, $($args)*)?),
            $crate::Language::Chinese => format!("每 {} 小时" $(, $($args)*)?),
        }
    };
    (SystemSchedule $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("System schedule: {} · {}" $(, $($args)*)?),
            $crate::Language::Chinese => format!("系统任务：{} · {}" $(, $($args)*)?),
        }
    };
    (AppliedIntervalHoursConfigurationNotYetApplied $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Applied interval: {} hours; configuration not yet applied" $(, $($args)*)?),
            $crate::Language::Chinese => format!("已应用间隔 {} 小时，配置尚未应用" $(, $($args)*)?),
        }
    };
}
