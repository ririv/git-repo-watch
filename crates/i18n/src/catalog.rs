//! Complete bilingual catalog; each message has both languages.
use crate::{Language, language};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Message {
    CommandHistory,
    CommandHistoryEmpty,
    CommandHistoryEntryHint,
    CommandHistoryHint,
    InvalidCommandHistory,
    ReadCommandHistoryFailed,
    SaveCommandHistoryFailed,
    WorktreeHorizontalHint,
    ConflictControlsTitle,
    ConflictControlsHint,
    ConflictChoiceLeft,
    ConflictChoiceRight,
    ConflictChoiceBoth,
    ConflictChoiceUndo,
    ConflictChoiceReset,
    ConflictChoiceStage,
    ConflictReadingHint,
    ConflictResolvedHint,
    ConflictAcceptLeft,
    ConflictAcceptRight,
    ConflictAcceptBoth,
    ConflictResolveUnsupported,
    ConflictResolveChanged,
    ConflictMarkersInvalid,
    ConflictBlockRequired,
    ConflictAcceptCommand,
    ConflictBlockCommand,
    ConflictBlockPrevious,
    ConflictBlockNext,
    ConflictResetCommand,
    ConflictResetConfirm,
    ConflictResetUnsupported,
    ConflictUndoCommand,
    ConflictUndoUnavailable,
    ConflictResolvedPendingStage,
    ConflictBlockCount,

    CopyConflictUnavailable,
    CopyConflictVersions,
    CopyConflictResult,
    ConflictIncludeBase,
    ConflictCompareCurrent,
    ConflictCompareIncoming,
    ConflictCompareResult,
    ConflictOverview,
    ConflictCompareSides,
    ConflictCompareLeftResult,
    ConflictCompareResultRight,
    ConflictInputHint,
    ConflictBase,
    ConflictCurrent,
    ConflictIncoming,
    ConflictFileRequired,
    ConflictCompare,
    ConflictResult,
    ConflictViewCommand,
    ConflictStage,
    ConflictAbsent,
    ConflictEmpty,
    ConflictLineLimit,
    ConflictNonText,
    ConflictResultHint,
    BranchFilterCommand,
    BranchFilterRequiresView,
    HistoryParentCommand,
    HistoryParentRequiresMerge,
    CommandTargetReadingBranch,
    CommandTargetBranchFailed,
    RefreshUnavailable,
    RefreshHistorical,
    TerminalSelectionRequired,
    ExternalEditorCurrent,
    ExternalEditorOpening,
    ExternalEditorGit,
    ExternalEditorInvalid,
    ExternalEditorDetectionFailed,
    ExternalEditorMissing,
    ExternalEditorNoFolders,
    ExternalEditorNeedsTerminal,
    ExternalEditorFailed,
    ExternalEditorOpen,
    ExternalEditorSetting,
    ExternalEditorHint,
    ExternalEditorSelectionRequired,
    ExternalEditorOpened,
    ExternalEditorTools,

    LocalObservationFailed,
    CommandRequiresWorktreePreview,
    CommandRequiresFileTarget,
    PartialSelectRequired,
    PartialChanged,
    PartialSelectionConflict,
    PartialUnsupported,
    PartialSelectMode,
    PartialHunk,
    PartialLine,
    PartialDiscard,
    PartialDiscardConfirm,
    DiscardFile,
    DiscardFileSelect,
    DiscardFileUnsupported,
    DiscardFileRestoreReview,
    DiscardFileRemoveReview,
    DiscardFileInterrupted,
    PartialStage,
    PartialUnstage,
    PartialHunkHint,
    PartialLineHint,
    ManageRemove,
    ManageBack,
    ScanUsage,
    ScanExclusionOutsideRoot,
    ScanResults,
    ScanNew,
    ScanRegistered,
    ScanFailed,
    ScanAddSelected,
    ScanEmptySelection,
    ScanHint,
    ManageHint,
    ScanActionsHint,
    ScanActionsTitle,
    WorkflowFinishTitle,
    WorkflowFinishHint,
    ScanTemporaryExclusions,

    PushConfirmationNeeded,
    PushTargetNeeded,
    PushRemoteRequired,
    PushInvalidBranch,
    PushTargetTitle,
    PushTargetBranch,
    PushTargetHint,
    PushForceTitle,
    PushForceWarning,
    PushRemoteTip,
    PushNewTip,
    PushRecreateTitle,
    PushSetUpstream,

    ReplayPaused,
    ReplayBatchAtLeastTwo,
    ReplayControlsTitle,
    ReplayControlsHint,
    ReplayControlsEntryHint,
    ReplayContinueShort,
    ReplaySkipShort,
    ReplayAbortShort,
    ReplayConflictsShort,
    ReplayRemainingShort,

    ReplayBatchTitle,
    ReplayBatchTooLarge,
    ReplayDuplicateCommit,
    ReplayBatchMerge,
    ReplayOrder,
    ReplayQueueRemaining,
    ReplayConflictFiles,
    ReplayReadyContinue,
    HistoryMarked,
    HistorySelect,
    HistorySelectClear,
    HistorySelectDone,
    HistorySelectionHint,
    HistorySelectionRequired,
    HistorySelectionBack,
    HistoryActionCherryPick,
    HistoryActionRevert,
    HistoryActionCopy,
    HistoryActionsTitle,
    HistoryActionsCount,
    HistoryActionsHint,

    ReplayCherryPick,
    ReplayRevert,
    ReplayContinue,
    ReplaySkip,
    ReplayAbort,
    ReplayMetadataTooLarge,
    ReplayInvalidState,
    ReplayAttachedRequired,
    ReplayInvalidMessage,
    ReplayParentRequired,
    ReplayStats,
    ReplayNotActive,
    ReplayResolveFirst,
    ReplaySelectCommit,
    ReplayParentsTitle,
    ReplayParentsHint,
    ReplayPendingHint,
    ReplayAbortHint,
    ReplaySkipHint,
    ReplayMainline,
    ReplayMainlineHelp,
    ReplayConflictWorktree,
    ReplaySource,
    ReplayTargetBranch,
    ReplayActive,

    PressCtrlCAgainToQuit,
    ComposeTooLarge,
    ComposeFileTooLarge,
    ComposeTitle,
    ExperimentalFeatures,
    ExperimentalEnabled,
    ExperimentalHint,
    ExperimentalDisabled,
    ComposeCommand,
    ComposeInvalid,
    ComposeCoverage,
    ComposeDependency,
    ComposeNoChanges,
    ComposeSubmodule,
    ComposeChanged,
    ComposeEmptyStep,
    ComposeIndexLocked,
    ComposeCommitMissing,
    ComposeCommitStopped,
    ComposeHookConflict,
    ComposeHookChanged,
    ComposeGenerating,
    ComposeNext,
    ComposeDone,
    ComposeHint,
    ComposeEditHint,
    CommandEnterParameters,
    CommandEnterArgument,
    CommandEnterRequired,
    CommandEnterReview,
    CommandEnterOpen,
    CommandEnterRun,
    CommandEnterApply,
    CommandEnterFillCommand,
    CommandEnterFillPath,
    CommandEnterFillValue,
    CommandEnterWaiting,
    CommandCandidateKeys,
    CommandScopedCandidateKeys,
    CommandCandidateHistoryKeys,
    OptionCompletionHint,
    UnknownOptionValue,
    NoMatchingOptions,
    ReadingCommandCandidates,
    ValueCompletionHint,
    CommandRequiresHistory,
    CommandRequiresBrowser,
    CommandNeedsUnstaged,
    CommandNeedsStaged,
    CommandReplayStateUnknown,
    CommandReplayInProgress,
    CommandSourceCurrent,
    CommandPreviewCurrent,
    CommandRequiresPreviewableFile,
    CommandTargetFiles,
    CommandTargetLines,
    CommandTargetCommits,
    ComposeWaitingHint,
    ComposeRetryHint,
    ComposeGenerationFailed,
    ComposeEdit,
    ComposeUp,
    ComposeDown,
    ComposeSplit,
    ComposeMerge,
    ComposeMove,
    ComposeMoveHint,
    ComposeRegenerate,
    ComposeSelected,
    ComposePending,
    ComposeUnits,
    ComposeCompleted,

    AgentStreamIncomplete,
    AgentStreamUnexpectedRequest,
    CommitGeneratingHint,
    CommitEditingHint,
    CommitActionsHint,
    CommitRegenerateAction,
    CommitSubmitting,
    CommitSubmittingHint,
    CommitCancelAction,
    CommitReturnAction,
    SearchInputTitle,
    SearchModeCommand,
    SearchFilterCommand,
    SearchLocateCommand,
    SearchFilter,
    SearchLocate,
    SearchLocateHint,
    SearchCommandOnly,
    SearchUnavailable,
    SearchTargetRepositories,
    SearchTargetFiles,
    SearchTargetContent,
    ContentSearchPlaceholder,
    SearchTargetBranches,
    SearchTargetStashes,
    SearchTargetCommits,
    PaneReadHint,
    BrowserPreviewHint,
    DiffPreviewInputHint,
    PaneFocusCommand,
    PaneFocusTitle,
    PaneFocusHint,
    PaneZoomCommand,
    PaneUnavailable,
    TasksCheck,
    TasksScheduledCheck,
    TaskInterrupted,
    Tasks,
    TasksHint,
    TasksEmpty,
    TasksPending,
    TasksRunning,
    TasksCancelling,
    TasksSuccess,
    TasksFailedHandled,
    TasksCancelled,
    TasksHistory,
    TasksUnavailable,
    TasksSummary,
    TasksOpenCommand,
    TasksEntrySuffix,
    TasksDetailsHint,
    TasksInteractive,
    TasksScheduled,
    TasksImported,
    TasksStarted,
    TasksFinished,
    TasksDetails,
    TasksFailureReason,
    TasksProgress,
    TasksSource,
    TasksScope,
    TasksAll,
    TasksApplication,
    TasksStorageError,

    ExpandOperationResult,
    OperationResultReaderHint,
    Ai,
    PreferenceSaved,
    AiHelp,
    AiSettings,
    AiEnabled,
    AiEnabledHint,
    AiDisabled,
    AgentNotFound,
    AgentSettings,
    AgentChoice,
    AgentNameHint,
    UnknownAgent,
    SupportedAgents,
    ArgumentCandidates,
    AgentChoiceHint,
    AgentModel,
    AgentModelHint,
    AgentPrompt,
    AgentPromptEditorHint,
    AgentDefaultPrompt,
    AgentCustomPrompt,
    AgentPromptHint,
    AgentDefault,
    AgentNone,
    AgentSaved,
    AgentGenerate,
    AgentGenerating,
    AgentGenerated,
    AgentDraftChanged,
    AgentCancelled,
    AgentHint,
    AgentRunningHint,
    AgentModelInvalid,
    AgentPromptInvalid,
    AgentPromptConflict,
    AgentDiffTooLarge,
    AgentOutputTooLarge,
    AgentMessageInvalid,
    AgentStructuredOutputMissing,
    SettingsChecks,
    SettingsInput,
    SettingsConcurrency,
    SettingsConcurrencyHint,
    SettingsChineseTrigger,
    SettingsChineseTriggerHint,
    SettingsPreciseClick,
    SettingsPreciseClickHint,
    SettingsScrollHint,
    SettingsSelectPane,
    SettingsSelectPaneHint,

    MutationStashConflicts,
    BranchCreateCommand,
    BranchSwitchCommand,
    FetchCommand,
    AutoFetch,
    AutoFetchHint,
    AutoFetchInterval,
    AutoFetchIntervalHint,
    AutoFetchEnabled,
    AutoFetchDisabled,
    StartupFetch,
    StartupFetchHint,
    StartupFetchEnabled,
    StartupFetchDisabled,
    StartupFetchSaved,
    FetchAllCommand,
    PullCommand,
    SyncCommand,
    SyncPreparing,
    ProgressWaitingRepository,
    ProgressWaitingCapacity,
    ProgressInspectingRepository,
    ProgressFetchingRemote,
    ProgressComparingBranches,
    ProgressCheckingWorktree,
    ProgressVerifyingRemote,
    ProgressPreparingPreview,
    ProgressRefreshingState,
    ProgressReadingView,
    ProgressTimings,
    SyncCurrent,
    SyncNoRemotes,
    SyncNoUpstream,
    SyncLocalUpstream,
    SyncDetached,
    SyncUnborn,
    SyncDiverged,
    SyncPullReview,
    SyncPushReview,
    SyncPulled,
    SyncPushed,
    StashPreviewEmpty,
    StashPreviewNoMatch,
    StashPreviewAll,
    StashPreviewStaged,
    StashPreviewUnstaged,
    StashPreviewScope,
    StashBase,
    StashSavedWorktree,
    StashSavedIndex,
    StashAbsent,
    StashSavedUntracked,
    StashInvalidStructure,
    StashPreviewCommand,
    StashPreviewRequiresDetail,
    StashPreviewHint,
    StashDiffHint,
    StashWrappedDiffHint,
    StashTitle,
    StashRead,
    StashCommand,
    StashSaveIncomplete,
    StashSelectFiles,
    StashInvalidPath,
    StashInvalidMessage,
    StashSelectionChanged,
    StashScopeTracked,
    StashScopeAll,
    StashScopeSelected,
    StashScopeLabel,
    StashFilesCount,
    StashMessageOptional,
    StashDefaultMessage,
    StashSaveHint,
    StashSaveCommand,
    StashSaveAllCommand,
    StashApplyCommand,
    StashPopCommand,
    StashDropCommand,
    StashPosition,
    StashEmpty,
    StashDetails,
    StashSearch,
    StashHint,
    StashCopyCommand,
    StashObject,
    BranchNameHint,
    StashMessageHint,
    StashIdHint,
    StashIdRequired,
    MutationRun,
    MutationComplete,
    PullCurrent,
    PullReview,
    StashDropWarning,
    MutationTarget,

    MutationRequiresCommit,
    MutationDirtySubmodule,
    MutationNonRegularFile,
    MutationChanged,
    MutationCleanRequired,
    MutationInvalidBranch,
    MutationLocalBranchRequired,
    MutationAlreadyOnBranch,
    MutationBranchExists,
    MutationBranchInOtherWorktree,
    MutationStashMessageRequired,
    MutationNothingToStash,
    MutationStashChanged,
    MutationUpstreamRequired,
    MutationPullAttached,
    MutationPullDiverged,
    MutationRemoteNotUtf8,
    MutationFetchMappingRequired,
    MutationNoRemote,
    NoRemoteConfiguredSkipped,
    CopyRepositoryCommand,
    CopyDiffCommand,
    CopyLogCommand,
    CopyBranchCommand,
    CopyErrorCommand,
    CopyRepositoryPath,
    CopyFilePaths,
    CopyCommit,
    CopyBranch,
    CopyDiff,
    CopyError,
    CopyWriting,
    CopySucceeded,
    CopyFailed,
    CopyUnavailable,
    CopySelectionRequired,
    CopySelectionLoading,
    CopyPathNotUtf8,
    CopyDiffRequired,
    CopyDiffImage,
    CopyDiffTruncated,
    CopyErrorHint,
    ClipboardOpenFailed,
    ClipboardWriteFailed,
    ClipboardWorkerStopped,
    ClipboardBusy,
    RefreshContextView,
    WorkflowLegacyConflict,
    WorkflowPolicyString,
    WorkflowObjectExpected,
    WorkflowNoPush,
    WorkflowFetch,
    WorkflowPull,
    WorkflowPush,
    WorkflowInspect,
    WorkflowFetchDescription,
    WorkflowPullDescription,
    WorkflowPushDescription,
    WorkflowInspectDescription,
    WorkflowStartup,
    WorkflowScheduled,
    WorkflowInspectLast,
    WorkflowDuplicateStep,
    WorkflowPushNeedsRefresh,
    WorkflowUnknownStep,
    WorkflowScheduledActions,
    WorkflowToggle,
    WorkflowEnableStep,
    WorkflowDisableStep,
    WorkflowRequired,
    WorkflowSequence,
    WorkflowControls,
    WorkflowSelectedStep,
    WorkflowEditorHint,
    WorkflowMoveUp,
    WorkflowMoveDown,
    WorkflowSave,
    WorkflowCancel,
    WorkflowSaved,
    WorkflowEditing,
    WorkflowScope,
    WorkflowFixedInspect,
    WorkflowActionsArgument,
    WorkflowScheduleHint,
    WorkflowStopped,
    ContextCommands,
    ContextCommandPlaceholder,
    ContextCommandHint,
    ContextGlobalCommandHint,
    ContextPrefixHint,
    RepositoryFilterCommand,
    RepositoryFilterLabel,
    RepositoryFilterAll,
    RepositoryFilterProblems,
    RepositoryFilterClean,
    RepositoryFilterFailed,
    RepositoryFilterUnchecked,
    RepositoryFilterNoUpstream,
    RepositoryFilterPushes,
    RepositorySortCommand,
    RepositorySortLabel,
    RepositorySortPriority,
    RepositorySortName,
    RepositorySortPath,
    RepositorySortUpdated,
    RepositoryOrderPriority,
    RepositoryOrderName,
    RepositoryOrderPath,
    RepositoryOrderUpdated,
    RepositoryListOnly,
    RepositoryListHint,
    RepositoryListNoMatches,
    InvalidScheduleAnchor,
    AmbiguousScheduleAnchor,
    ScheduleEnableToSetAnchor,
    ScheduleAnchorLabel,
    ScheduleNextLabel,
    ScheduleAnchorAutomatic,
    ScheduleAnchorUnset,
    ScheduleNextDisabled,
    ScheduleNextUnanchored,
    ScheduleNextMissing,
    EditScheduleAnchor,
    ScheduleEditButton,
    ScheduleWorkflowButton,
    ScheduleEnableButton,
    ScheduleDisableButton,
    ScheduleApplyButton,
    ScheduleAnchorHint,
    SaveScheduleAnchor,

    LayoutVisible,
    LayoutCurrent,

    ChangesOpen,
    ChangesOpenStaged,
    ChangesOpenUnstaged,
    ChangesLayout,
    ChangesTitle,
    ChangesUnstaged,
    ChangesStaged,
    FileListClean,
    ChangesNoStaged,
    ChangesNoUnstaged,
    FileListNoMatches,
    ChangesSelectFile,
    ChangesSelectionRequired,
    ChangesWrongArea,
    ChangesInputHint,
    DirectoryInputHint,

    InvalidToml,
    ConfigurationMigrationRequired,
    ConfigurationMigrationConflict,
    TransferTargetExists,
    TransferSameSource,
    MigrateFromUsage,
    StorageMigrationRequired,
    InvalidStateDatabase,
    UnsupportedStateDatabase,
    StateDatabaseRequiresWal,
    InvalidHistoryLimit,
    LegacyStateConflict,
    LegacyStateChanged,

    RepositoryStateSaveFailed,
    RepositoryTaskBusy,
    RepositoryTaskLimit,
    RepositoryTasksRunning,
    RepositoryTaskCompleted,
    ImageInputHint,
    ImagePreview,
    ImageAbsent,
    ImageNotRaster,
    ImageEncodedLimit,
    ImageDecodedLimit,
    ImageDecodeFailed,
    ImagePreparing,
    ImageFormats,

    BranchTitle,
    BranchCatalogRead,
    BranchCommand,
    BranchTipTextInvalid,
    BranchAll,
    BranchLocal,
    BranchRemote,
    BranchEmpty,
    BranchSearch,
    BranchHint,
    BranchReference,
    BranchLocalRefs,

    WorkspaceOverview,
    WorkspaceHistory,
    WorkspaceFocusHint,
    WorkspaceMaximizeHint,
    WorkspaceRestoreHint,

    OverviewUpstream,
    OverviewMailboxOrRebase,

    OverviewTitle,
    OverviewLoading,
    OverviewSnapshot,
    OverviewCheckout,
    OverviewUnborn,
    OverviewDetached,
    OverviewOperation,
    OverviewStaged,
    OverviewUnstaged,
    OverviewUntracked,
    OverviewConflicts,
    OverviewShallow,
    OverviewGone,
    OverviewCheck,
    OverviewInputHint,
    RepositoryPanesHint,
    OverviewMerging,
    OverviewRebasing,
    OverviewReverting,
    OverviewCherryPicking,
    OverviewBisecting,
    OverviewApplyingMail,

    GraphicsUnsupported,
    GraphicsMetricsUnavailable,
    GraphicsTooLarge,
    GraphicsPreparing,
    GraphicsWorkerStopped,
    HistoryGraftedParents,
    HistoryNoFilesMatch,
    HistoryShallow,
    HistoryParentUnavailable,
    HistoryGraphTooWide,
    HistoryOpenCommitFirst,
    HistoryTitle,
    HistoryCommand,
    HistoryMoreCommand,
    HistoryRead,
    HistoryReadCommit,
    HistoryReadDiff,
    HistoryEmpty,
    HistoryEnd,
    HistoryMore,
    HistoryMoreSearch,
    HistoryLoaded,
    HistoryScope,
    HistoryNoMore,
    HistoryRequiresOpen,
    HistorySearch,
    HistorySearchHint,
    HistoryInputHint,
    HistoryCommitInputHint,
    HistoryReviewInputHint,
    HistoryDiffHint,
    HistoryNoMatch,
    HistoryHash,
    HistoryParents,
    HistoryAuthor,
    HistoryCommitter,
    HistoryAuthorTime,
    HistoryCommitterTime,
    HistoryTime,
    HistoryMessage,
    HistoryRefs,
    HistoryChanges,
    HistoryNoChanges,
    HistoryRoot,
    HistoryCompare,
    HistoryTruncated,
    HistoryUnborn,
    HistoryInvalidTime,
    HistoryReferenceNotUtf8,
    HistoryReferenceNotCommit,
    HistoryCursorRepositoryChanged,
    HistorySnapshotUnavailable,
    HistoryTextNotUtf8,
    HistoryParentNotInCommit,
    HistoryUnsupportedChange,
    DiffAuto,
    DiffSplit,
    DiffUnified,
    DiffChooseAuto,
    DiffChooseSplit,
    FilesLayout,
    FilesTree,
    FilesFlat,
    DiffChooseUnified,
    PartialSelectionIdle,
    DiffContextCommand,
    DiffContextFull,
    DiffContextChanges,
    DiffFullUnavailable,
    DiffFullLineLimit,
    DiffFullTruncated,
    DiffHeaderCommand,
    DiffHeaderUnavailable,
    DiffHeaderCollapsed,
    DiffHeaderExpanded,
    DiffHeaderAdded,
    DiffHeaderDeleted,
    DiffHeaderMode,
    DiffHeaderRenamed,
    DiffHeaderCopied,
    DiffHeaderSimilarity,
    DiffHeaderDissimilarity,
    DiffModeRegular,
    DiffModeExecutable,
    DiffModeSymlink,
    DiffModeSubmodule,
    DiffLayoutSelected,
    DiffBefore,
    DiffAfter,
    DiffTooNarrow,
    InvalidDiffStructure,
    DiffSourceChanged,
    SyntaxLabel,
    SyntaxUnknown,
    SyntaxSourceLimit,
    SyntaxLineLimit,
    SyntaxLineCountLimit,
    SyntaxBinary,
    SyntaxNonRegular,
    ChineseCommandTrigger,
    MouseSettings,
    MousePreciseClick,
    MouseScroll,
    MouseSelectPane,
    MouseSettingsSaved,
    ChineseCommandTriggerSaved,
    AutoPushDisabledOption,
    AutoPushFastForwardOption,
    AgentCodexOption,
    AgentClaudeOption,
    BooleanArgumentRequired,
    BrowserWrappedPreviewHint,
    HistoryWrappedDiffHint,
    WrappedDiffPreviewHint,
    WrappedChangesInputHint,
    TextWrap,
    TextWrapHint,
    FileIcons,
    FileIconsHint,
    TextWrapEnabled,
    TextWrapDisabled,
    SettingsDisplay,
    PathArgumentHint,
    UnsupportedNamedHome,
    ExpandedPathNotUtf8,
    DirectoryCandidates,
    DirectoryCompletionHint,
    ReadCompletionDirectory,
    NoMatchingDirectories,
    UneditableDirectoryName,
    DurationArgumentHint,
    PolicyArgumentHint,
    CountArgumentHint,
    MissingCommandArgument,
    UnclosedPathQuote,

    ScheduleOwnedByAnotherInstallation,
    ScheduleDefinitionMissing,
    EnableScheduledTask,
    DisableScheduledTask,
    InvalidScheduleOperation,
    ScheduleEnabledNoCheck,
    ScheduleDisabledPreserved,
    ScheduleAppliedPreserved,
    ScheduleEnableConfirmation,
    ScheduleDisableConfirmation,
    ScheduleApplyConfirmation,
    TaskStatusLabel,
    CheckProcessLabel,
    TaskEnabled,
    TaskDisabled,
    TaskMissing,
    IntervalLabel,
    AutopushLabel,
    ApplicationLabel,
    IntervalApplied,
    IntervalUninstalled,
    IntervalPending,
    ScheduleActions,
    EditInterval,
    EditAutopush,
    ScheduleHelpHint,
    PolicyAppliesNextRun,

    GeneralSettings,
    OpenRepositoryOverview,
    ManageRepositories,
    RepositoryManagementSettings,
    RefreshRepositoryReport,
    RepositoryManagement,
    ConfigureSchedule,
    ScheduleSettings,

    OverlappingFetchDestinations,
    FetchRefspecNotUtf8,
    InvalidFetchRefspec,
    UpstreamExcludedFromFetch,
    UnsupportedFetchDestination,
    UpstreamFetchMappingNotUnique,
    IncompleteBranchUpstream,
    UpstreamRemoteFetchSkipped,

    ConfirmSelectHint,
    ConfirmReviewHint,
    ActionCommit,
    ActionPush,
    ActionApply,
    ActionUnregister,
    ActionSave,
    ActionAdd,
    ActionExclude,
    ActionUnexclude,
    ActionScan,
    ActionImport,
    ActionReturnEdit,
    ActionCancel,

    SingleReportSaveFailed,
    CheckAtMostOnePath,
    CheckUpdatedAt,
    CheckCancelledRetained,
    CheckOneComplete,
    CancelBackgroundCheck,
    BackgroundCheckCancelling,
    BackgroundCheckProgress,
    CheckWriteConflict,
    NoBackgroundCheck,
    CheckCancelRequested,
    CheckAlreadyRunning,
    InvalidRepositoryCheckReport,
    SyncPushFailedCount,
    SyncPushedCount,
    Synced,
    SyncColumn,
    RepositoryErrorsFilter,
    HomeCheckComplete,
    SystemdPathControlUnsupported,
    UnexpectedSystemdTimerState,
    XdgDirectoryMustBeAbsolute,
    LinuxTerminalLauncherRequired,
    ReadDefaultTerminalFailed,
    DefaultTerminalNotConfigured,
    OpenRepositoryTerminal,
    RepositoryTerminalOpened,
    TerminalDirectoryMissing,
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
    ConfigurationMissingRunInit,
    HomeMustBeAnAbsolutePath,
    StatusOnlyAcceptsJson,
    NoCompleteReportYetWaitForThe,
    AddRequiresAtLeastOneRepositoryPath,
    AddedNRepositories,
    ConfigurationSavedApplySchedule,
    FoundRepositoriesFailedCouldNotBeRead,
    Checking,
    UnableToPrintResultE,
    SystemNotificationFailedN,
    CheckCompleteRepositories,
    InstalledCommandAndSystemSchedule,
    ScheduleRemovedConfigurationReportsAndCommandRetained,
    UnknownCommandShowHelp,
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
    IntervalNotAppliedApplySchedule,
    ExcludedDirectory,
    LatestReportNoCompleteReportYet,
    LastCheckLatestResultCleanNeedAttention,
    OtherRepositoriesFailedToCheckTheseCounts,
    SystemNotificationsFailedSeePushReport,
    LatestReportDidNotPushScheduledPolicy,
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
    PushReportDetails,
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
    Failed,
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
    QuerySystemSchedule,
    SchedulePathIsNotUtf8,
    ReadScheduledInterval,
    InvalidScheduledInterval,
    InvalidInstallationArguments,
    InstalledCommandOwnershipMismatch,
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
    InteractiveTerminalRequired,
    UnableToRestoreTerminalCursor,
    SystemNotificationFailedNotice,
    BackgroundWorkerTerminatedUnexpectedly,
    BackgroundWorkerPanicked,
    OperationInterruptedSomeWritesMayHaveCompleted,
    CommittedRepositoryUpdated,
    ContentsChangedVerifyTheCommit,
    PushedRepositoryUpdated,
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
    ApplySystemSchedule,
    SystemScheduleAppliedNoCheckStarted,
    UnregisterRepository,
    RepositoryUnregisteredFilesRetained,
    AddedRepositories,
    FoundRepositoriesScanSaved,
    ConfigurationSavedApplyScheduleAfterChangingThe,
    RepositoryBranchScopeStagedContentsOnlyStaged,
    FilesCurrentState,
    Repositories,
    RepositoryDetails,
    Settings,
    InteractionCommand,
    InteractionApplicationCommand,
    InteractionTextCommand,
    InteractionApplication,
    InteractionText,
    InteractionSetting,
    InteractionHint,
    InteractionSaveFailed,
    ViewPreferenceSaveFailed,
    Help,
    CheckingAlt,
    ScheduleNotLoaded,
    NoCompleteReportYet,
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
    ApplySavedIntervalAndReloadTheSystem,
    UnregisterRepositoryFilesOnDiskAreRetained,
    ConfirmOperation,
    CurrentRepositoryIsNotInTheFiltered,
    Instructions,
    BrowserTitle,
    BrowserDescription,
    BrowserDiffDescription,
    BrowserRefreshDescription,
    BrowserSearch,
    BrowserPlaceholder,
    BrowserHint,
    BrowserLoading,
    BrowserReading,
    BrowserSearching,
    BrowserSearchLimited,
    BrowserSelectFile,
    BrowserEmpty,
    BrowserInvalidPath,
    BrowserSpecialFile,
    BrowserNotUtf8,
    BrowserMissing,
    BrowserBinary,
    BrowserTooLarge,
    BrowserDirectory,
    BrowserSymlink,
    BrowserHighlightLimit,
    BrowserNoDiff,
    LinkOpen,
    LinkOpening,
    LinkRequested,
    LinkOpenFailed,
    LinkPathMissing,
    FileSource,
    FileMarkdownPreview,
    FileSvgPreview,
    FileSplit,
    FileSingle,
    FileSourceCommand,
    FileModeCommand,
    FileLayoutCommand,
    ComposeReorderCommand,
    ScheduleEnabledCommand,
    ComposeFirstStep,
    ComposeLastStep,
    FilePreviewCommand,
    SvgInvalid,
    SvgExternal,
    CurrentRepo,
    Global,
    CurrentView,
    Commands,
    NoRepositorySelected,
    NoMatchingCommands,
    TuiHelp,
    Repos,
    Search,
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
    RemoteRepositories,
    RemotePush,
    RemoteNone,
    BranchSync,
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
    SearchEditingHint,
    SelectSpaceInsMarkCtrlAAll,
    ScrollEscFilesCommands,
    SelectEnterEditBackCommands,
    PgdnScrollEscBackCommands,
    CommandsAlt,
    SearchFiles,
    SearchRepositories,
    TypeToSearchOrForCommands,
    CommitMessageAlt,
    DescribeThisCommit,
    EnterConfirmEscCancelPgupPgdnReview,
    ConfirmOperationAlt,
    ConfirmCommitAlt,
    ConfirmPushAlt,
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
            (Self::CommandHistory, Language::Chinese) => "命令历史",
            (Self::CommandHistory, Language::English) => "Command history",
            (Self::CommandHistoryEntryHint, Language::Chinese) => {
                "↑ 首项历史 · ↓候选 · Tab 补全 · Enter 执行 · Esc 清空"
            }
            (Self::CommandHistoryEntryHint, Language::English) => {
                "↑ at top: history · ↓ candidates · Tab complete · Enter run · Esc close"
            }
            (Self::CommandHistoryEmpty, Language::Chinese) => "暂无命令历史",
            (Self::CommandHistoryEmpty, Language::English) => "No command history yet",
            (Self::CommandHistoryHint, Language::Chinese) => {
                "↑↓选择 · Enter/Tab 填入 · Esc 返回命令"
            }
            (Self::CommandHistoryHint, Language::English) => {
                "↑↓ select · Enter/Tab fill · Esc commands"
            }
            (Self::InvalidCommandHistory, Language::Chinese) => "命令历史格式或容量不受支持",
            (Self::InvalidCommandHistory, Language::English) => {
                "Unsupported command history format or size"
            }
            (Self::ReadCommandHistoryFailed, Language::Chinese) => "读取命令历史失败：{0}",
            (Self::ReadCommandHistoryFailed, Language::English) => {
                "Could not read command history: {0}"
            }
            (Self::SaveCommandHistoryFailed, Language::Chinese) => "保存命令历史失败：{0}",
            (Self::SaveCommandHistoryFailed, Language::English) => {
                "Could not save command history: {0}"
            }
            (Self::WorktreeHorizontalHint, Language::Chinese) => "↑↓选择 · ←→ 横向滚动 · Enter 差异 · Esc 返回 · Space 勾选",
            (Self::WorktreeHorizontalHint, Language::English) => "↑↓ select · ←→ scroll · Enter diff · Esc back · Space check",
            (Self::ConflictControlsTitle, Language::Chinese) => "冲突处理",
            (Self::ConflictControlsTitle, Language::English) => "Resolve conflicts",
            (Self::ConflictControlsHint, Language::Chinese) => {
                "Shift+Tab 切换页 · ←→选项 · ↑↓选块 · Enter 执行 · Esc 输入"
            }
            (Self::ConflictControlsHint, Language::English) => {
                "Shift+Tab next page · ←→ action · ↑↓ block · Enter run · Esc input"
            }
            (Self::ConflictChoiceLeft, Language::Chinese) => "左（当前）",
            (Self::ConflictChoiceLeft, Language::English) => "Left (current)",
            (Self::ConflictChoiceRight, Language::Chinese) => "右（传入）",
            (Self::ConflictChoiceRight, Language::English) => "Right (incoming)",
            (Self::ConflictChoiceBoth, Language::Chinese) => "双方",
            (Self::ConflictChoiceBoth, Language::English) => "Both",
            (Self::ConflictChoiceUndo, Language::Chinese) => "撤销",
            (Self::ConflictChoiceUndo, Language::English) => "Undo",
            (Self::ConflictChoiceReset, Language::Chinese) => "重置",
            (Self::ConflictChoiceReset, Language::English) => "Reset",
            (Self::ConflictChoiceStage, Language::Chinese) => "暂存",
            (Self::ConflictChoiceStage, Language::English) => "Stage",

            (Self::ConflictReadingHint, Language::Chinese) => {
                "↑↓滚动 · PgUp/PgDn 翻页 · /editor 编辑 · Esc 文件"
            }
            (Self::ConflictReadingHint, Language::English) => {
                "↑↓ scroll · PgUp/PgDn page · /editor edit · Esc files"
            }
            (Self::ConflictResolvedHint, Language::Chinese) => {
                "Enter/Space 操作栏 · ↑↓滚动 · Esc 文件 · / 查看命令"
            }
            (Self::ConflictResolvedHint, Language::English) => {
                "Enter/Space actions · ↑↓ scroll · Esc files · / commands"
            }
            (Self::ConflictAcceptLeft, Language::Chinese) => "采用左（当前）",
            (Self::ConflictAcceptLeft, Language::English) => "Accept left (current)",
            (Self::ConflictAcceptRight, Language::Chinese) => "采用右（传入）",
            (Self::ConflictAcceptRight, Language::English) => "Accept right (incoming)",
            (Self::ConflictAcceptBoth, Language::Chinese) => "保留双方（左后右）",
            (Self::ConflictAcceptBoth, Language::English) => "Keep both (left then right)",
            (Self::ConflictResolveUnsupported, Language::Chinese) => {
                "此文件无法进行内置冲突处理，请使用编辑器"
            }
            (Self::ConflictResolveUnsupported, Language::English) => {
                "This file requires an external editor to resolve"
            }
            (Self::ConflictResolveChanged, Language::Chinese) => "文件或仓库已变化，请刷新后重试",
            (Self::ConflictResolveChanged, Language::English) => {
                "File or repository changed; refresh before resolving"
            }
            (Self::ConflictMarkersInvalid, Language::Chinese) => {
                "冲突标记不完整或存在嵌套，请使用编辑器"
            }
            (Self::ConflictMarkersInvalid, Language::English) => {
                "Incomplete or nested conflict markers; use an editor"
            }
            (Self::ConflictBlockRequired, Language::Chinese) => "没有待处理的冲突块",
            (Self::ConflictBlockRequired, Language::English) => "No unresolved conflict block",
            (Self::ConflictAcceptCommand, Language::Chinese) => "采用当前冲突块的版本",
            (Self::ConflictAcceptCommand, Language::English) => {
                "Resolve the selected conflict block"
            }
            (Self::ConflictBlockCommand, Language::Chinese) => "选择冲突块",
            (Self::ConflictBlockCommand, Language::English) => "Select a conflict block",
            (Self::ConflictBlockPrevious, Language::Chinese) => "上一个冲突块",
            (Self::ConflictBlockPrevious, Language::English) => "Previous conflict block",
            (Self::ConflictBlockNext, Language::Chinese) => "下一个冲突块",
            (Self::ConflictBlockNext, Language::English) => "Next conflict block",
            (Self::ConflictResetCommand, Language::Chinese) => "重置冲突结果",
            (Self::ConflictResetCommand, Language::English) => "Reset conflict result",
            (Self::ConflictResetConfirm, Language::Chinese) => {
                "重置当前文件的全部解决结果，重新生成冲突块。请检查将被覆盖的内容。"
            }
            (Self::ConflictResetConfirm, Language::English) => {
                "Reset all resolutions in this file and recreate its conflict blocks. Review the content that will be replaced."
            }
            (Self::ConflictResetUnsupported, Language::Chinese) => {
                "此文件的冲突无法用内置文本合并重置"
            }
            (Self::ConflictResetUnsupported, Language::English) => {
                "This conflict cannot be reset with the built-in text merger"
            }
            (Self::ConflictUndoCommand, Language::Chinese) => "撤销上次冲突处理",
            (Self::ConflictUndoCommand, Language::English) => "Undo last conflict change",
            (Self::ConflictUndoUnavailable, Language::Chinese) => "没有可撤销的冲突处理",
            (Self::ConflictUndoUnavailable, Language::English) => "No conflict resolution to undo",
            (Self::ConflictResolvedPendingStage, Language::Chinese) => "冲突块已处理，待暂存",
            (Self::ConflictResolvedPendingStage, Language::English) => {
                "Blocks resolved; stage the result"
            }
            (Self::ConflictBlockCount, Language::Chinese) => "冲突块 {current}/{total}",
            (Self::ConflictBlockCount, Language::English) => "Conflict {current}/{total}",

            (Self::CopyConflictUnavailable, Language::Chinese) => "冲突版本无法完整读取为文本",
            (Self::CopyConflictUnavailable, Language::English) => {
                "Conflict versions cannot be read completely as text"
            }
            (Self::ConflictCompareCurrent, Language::Chinese) => "基础和左（当前）",
            (Self::ConflictCompareCurrent, Language::English) => "Base and left (current)",
            (Self::ConflictCompareIncoming, Language::Chinese) => "基础和右（传入）",
            (Self::ConflictCompareIncoming, Language::English) => "Base and right (incoming)",
            (Self::ConflictCompareResult, Language::Chinese) => "基础和结果",
            (Self::ConflictCompareResult, Language::English) => "Base and result",
            (Self::ConflictCompareSides, Language::Chinese) => "左（当前）和右（传入）",
            (Self::ConflictCompareSides, Language::English) => {
                "Left (current) and right (incoming)"
            }
            (Self::ConflictCompareLeftResult, Language::Chinese) => "左（当前）和结果",
            (Self::ConflictCompareLeftResult, Language::English) => "Left (current) and result",
            (Self::ConflictCompareResultRight, Language::Chinese) => "结果和右（传入）",
            (Self::ConflictCompareResultRight, Language::English) => "Result and right (incoming)",
            (Self::CopyConflictResult, Language::Chinese) => "复制工作区结果源码",
            (Self::CopyConflictResult, Language::English) => "Copy working result source",
            (Self::ConflictIncludeBase, Language::Chinese) => "包含基础版本",
            (Self::ConflictIncludeBase, Language::English) => "Include base",
            (Self::ConflictOverview, Language::Chinese) => "左、右和结果",
            (Self::ConflictOverview, Language::English) => "Left, right and result",
            (Self::CopyConflictVersions, Language::Chinese) => "复制冲突版本",
            (Self::CopyConflictVersions, Language::English) => "Copy conflict versions",
            (Self::ConflictInputHint, Language::Chinese) => {
                "↑↓选择冲突块 · Enter/Space 操作栏 · PgUp/PgDn 滚动 · ←→平移 · Esc 文件"
            }
            (Self::ConflictInputHint, Language::English) => {
                "↑↓ conflict blocks · Enter/Space actions · PgUp/PgDn scroll · ←→ pan · Esc files"
            }
            (Self::ConflictBase, Language::Chinese) => "基础",
            (Self::ConflictBase, Language::English) => "Base",
            (Self::ConflictCurrent, Language::Chinese) => "左（当前）",
            (Self::ConflictCurrent, Language::English) => "Left (current)",
            (Self::ConflictIncoming, Language::Chinese) => "右（传入）",
            (Self::ConflictIncoming, Language::English) => "Right (incoming)",
            (Self::ConflictFileRequired, Language::Chinese) => "请先选择冲突文件",
            (Self::ConflictFileRequired, Language::English) => "Select a conflicted file first",
            (Self::ConflictCompare, Language::Chinese) => "版本比较",
            (Self::ConflictCompare, Language::English) => "Input comparison",
            (Self::ConflictResult, Language::Chinese) => "工作区结果",
            (Self::ConflictResult, Language::English) => "Worktree result",
            (Self::ConflictViewCommand, Language::Chinese) => "冲突版本显示",
            (Self::ConflictViewCommand, Language::English) => "Conflict versions",
            (Self::ConflictStage, Language::Chinese) => "索引阶段",
            (Self::ConflictStage, Language::English) => "Index stage",
            (Self::ConflictAbsent, Language::Chinese) => "此版本没有该文件",
            (Self::ConflictAbsent, Language::English) => "File absent in this version",
            (Self::ConflictEmpty, Language::Chinese) => "空文件",
            (Self::ConflictEmpty, Language::English) => "Empty file",
            (Self::ConflictLineLimit, Language::Chinese) => "文本比较超过 50,000 行上限",
            (Self::ConflictLineLimit, Language::English) => {
                "Text comparison exceeds the 50,000-line limit"
            }
            (Self::ConflictNonText, Language::Chinese) => "此版本无法完整读取为文本",
            (Self::ConflictNonText, Language::English) => {
                "This version is not plain text; line comparison is unavailable"
            }
            (Self::ConflictResultHint, Language::Chinese) => {
                "采用：/repo diff conflict accept · 编辑：/editor"
            }
            (Self::ConflictResultHint, Language::English) => {
                "Accept: /repo diff conflict accept · Edit: /editor"
            }
            (Self::SearchTargetContent, Language::English) => "content",
            (Self::SearchTargetContent, Language::Chinese) => "内容",
            (Self::ContentSearchPlaceholder, Language::English) => {
                "Find text in this file, or / for commands"
            }
            (Self::ContentSearchPlaceholder, Language::Chinese) => {
                "输入文字定位文件内容，或输入 / 查看命令"
            }
            (Self::CommandTargetReadingBranch, Language::English) => "Reading branch…",
            (Self::CommandTargetReadingBranch, Language::Chinese) => "读取分支…",
            (Self::CommandTargetBranchFailed, Language::English) => "Branch unavailable",
            (Self::CommandTargetBranchFailed, Language::Chinese) => "分支读取失败",
            (Self::RefreshUnavailable, Language::English) => {
                "No local data to refresh in this view"
            }
            (Self::RefreshUnavailable, Language::Chinese) => "当前视图没有可刷新的本地数据",
            (Self::RefreshHistorical, Language::English) => {
                "Historical snapshots do not need refreshing"
            }
            (Self::RefreshHistorical, Language::Chinese) => "历史快照无需刷新",
            (Self::TerminalSelectionRequired, Language::English) => {
                "Select a repository, file or folder to open its directory"
            }
            (Self::TerminalSelectionRequired, Language::Chinese) => {
                "请选择仓库、文件或目录以打开终端"
            }
            (Self::ExternalEditorCurrent, Language::English) => "Current setting",
            (Self::ExternalEditorCurrent, Language::Chinese) => "当前配置",
            (Self::ExternalEditorOpening, Language::English) => "An editor is already opening",
            (Self::ExternalEditorOpening, Language::Chinese) => "正在打开编辑器",
            (Self::ExternalEditorGit, Language::English) => "Follow Git",
            (Self::ExternalEditorGit, Language::Chinese) => "跟随 Git",
            (Self::ExternalEditorInvalid, Language::English) => {
                "Editor command must be nonempty, at most 4096 bytes and contain no NUL."
            }
            (Self::ExternalEditorInvalid, Language::Chinese) => {
                "编辑器命令不能为空，最多 4096 字节，且不能包含 NUL。"
            }
            (Self::ExternalEditorDetectionFailed, Language::English) => {
                "Unable to detect installed editors"
            }
            (Self::ExternalEditorDetectionFailed, Language::Chinese) => "检测编辑器失败",
            (Self::ExternalEditorMissing, Language::English) => "Editor not found: {0}",
            (Self::ExternalEditorMissing, Language::Chinese) => "未找到编辑器：{0}",
            (Self::ExternalEditorNoFolders, Language::English) => {
                "This editor cannot open folders. Choose a code editor in /settings editor."
            }
            (Self::ExternalEditorNoFolders, Language::Chinese) => {
                "这个编辑器不支持打开目录，请在 /settings editor 中选择代码编辑器。"
            }
            (Self::ExternalEditorNeedsTerminal, Language::English) => {
                "This editor needs an interactive terminal"
            }
            (Self::ExternalEditorNeedsTerminal, Language::Chinese) => "这个编辑器需要交互式终端",
            (Self::ExternalEditorFailed, Language::English) => "Editor exited unsuccessfully",
            (Self::ExternalEditorFailed, Language::Chinese) => "编辑器异常退出",
            (Self::ExternalEditorOpen, Language::English) => "Open selection in editor",
            (Self::ExternalEditorOpen, Language::Chinese) => "用编辑器打开当前选择",
            (Self::ExternalEditorSetting, Language::English) => "Editor",
            (Self::ExternalEditorSetting, Language::Chinese) => "编辑器",
            (Self::ExternalEditorHint, Language::English) => {
                "Follow Git, choose an installed editor, or enter an editor command with arguments."
            }
            (Self::ExternalEditorHint, Language::Chinese) => {
                "跟随 Git、选择已安装的编辑器，或输入带参数的编辑器命令。"
            }
            (Self::ExternalEditorSelectionRequired, Language::English) => {
                "Select a repository, working file or folder"
            }
            (Self::ExternalEditorSelectionRequired, Language::Chinese) => {
                "请选择仓库、工作区文件或目录"
            }
            (Self::ExternalEditorOpened, Language::English) => "Opened in editor: {0}",
            (Self::ExternalEditorOpened, Language::Chinese) => "已用编辑器打开：{0}",
            (Self::ExternalEditorTools, Language::English) => "External tools",
            (Self::ExternalEditorTools, Language::Chinese) => "外部工具",
            (Self::LocalObservationFailed, Language::English) => "Local refresh failed · {0}: {1}",
            (Self::LocalObservationFailed, Language::Chinese) => "本地刷新失败 · {0}：{1}",
            (Self::CommandRequiresWorktreePreview, Language::English) => {
                "Focus the working tree diff preview first"
            }
            (Self::CommandRequiresWorktreePreview, Language::Chinese) => "请先聚焦工作区差异预览",
            (Self::CommandRequiresFileTarget, Language::English) => "Select or open a file first",
            (Self::CommandRequiresFileTarget, Language::Chinese) => "请先选择或打开文件",
            (Self::PartialSelectRequired, Language::English) => {
                "Select a hunk or lines in the diff preview first"
            }
            (Self::PartialSelectRequired, Language::Chinese) => "请先在差异预览中选择变更块或行",
            (Self::PartialChanged, Language::English) => {
                "File or index changed; refresh and select again"
            }
            (Self::PartialChanged, Language::Chinese) => "文件或索引已变化，请刷新后重新选择",
            (Self::PartialSelectionConflict, Language::English) => {
                "These lines cannot be applied independently; expand the selection"
            }
            (Self::PartialSelectionConflict, Language::Chinese) => "这些行无法独立应用，请扩大选区",
            (Self::PartialUnsupported, Language::English) => {
                "This diff does not support hunk or line operations"
            }
            (Self::PartialUnsupported, Language::Chinese) => "此差异不支持按块或按行操作",
            (Self::PartialSelectMode, Language::English) => "Choose hunk or line selection",
            (Self::PartialSelectMode, Language::Chinese) => "选择变更粒度",
            (Self::PartialHunk, Language::English) => "Select hunks",
            (Self::PartialHunk, Language::Chinese) => "按块选择",
            (Self::PartialLine, Language::English) => "Select lines",
            (Self::PartialLine, Language::Chinese) => "按行选择",
            (Self::PartialDiscard, Language::English) => "Discard selected changes",
            (Self::PartialDiscard, Language::Chinese) => "丢弃所选变更",
            (Self::PartialDiscardConfirm, Language::English) => "Discard selected unstaged changes",
            (Self::PartialDiscardConfirm, Language::Chinese) => "丢弃所选未暂存改动",
            (Self::DiscardFile, Language::English) => "Discard whole file changes",
            (Self::DiscardFile, Language::Chinese) => "丢弃整文件改动",
            (Self::DiscardFileSelect, Language::English) => {
                "Select a file in the worktree changes list"
            }
            (Self::DiscardFileSelect, Language::Chinese) => "请在工作区变更列表中选中文件",
            (Self::DiscardFileUnsupported, Language::English) => {
                "This entry does not support whole-file discard"
            }
            (Self::DiscardFileUnsupported, Language::Chinese) => "此条目不支持整文件丢弃",
            (Self::DiscardFileRestoreReview, Language::English) => "Restore from the index",
            (Self::DiscardFileRestoreReview, Language::Chinese) => "从暂存区恢复",
            (Self::DiscardFileRemoveReview, Language::English) => {
                "Permanently delete untracked files"
            }
            (Self::DiscardFileRemoveReview, Language::Chinese) => "永久删除未跟踪文件",
            (Self::DiscardFileInterrupted, Language::English) => {
                "Discard stopped at {path} ({count}/{total} files completed)"
            }
            (Self::DiscardFileInterrupted, Language::Chinese) => {
                "丢弃在 {path} 处中止，已处理 {count}/{total} 个文件"
            }
            (Self::PartialStage, Language::English) => "Stage selected changes",
            (Self::PartialStage, Language::Chinese) => "暂存所选变更",
            (Self::PartialUnstage, Language::English) => "Unstage selected changes",
            (Self::PartialUnstage, Language::Chinese) => "取消暂存所选变更",
            (Self::PartialHunkHint, Language::English) => {
                "↑↓ hunks · /select lines · Space stage/unstage · /focus files"
            }
            (Self::PartialHunkHint, Language::Chinese) => {
                "↑↓选块 · /select 切换行 · Space 暂存/取消 · /focus 文件"
            }
            (Self::PartialLineHint, Language::English) => {
                "↑↓ lines · Shift+↑↓ range · Space stage/unstage · /focus files"
            }
            (Self::PartialLineHint, Language::Chinese) => {
                "↑↓选行 · Shift+↑↓扩选 · Space 暂存/取消 · /focus 文件"
            }
            (Self::ManageRemove, Language::English) => "Remove",
            (Self::ManageRemove, Language::Chinese) => "移除",
            (Self::ManageBack, Language::English) => "Back",
            (Self::ManageBack, Language::Chinese) => "返回",
            (Self::ScanUsage, Language::English) => "Usage: <path> [--exclude <path>…]",
            (Self::ScanUsage, Language::Chinese) => "用法：<path> [--exclude <path>…]",
            (Self::ScanExclusionOutsideRoot, Language::English) => {
                "Excluded directories must be inside the scan root"
            }
            (Self::ScanExclusionOutsideRoot, Language::Chinese) => "排除目录必须位于扫描起点之下",
            (Self::ScanResults, Language::English) => "Scan results",
            (Self::ScanResults, Language::Chinese) => "扫描结果",
            (Self::ScanNew, Language::English) => "Not added",
            (Self::ScanNew, Language::Chinese) => "未添加",
            (Self::ScanRegistered, Language::English) => "Added",
            (Self::ScanRegistered, Language::Chinese) => "已添加",
            (Self::ScanFailed, Language::English) => "Unreadable",
            (Self::ScanFailed, Language::Chinese) => "读取失败",
            (Self::ScanAddSelected, Language::English) => "Add selected",
            (Self::ScanAddSelected, Language::Chinese) => "添加选中",
            (Self::ScanEmptySelection, Language::English) => "Select repositories to add",
            (Self::ScanEmptySelection, Language::Chinese) => "请勾选要添加的仓库",
            (Self::ScanHint, Language::English) => {
                "Space/Ins mark · Ctrl+A all · Shift+Tab actions · Enter mark · Esc back"
            }
            (Self::ScanHint, Language::Chinese) => {
                "Space/Ins 勾选 · Ctrl+A 全选 · Shift+Tab 操作 · Enter 勾选 · Esc 返回"
            }
            (Self::ManageHint, Language::English) => "↑↓ select · / commands · Esc back",
            (Self::ManageHint, Language::Chinese) => "↑↓选择 · / 命令 · Esc 返回",
            (Self::ScanActionsTitle, Language::English) => "Scan actions",
            (Self::ScanActionsTitle, Language::Chinese) => "扫描操作",
            (Self::WorkflowFinishTitle, Language::English) => "Save workflow",
            (Self::WorkflowFinishTitle, Language::Chinese) => "保存流程",
            (Self::WorkflowFinishHint, Language::English) => {
                "←→ choose · Enter run · Shift+Tab step actions · Esc back"
            }
            (Self::WorkflowFinishHint, Language::Chinese) => {
                "←→选择 · Enter 执行 · Shift+Tab 步骤操作 · Esc 返回"
            }
            (Self::ScanActionsHint, Language::English) => {
                "←→ choose · Enter run · Shift+Tab input · Esc input"
            }
            (Self::ScanActionsHint, Language::Chinese) => {
                "←→选择 · Enter 执行 · Shift+Tab 输入 · Esc 输入"
            }
            (Self::ScanTemporaryExclusions, Language::English) => "Skipped for this scan",
            (Self::ScanTemporaryExclusions, Language::Chinese) => "本次跳过",
            (Self::PushConfirmationNeeded, Language::English) => {
                "Push has not run: confirmation required. Run /repo push again to review."
            }
            (Self::PushConfirmationNeeded, Language::Chinese) => {
                "推送尚未执行：需要确认，可再次执行 /repo push 查看。"
            }
            (Self::PushTargetNeeded, Language::English) => {
                "Push has not run: choose a destination. Run /repo push again to choose."
            }
            (Self::PushTargetNeeded, Language::Chinese) => {
                "推送尚未执行：需要选择目标，可再次执行 /repo push 选择。"
            }
            (Self::PushRemoteRequired, Language::English) => "Select a configured remote",
            (Self::PushRemoteRequired, Language::Chinese) => "请选择已配置的远程仓库",
            (Self::PushInvalidBranch, Language::English) => "Enter a valid remote branch name",
            (Self::PushInvalidBranch, Language::Chinese) => "请输入有效的远程分支名",
            (Self::PushTargetTitle, Language::English) => "Publish branch",
            (Self::PushTargetTitle, Language::Chinese) => "发布分支",
            (Self::PushTargetBranch, Language::English) => "Remote branch",
            (Self::PushTargetBranch, Language::Chinese) => "目标分支",
            (Self::PushTargetHint, Language::English) => {
                "↑↓ remote · Edit branch · Enter publish · Esc back"
            }
            (Self::PushTargetHint, Language::Chinese) => {
                "↑↓ 选择远程 · 输入分支名 · Enter 发布 · Esc 返回"
            }
            (Self::PushForceTitle, Language::English) => "Force push",
            (Self::PushForceTitle, Language::Chinese) => "强制推送",
            (Self::PushForceWarning, Language::English) => {
                "Commits removed from this remote branch"
            }
            (Self::PushForceWarning, Language::Chinese) => "将从此远程分支历史中移除的提交",
            (Self::PushRemoteTip, Language::English) => "Previous remote commit",
            (Self::PushRemoteTip, Language::Chinese) => "原远程提交",
            (Self::PushNewTip, Language::English) => "New remote commit",
            (Self::PushNewTip, Language::Chinese) => "推送后提交",
            (Self::PushRecreateTitle, Language::English) => "Recreate remote branch",
            (Self::PushRecreateTitle, Language::Chinese) => "重新创建远程分支",
            (Self::PushSetUpstream, Language::English) => "Set upstream after a successful push",
            (Self::PushSetUpstream, Language::Chinese) => "推送成功后设置上游",

            (Self::ReplayBatchTitle, Language::English) => "Cherry-pick selected commits",
            (Self::ReplayBatchTitle, Language::Chinese) => "拣选所选提交",
            (Self::ReplayBatchTooLarge, Language::English) => "Select at most 512 commits",
            (Self::ReplayBatchTooLarge, Language::Chinese) => "最多勾选 512 条提交",
            (Self::ReplayDuplicateCommit, Language::English) => {
                "A commit was selected more than once"
            }
            (Self::ReplayDuplicateCommit, Language::Chinese) => "提交被重复选择",
            (Self::ReplayBatchMerge, Language::English) => {
                "Replay merge commits separately and choose their mainline parent"
            }
            (Self::ReplayBatchMerge, Language::Chinese) => "合并提交请单独操作，并选择主线父提交",
            (Self::ReplayOrder, Language::English) => "Execution order (oldest first)",
            (Self::ReplayOrder, Language::Chinese) => "执行顺序（从旧到新）",
            (Self::ReplayQueueRemaining, Language::English) => "Remaining commits",
            (Self::ReplayQueueRemaining, Language::Chinese) => "后续提交",
            (Self::ReplayConflictFiles, Language::English) => "Unresolved files",
            (Self::ReplayConflictFiles, Language::Chinese) => "待解决文件",
            (Self::ReplayReadyContinue, Language::English) => "Conflicts staged; ready to continue",
            (Self::ReplayReadyContinue, Language::Chinese) => "冲突已暂存，可以继续",
            (Self::HistoryMarked, Language::English) => "Selected commits",
            (Self::HistoryMarked, Language::Chinese) => "已选提交",
            (Self::HistorySelect, Language::English) => "Toggle commit selection",
            (Self::HistorySelect, Language::Chinese) => "切换提交选择模式",
            (Self::HistorySelectClear, Language::English) => "Clear selected commits",
            (Self::HistorySelectClear, Language::Chinese) => "清空已选提交",
            (Self::HistorySelectDone, Language::English) => "Exit selection and clear marks",
            (Self::HistorySelectDone, Language::Chinese) => "退出选择并清空勾选",
            (Self::HistorySelectionHint, Language::English) => {
                "↑↓ move · Space mark · Shift+↑↓ range · Enter actions · Esc exit"
            }
            (Self::HistorySelectionHint, Language::Chinese) => {
                "↑↓移动 · Space勾选 · Shift+↑↓范围 · Enter操作 · Esc退出"
            }
            (Self::HistorySelectionRequired, Language::English) => {
                "Mark commits before choosing an action"
            }
            (Self::HistorySelectionRequired, Language::Chinese) => "请先勾选提交，再选择操作",
            (Self::HistorySelectionBack, Language::English) => "Back to selection",
            (Self::HistorySelectionBack, Language::Chinese) => "返回选择",
            (Self::HistoryActionCherryPick, Language::English) => "Cherry-pick selected commits",
            (Self::HistoryActionCherryPick, Language::Chinese) => "拣选所选提交",
            (Self::HistoryActionRevert, Language::English) => "Revert the selected commit",
            (Self::HistoryActionRevert, Language::Chinese) => "撤销所选提交",
            (Self::HistoryActionCopy, Language::English) => "Copy selected commit hashes",
            (Self::HistoryActionCopy, Language::Chinese) => "复制提交哈希",
            (Self::HistoryActionsTitle, Language::English) => "Commit actions",
            (Self::HistoryActionsTitle, Language::Chinese) => "提交操作",
            (Self::HistoryActionsCount, Language::English) => "Selected commits: {count}",
            (Self::HistoryActionsCount, Language::Chinese) => "已选 {count} 条提交",
            (Self::HistoryActionsHint, Language::English) => {
                "↑↓ choose · Enter confirm · Shift+Tab input · Esc back"
            }
            (Self::HistoryActionsHint, Language::Chinese) => {
                "↑↓ 选择 · Enter 确认 · Shift+Tab 输入 · Esc 返回"
            }
            (Self::ReplayBatchAtLeastTwo, Language::English) => {
                "Select at least two commits for a batch"
            }
            (Self::ReplayBatchAtLeastTwo, Language::Chinese) => "批量拣选至少需要两条提交",
            (Self::BranchFilterCommand, Language::English) => {
                "Filter all, local or remote branches"
            }
            (Self::BranchFilterCommand, Language::Chinese) => "筛选全部、本地或远程分支",
            (Self::BranchFilterRequiresView, Language::English) => "Open the branch list first",
            (Self::BranchFilterRequiresView, Language::Chinese) => "请先打开分支列表",
            (Self::HistoryParentCommand, Language::English) => {
                "Switch the comparison to the next merge parent"
            }
            (Self::HistoryParentCommand, Language::Chinese) => "切换到合并提交的下一个比较父提交",
            (Self::HistoryParentRequiresMerge, Language::English) => "Open a merge commit first",
            (Self::HistoryParentRequiresMerge, Language::Chinese) => "请先打开合并提交详情",
            (Self::ReplayControlsTitle, Language::English) => "Recovery actions",
            (Self::ReplayControlsTitle, Language::Chinese) => "操作恢复",
            (Self::ReplayControlsHint, Language::English) => {
                "←→ select · Enter review · Shift+Tab input · Esc back"
            }
            (Self::ReplayControlsHint, Language::Chinese) => {
                "←→选择 · Enter 预览 · Shift+Tab 输入 · Esc 返回"
            }
            (Self::ReplayControlsEntryHint, Language::English) => "Shift+Tab: next page",
            (Self::ReplayControlsEntryHint, Language::Chinese) => "Shift+Tab 切换页",
            (Self::ReplayContinueShort, Language::English) => "Continue",
            (Self::ReplayContinueShort, Language::Chinese) => "继续",
            (Self::ReplaySkipShort, Language::English) => "Skip",
            (Self::ReplaySkipShort, Language::Chinese) => "跳过",
            (Self::ReplayAbortShort, Language::English) => "Abort",
            (Self::ReplayAbortShort, Language::Chinese) => "中止",
            (Self::ReplayConflictsShort, Language::English) => "Conflicts",
            (Self::ReplayConflictsShort, Language::Chinese) => "冲突文件",
            (Self::ReplayRemainingShort, Language::English) => "Next",
            (Self::ReplayRemainingShort, Language::Chinese) => "后续",
            (Self::ReplayCherryPick, Language::English) => "Cherry-pick commit",
            (Self::ReplayPaused, Language::English) => {
                "Operation paused; review task details, then continue, skip or abort"
            }
            (Self::ReplayPaused, Language::Chinese) => "操作已暂停；查看任务详情后继续、跳过或中止",
            (Self::ReplayCherryPick, Language::Chinese) => "拣选提交",
            (Self::ReplayRevert, Language::English) => "Revert commit",
            (Self::ReplayRevert, Language::Chinese) => "撤销提交",
            (Self::ReplayContinue, Language::English) => "Continue operation",
            (Self::ReplayContinue, Language::Chinese) => "继续操作",
            (Self::ReplaySkip, Language::English) => "Skip current commit",
            (Self::ReplaySkip, Language::Chinese) => "跳过当前提交",
            (Self::ReplayAbort, Language::English) => "Abort operation",
            (Self::ReplayAbort, Language::Chinese) => "中止操作",
            (Self::ReplayMetadataTooLarge, Language::English) => {
                "Git operation metadata exceeds 1 MiB"
            }
            (Self::ReplayMetadataTooLarge, Language::Chinese) => "Git 操作元数据超过 1 MiB",
            (Self::ReplayInvalidState, Language::English) => {
                "Git sequencer state is incomplete or inconsistent"
            }
            (Self::ReplayInvalidState, Language::Chinese) => "Git 连续操作状态不完整或不一致",
            (Self::ReplayAttachedRequired, Language::English) => {
                "Switch to an existing local branch first"
            }
            (Self::ReplayAttachedRequired, Language::Chinese) => "请先切换到已有提交的本地分支",
            (Self::ReplayInvalidMessage, Language::English) => "Commit summary is not valid UTF-8",
            (Self::ReplayInvalidMessage, Language::Chinese) => "提交标题不是有效的 UTF-8",
            (Self::ReplayParentRequired, Language::English) => {
                "Explicitly select a mainline parent for this merge commit"
            }
            (Self::ReplayParentRequired, Language::Chinese) => "请明确选择该合并提交的主线父提交",
            (Self::ReplayStats, Language::English) => "{} files · +{} −{}",
            (Self::ReplayStats, Language::Chinese) => "{} 个文件 · +{} −{}",
            (Self::ReplayNotActive, Language::English) => "No cherry-pick or revert is in progress",
            (Self::ReplayNotActive, Language::Chinese) => "当前没有进行中的 cherry-pick 或 revert",
            (Self::ReplayResolveFirst, Language::English) => {
                "Resolve and stage all conflicts before continuing"
            }
            (Self::ReplayResolveFirst, Language::Chinese) => "请先解决并暂存所有冲突，再继续",
            (Self::ReplaySelectCommit, Language::English) => "Select a commit in the history first",
            (Self::ReplaySelectCommit, Language::Chinese) => "请先在提交历史中选择一个提交",
            (Self::ReplayParentsTitle, Language::English) => "Select mainline parent",
            (Self::ReplayParentsTitle, Language::Chinese) => "选择主线父提交",
            (Self::ReplayParentsHint, Language::English) => {
                "↑↓ select parent · Enter review · Esc back"
            }
            (Self::ReplayParentsHint, Language::Chinese) => "↑↓选择父提交 · Enter 预览 · Esc 返回",
            (Self::ReplayPendingHint, Language::English) => {
                "Resolve conflicts in the worktree, then continue, skip or abort."
            }
            (Self::ReplayPendingHint, Language::Chinese) => {
                "请在工作区处理冲突，再选择继续、跳过或中止。"
            }
            (Self::ReplayAbortHint, Language::English) => {
                "Discard this operation and its conflict resolutions; return to its starting state."
            }
            (Self::ReplayAbortHint, Language::Chinese) => {
                "放弃本次操作及冲突处理，回到操作开始前的状态。"
            }
            (Self::ReplaySkipHint, Language::English) => {
                "Skip this commit and discard its conflict resolutions."
            }
            (Self::ReplaySkipHint, Language::Chinese) => "跳过当前提交，放弃它的冲突处理。",
            (Self::ReplayMainlineHelp, Language::English) => {
                "Use the difference from this parent to the merge commit; revert applies its inverse."
            }
            (Self::ReplayMainlineHelp, Language::Chinese) => {
                "以此父提交到合并提交的差异为操作内容；撤销时应用相反的改动。"
            }
            (Self::ReplayConflictWorktree, Language::English) => "Conflicted worktree",
            (Self::ReplayConflictWorktree, Language::Chinese) => "冲突工作区",
            (Self::ReplayMainline, Language::English) => "Mainline parent",
            (Self::ReplayMainline, Language::Chinese) => "主线父提交",
            (Self::ReplaySource, Language::English) => "Source commit",
            (Self::ReplaySource, Language::Chinese) => "来源提交",
            (Self::ReplayTargetBranch, Language::English) => "Target branch",
            (Self::ReplayTargetBranch, Language::Chinese) => "目标分支",
            (Self::ReplayActive, Language::English) => "In progress",
            (Self::ReplayActive, Language::Chinese) => "进行中",

            (Self::ComposeTooLarge, Language::English) => {
                "Composition exceeds 512 units or 128 KiB of patch context; commit part of the changes first."
            }
            (Self::ComposeTooLarge, Language::Chinese) => {
                "编排超过 512 个改动块或 128 KiB 差异，请先提交部分改动。"
            }
            (Self::ComposeFileTooLarge, Language::English) => {
                "Unstaged files over 16 MiB cannot be captured for composition"
            }
            (Self::ComposeFileTooLarge, Language::Chinese) => {
                "编排不能读取超过 16 MiB 的未暂存文件"
            }

            (Self::ExperimentalFeatures, Language::English) => "Experimental features",
            (Self::ExperimentalFeatures, Language::Chinese) => "实验性功能",
            (Self::ExperimentalEnabled, Language::English) => "Enable experimental features",
            (Self::ExperimentalEnabled, Language::Chinese) => "启用实验性功能",
            (Self::ExperimentalHint, Language::English) => {
                "A distinct text color marks experimental features, including AI commit composition."
            }
            (Self::ExperimentalHint, Language::Chinese) => {
                "特殊文字颜色标识实验性功能，当前包含 AI 编排提交。"
            }
            (Self::ExperimentalDisabled, Language::English) => {
                "Experimental features are disabled. Enable them in settings to use commit composition."
            }
            (Self::ExperimentalDisabled, Language::Chinese) => {
                "实验性功能已关闭，请在设置中启用后使用编排提交。"
            }
            (Self::ComposeTitle, Language::English) => "AI commit composition",
            (Self::ComposeTitle, Language::Chinese) => "AI 编排提交",
            (Self::ComposeCommand, Language::English) => "Compose commits with AI",
            (Self::ComposeCommand, Language::Chinese) => "AI 编排提交",
            (Self::ComposeInvalid, Language::English) => "Invalid commit plan.",
            (Self::ComposeInvalid, Language::Chinese) => "提交计划格式无效。",
            (Self::ComposeCoverage, Language::English) => {
                "Every change unit must be assigned exactly once; the final tree must match the captured changes."
            }
            (Self::ComposeCoverage, Language::Chinese) => {
                "每个改动块必须恰好分配一次，最终内容必须与快照一致。"
            }
            (Self::ComposeDependency, Language::English) => {
                "Change units cannot be applied in this order; group dependent changes together or reorder the commits"
            }
            (Self::ComposeDependency, Language::Chinese) => {
                "改动块无法按此顺序应用，请合并相关改动或调整提交顺序"
            }
            (Self::ComposeNoChanges, Language::English) => "No committable changes.",
            (Self::ComposeNoChanges, Language::Chinese) => "没有可提交的改动。",
            (Self::ComposeSubmodule, Language::English) => {
                "Untracked nested repositories cannot be composed."
            }
            (Self::ComposeSubmodule, Language::Chinese) => "未跟踪的嵌套仓库不能参与编排。",
            (Self::ComposeChanged, Language::English) => {
                "HEAD, index or working-tree contents changed. Generate a new plan."
            }
            (Self::ComposeChanged, Language::Chinese) => {
                "HEAD、暂存区或工作区内容已变化，请重新生成编排。"
            }
            (Self::ComposeEmptyStep, Language::English) => "A planned commit contains no changes.",
            (Self::ComposeEmptyStep, Language::Chinese) => "计划中存在没有实际改动的提交。",
            (Self::ComposeIndexLocked, Language::English) => "Cannot lock the repository index",
            (Self::ComposeIndexLocked, Language::Chinese) => "无法锁定仓库索引",
            (Self::ComposeCommitMissing, Language::English) => {
                "Git completed without creating a commit."
            }
            (Self::ComposeCommitMissing, Language::Chinese) => "Git 未创建提交。",
            (Self::ComposeCommitStopped, Language::English) => {
                "Composition stopped; inspect the repository and this commit before continuing"
            }
            (Self::ComposeCommitStopped, Language::Chinese) => {
                "编排已停止，请检查仓库和此提交后再继续"
            }
            (Self::ComposeHookConflict, Language::English) => {
                "A commit was created but hooks changed its contents; the original index was retained. Review staged changes manually. Commit"
            }
            (Self::ComposeHookConflict, Language::Chinese) => {
                "提交已创建，但钩子改动无法无冲突地合并到原暂存区；原索引已保留，请手动核对。提交"
            }
            (Self::ComposeHookChanged, Language::English) => {
                "Hooks changed the committed contents; the index was reconciled and composition stopped. Commit"
            }
            (Self::ComposeHookChanged, Language::Chinese) => {
                "钩子修改了提交内容，已同步索引并停止编排。提交"
            }
            (Self::ComposeGenerating, Language::English) => "Generating commit plan",
            (Self::ComposeGenerating, Language::Chinese) => "正在生成提交编排",
            (Self::ComposeNext, Language::English) => "Commit next",
            (Self::ComposeNext, Language::Chinese) => "提交下一项",
            (Self::ComposeDone, Language::English) => "Commit composition completed",
            (Self::ComposeDone, Language::Chinese) => "编排提交已完成",
            (Self::ComposeHint, Language::English) => {
                "↑↓ select · /focus pane · Enter edit message · / commands · Esc back"
            }
            (Self::ComposeHint, Language::Chinese) => {
                "↑↓ 选择 · /focus 面板 · Enter 编辑说明 · / 命令 · Esc 返回"
            }
            (Self::ComposeEditHint, Language::English) => {
                "Enter newline · Ctrl+S save · Esc discard"
            }
            (Self::ComposeEditHint, Language::Chinese) => "Enter 换行 · Ctrl+S 保存 · Esc 放弃",
            (Self::CommandEnterParameters, Language::English) => "Enter choose a parameter",
            (Self::CommandEnterParameters, Language::Chinese) => "Enter 选择参数",
            (Self::CommandEnterArgument, Language::English) => "Enter edit parameters",
            (Self::CommandEnterArgument, Language::Chinese) => "Enter 输入参数",
            (Self::CommandEnterRequired, Language::English) => "Type the required parameter",
            (Self::CommandEnterRequired, Language::Chinese) => "请输入必填参数",
            (Self::CommandEnterReview, Language::English) => "Enter validate and preview",
            (Self::CommandEnterReview, Language::Chinese) => "Enter 校验并预览",
            (Self::CommandEnterRun, Language::English) => "Enter run",
            (Self::CommandEnterRun, Language::Chinese) => "Enter 执行",
            (Self::CommandEnterApply, Language::English) => "Enter apply",
            (Self::CommandEnterApply, Language::Chinese) => "Enter 应用",
            (Self::CommandEnterOpen, Language::English) => "Enter open",
            (Self::CommandEnterOpen, Language::Chinese) => "Enter 打开",
            (Self::CommandEnterFillCommand, Language::English) => "Enter fill command",
            (Self::CommandEnterFillCommand, Language::Chinese) => "Enter 填入命令",
            (Self::CommandEnterFillPath, Language::English) => "Enter fill path",
            (Self::CommandEnterFillPath, Language::Chinese) => "Enter 填入路径",
            (Self::CommandEnterFillValue, Language::English) => "Enter use selected value",
            (Self::CommandEnterFillValue, Language::Chinese) => "Enter 使用选中值",
            (Self::CommandEnterWaiting, Language::English) => "Loading candidates",
            (Self::CommandEnterWaiting, Language::Chinese) => "正在加载候选",
            (Self::CommandScopedCandidateKeys, Language::English) => "/ global commands · Tab complete · ⌫ edit prefix",
            (Self::CommandScopedCandidateKeys, Language::Chinese) => "/ 全局命令 · Tab 补全 · ⌫ 编辑前缀",
            (Self::CommandCandidateKeys, Language::English) => "Tab complete · ↑↓ choose · Esc back",
            (Self::CommandCandidateKeys, Language::Chinese) => "Tab 补全 · ↑↓选择 · Esc 返回",
            (Self::CommandCandidateHistoryKeys, Language::English) => "Tab complete · ↓ choose · ↑ history · Esc close",
            (Self::CommandCandidateHistoryKeys, Language::Chinese) => "Tab 补全 · ↓候选 · ↑历史 · Esc 关闭",
            (Self::OptionCompletionHint, Language::English) => {
                "↑↓ choose · Tab complete · Enter apply · Esc close options"
            }
            (Self::OptionCompletionHint, Language::Chinese) => {
                "↑↓ 选择 · Tab 补全 · Enter 应用 · Esc 收起选项"
            }
            (Self::UnknownOptionValue, Language::English) => {
                "Unknown value: {value}. Allowed values: {choices}"
            }
            (Self::UnknownOptionValue, Language::Chinese) => "未知参数：{value}。可选值：{choices}",
            (Self::NoMatchingOptions, Language::English) => "No matching option is available",
            (Self::ReadingCommandCandidates, Language::English) => {
                "Reading candidates · Esc cancel"
            }
            (Self::ReadingCommandCandidates, Language::Chinese) => "正在读取候选 · Esc 取消",
            (Self::ValueCompletionHint, Language::English) => {
                "↑↓ choose · Tab complete · Enter review · Esc close"
            }
            (Self::ValueCompletionHint, Language::Chinese) => {
                "↑↓ 选择 · Tab 补全 · Enter 预览 · Esc 收起"
            }
            (Self::CommandRequiresHistory, Language::English) => "Open commit history first",
            (Self::CommandRequiresHistory, Language::Chinese) => "请先打开提交历史",
            (Self::CommandRequiresBrowser, Language::English) => "Open the file browser first",
            (Self::CommandRequiresBrowser, Language::Chinese) => "请先打开文件浏览",
            (Self::CommandNeedsUnstaged, Language::English) => "Select unstaged changes",
            (Self::CommandNeedsUnstaged, Language::Chinese) => "请选择未暂存改动",
            (Self::CommandNeedsStaged, Language::English) => "Select staged changes",
            (Self::CommandNeedsStaged, Language::Chinese) => "请选择已暂存改动",
            (Self::CommandReplayStateUnknown, Language::English) => {
                "Open the repository to read operation state"
            }
            (Self::CommandReplayStateUnknown, Language::Chinese) => "请先打开仓库以读取操作状态",
            (Self::CommandReplayInProgress, Language::English) => {
                "Finish or abort the current Git operation first"
            }
            (Self::CommandReplayInProgress, Language::Chinese) => "请先完成或中止当前 Git 操作",
            (Self::CommandSourceCurrent, Language::English) => "Source view is already active",
            (Self::CommandSourceCurrent, Language::Chinese) => "当前已是源文件视图",
            (Self::CommandPreviewCurrent, Language::English) => "Preview is already active",
            (Self::CommandPreviewCurrent, Language::Chinese) => "当前已是预览视图",
            (Self::CommandRequiresPreviewableFile, Language::English) => {
                "Select a Markdown or SVG file first"
            }
            (Self::CommandRequiresPreviewableFile, Language::Chinese) => {
                "请先选择 Markdown 或 SVG 文件"
            }
            (Self::CommandTargetFiles, Language::English) => "{count} files",
            (Self::CommandTargetFiles, Language::Chinese) => "{count} 个文件",
            (Self::CommandTargetLines, Language::English) => "{count} lines",
            (Self::CommandTargetLines, Language::Chinese) => "{count} 行",
            (Self::CommandTargetCommits, Language::English) => "{count} commits",
            (Self::CommandTargetCommits, Language::Chinese) => "{count} 个提交",
            (Self::NoMatchingOptions, Language::Chinese) => "没有匹配的可用选项",
            (Self::ComposeWaitingHint, Language::English) => "Esc cancel and return · / commands",
            (Self::ComposeWaitingHint, Language::Chinese) => "Esc 取消并返回 · / 命令",
            (Self::ComposeRetryHint, Language::English) => {
                "Enter retry · PgUp/PgDn scroll · Esc back · / commands"
            }
            (Self::ComposeRetryHint, Language::Chinese) => {
                "Enter 重试 · PgUp/PgDn 滚动 · Esc 返回 · / 命令"
            }
            (Self::ComposeGenerationFailed, Language::English) => "Commit plan generation failed",
            (Self::ComposeGenerationFailed, Language::Chinese) => "提交编排生成失败",
            (Self::ComposeEdit, Language::English) => "Edit selected commit message",
            (Self::ComposeEdit, Language::Chinese) => "编辑所选提交的说明",
            (Self::ComposeUp, Language::English) => "Move selected commit earlier",
            (Self::ComposeUp, Language::Chinese) => "将所选提交前移",
            (Self::ComposeDown, Language::English) => "Move selected commit later",
            (Self::ComposeDown, Language::Chinese) => "将所选提交后移",
            (Self::ComposeSplit, Language::English) => {
                "Move selected change unit into a new commit"
            }
            (Self::ComposeSplit, Language::Chinese) => "将所选改动块拆成新提交",
            (Self::ComposeMerge, Language::English) => {
                "Merge selected commit with the following commit"
            }
            (Self::ComposeMerge, Language::Chinese) => "将所选提交与下一项合并",
            (Self::ComposeMove, Language::English) => "Move selected change unit to another commit",
            (Self::ComposeMove, Language::Chinese) => "将所选改动块移到其它提交",
            (Self::ComposeMoveHint, Language::English) => {
                "↑↓ target commit · Enter move · Esc back"
            }
            (Self::ComposeMoveHint, Language::Chinese) => "↑↓ 目标提交 · Enter 移动 · Esc 返回",
            (Self::ComposeRegenerate, Language::English) => {
                "Generate a new plan from current repository contents"
            }
            (Self::ComposeRegenerate, Language::Chinese) => "根据仓库当前内容重新生成编排",
            (Self::ComposeSelected, Language::English) => {
                "Select a planned commit or change unit first."
            }
            (Self::ComposeSelected, Language::Chinese) => "请先选中计划中的提交或改动块。",
            (Self::ComposePending, Language::English) => "Remaining commits",
            (Self::ComposePending, Language::Chinese) => "待提交",
            (Self::ComposeUnits, Language::English) => "Change units",
            (Self::ComposeUnits, Language::Chinese) => "改动块",
            (Self::ComposeCompleted, Language::English) => "Completed commits",
            (Self::ComposeCompleted, Language::Chinese) => "已提交",

            (Self::PaneReadHint, Language::English) => "Enter reload · ←/Esc back · / commands",
            (Self::PaneReadHint, Language::Chinese) => "Enter重新读取 · ←/Esc返回 · /命令",
            (Self::BrowserPreviewHint, Language::English) => {
                "↑↓ scroll · ←→ pan · /focus panes · Esc files · / commands"
            }
            (Self::BrowserPreviewHint, Language::Chinese) => {
                "↑↓滚动 · ←→平移 · /focus面板 · Esc文件 · /命令"
            }
            (Self::DiffPreviewInputHint, Language::English) => {
                "↑↓ scroll · ←→ pan · /focus files · Esc back · / commands"
            }
            (Self::DiffPreviewInputHint, Language::Chinese) => {
                "↑↓滚动 · ←→平移 · /focus文件 · Esc返回 · /命令"
            }
            (Self::AgentStreamIncomplete, Language::English) => {
                "Agent stream ended without a completed message."
            }
            (Self::AgentStreamIncomplete, Language::Chinese) => "生成流在消息完成前结束。",
            (Self::AgentStreamUnexpectedRequest, Language::English) => {
                "Unexpected agent request; generation cannot request tools or approval."
            }
            (Self::AgentStreamUnexpectedRequest, Language::Chinese) => {
                "生成工具发出了不支持的请求；消息生成不调用工具或请求审批。"
            }
            (Self::CommitGeneratingHint, Language::English) => {
                "Read only · Enter / Ctrl+R / Esc cancel"
            }
            (Self::CommitGeneratingHint, Language::Chinese) => {
                "说明只读 · Enter / Ctrl+R / Esc 取消生成"
            }
            (Self::CommitEditingHint, Language::English) => {
                "Enter newline · Shift+Tab options · Ctrl+S commit · Esc back"
            }
            (Self::CommitEditingHint, Language::Chinese) => {
                "Enter 换行 · Shift+Tab 选项 · Ctrl+S 提交 · Esc 返回"
            }
            (Self::CommitActionsHint, Language::English) => {
                "←→ select · Enter confirm · Shift+Tab input · Esc back"
            }
            (Self::CommitActionsHint, Language::Chinese) => {
                "←→ 选择 · Enter 确认 · Shift+Tab 输入 · Esc 返回"
            }
            (Self::CommitRegenerateAction, Language::English) => "Regenerate",
            (Self::CommitRegenerateAction, Language::Chinese) => "重新生成",
            (Self::CommitSubmitting, Language::English) => "Committing",
            (Self::CommitSubmitting, Language::Chinese) => "正在提交",
            (Self::CommitSubmittingHint, Language::English) => {
                "Committing in background · Enter / Esc back"
            }
            (Self::CommitSubmittingHint, Language::Chinese) => "正在后台提交 · Enter / Esc 返回",
            (Self::CommitCancelAction, Language::English) => "Cancel generation",
            (Self::CommitCancelAction, Language::Chinese) => "取消生成",
            (Self::CommitReturnAction, Language::English) => "Back",
            (Self::CommitReturnAction, Language::Chinese) => "返回",
            (Self::SearchInputTitle, Language::English) => "{mode} {target}",
            (Self::SearchInputTitle, Language::Chinese) => "{mode}{target}",
            (Self::SearchModeCommand, Language::English) => "Choose filter or locate for this view",
            (Self::SearchModeCommand, Language::Chinese) => "选择当前视图的过滤或定位方式",
            (Self::SearchFilterCommand, Language::English) => "Filter the current list by input",
            (Self::SearchFilterCommand, Language::Chinese) => "输入时只显示列表中的匹配项",
            (Self::SearchLocateCommand, Language::English) => {
                "Keep all content; ↑/↓ move between matches"
            }
            (Self::SearchLocateCommand, Language::Chinese) => "保留完整内容，↑/↓切换匹配项",
            (Self::SearchFilter, Language::English) => "Filter",
            (Self::SearchFilter, Language::Chinese) => "过滤",
            (Self::SearchLocate, Language::English) => "Locate",
            (Self::SearchLocate, Language::Chinese) => "定位",
            (Self::SearchLocateHint, Language::English) => {
                "↑/↓ previous/next match · ←/→ edit · Esc clear · / commands"
            }
            (Self::SearchLocateHint, Language::Chinese) => {
                "↑/↓上个/下个匹配 · ←/→编辑 · Esc清空 · /命令"
            }
            (Self::SearchCommandOnly, Language::English) => "Press / to open the command menu",
            (Self::SearchCommandOnly, Language::Chinese) => "按 / 打开命令菜单",
            (Self::SearchUnavailable, Language::English) => {
                "This view does not support that search mode."
            }
            (Self::SearchUnavailable, Language::Chinese) => "当前视图不支持该搜索方式。",
            (Self::SearchTargetRepositories, Language::English) => "repositories",
            (Self::SearchTargetRepositories, Language::Chinese) => "仓库",
            (Self::SearchTargetFiles, Language::English) => "files",
            (Self::SearchTargetFiles, Language::Chinese) => "文件",
            (Self::SearchTargetBranches, Language::English) => "branches",
            (Self::SearchTargetBranches, Language::Chinese) => "分支",
            (Self::SearchTargetStashes, Language::English) => "stashes",
            (Self::SearchTargetStashes, Language::Chinese) => "贮藏",
            (Self::SearchTargetCommits, Language::English) => "loaded commits",
            (Self::SearchTargetCommits, Language::Chinese) => "已加载提交",

            (Self::PaneFocusTitle, Language::English) => "Focus mode",
            (Self::PaneFocusTitle, Language::Chinese) => "聚焦模式",
            (Self::PaneFocusHint, Language::English) => {
                "Arrows focus · Enter/Esc finish · / commands"
            }
            (Self::PaneFocusHint, Language::Chinese) => "方向键聚焦 · Enter/Esc结束 · /命令",

            (Self::PaneFocusCommand, Language::English) => "Enter pane focus mode",
            (Self::PaneFocusCommand, Language::Chinese) => "进入面板聚焦模式",
            (Self::PaneZoomCommand, Language::English) => "Maximize or restore focused pane",
            (Self::PaneZoomCommand, Language::Chinese) => "放大或还原当前面板",
            (Self::PaneUnavailable, Language::English) => {
                "This view has no applicable pane operation."
            }
            (Self::PaneUnavailable, Language::Chinese) => "当前视图没有可执行的面板操作。",

            (Self::TasksCheck, Language::English) => "Fetch",
            (Self::TasksCheck, Language::Chinese) => "获取远程",
            (Self::TasksScheduledCheck, Language::English) => "Scheduled check",
            (Self::TasksScheduledCheck, Language::Chinese) => "定时检查",
            (Self::TaskInterrupted, Language::English) => {
                "Task interrupted: its worker is no longer running. Review the repository before retrying."
            }
            (Self::TaskInterrupted, Language::Chinese) => {
                "任务中断：执行进程已退出。重试前请检查仓库状态。"
            }
            (Self::Tasks, Language::English) => "Tasks",
            (Self::Tasks, Language::Chinese) => "任务",
            (Self::TasksHint, Language::English) => {
                "↑↓ select · Enter details · c cancel · r retry · d mark handled · Esc return"
            }
            (Self::TasksHint, Language::Chinese) => {
                "↑↓选择 · Enter 详情 · c 取消 · r 重试 · d 标记已处理 · Esc 返回"
            }
            (Self::TasksEmpty, Language::English) => "No tasks yet",
            (Self::TasksEmpty, Language::Chinese) => "暂无任务",
            (Self::TasksPending, Language::English) => "Unresolved failure",
            (Self::TasksPending, Language::Chinese) => "未处理失败",
            (Self::TasksEntrySuffix, Language::English) => "",
            (Self::TasksEntrySuffix, Language::Chinese) => " 任务",
            (Self::TasksOpenCommand, Language::English) => "/tasks",
            (Self::TasksOpenCommand, Language::Chinese) => "/tasks 任务",
            (Self::TasksRunning, Language::English) => "Running",
            (Self::TasksRunning, Language::Chinese) => "运行中",
            (Self::TasksCancelling, Language::English) => "Cancelling",
            (Self::TasksCancelling, Language::Chinese) => "正在取消",
            (Self::TasksSuccess, Language::English) => "Succeeded",
            (Self::TasksSuccess, Language::Chinese) => "成功",
            (Self::TasksFailedHandled, Language::English) => "Failed · handled",
            (Self::TasksFailedHandled, Language::Chinese) => "失败 · 已处理",
            (Self::TasksCancelled, Language::English) => "Cancelled",
            (Self::TasksCancelled, Language::Chinese) => "已取消",
            (Self::TasksHistory, Language::English) => "History",
            (Self::TasksHistory, Language::Chinese) => "历史",
            (Self::TasksUnavailable, Language::English) => {
                "This task cannot be retried here. Review its details and run the operation again."
            }
            (Self::TasksUnavailable, Language::Chinese) => {
                "此任务不支持直接重试，请查看详情后重新执行操作。"
            }
            (Self::TasksSummary, Language::English) => {
                "Unresolved failures: {pending} · Running: {running} · /tasks"
            }
            (Self::TasksSummary, Language::Chinese) => {
                "{pending} 未处理失败 · {running} 运行中 · /tasks 任务"
            }
            (Self::TasksDetailsHint, Language::English) => {
                "↑↓ scroll · c cancel · r retry · d mark handled · Esc return"
            }
            (Self::TasksDetailsHint, Language::Chinese) => {
                "↑↓滚动 · c 取消 · r 重试 · d 标记已处理 · Esc 返回"
            }
            (Self::TasksInteractive, Language::English) => "Interactive",
            (Self::TasksInteractive, Language::Chinese) => "手动",
            (Self::TasksScheduled, Language::English) => "Scheduled",
            (Self::TasksScheduled, Language::Chinese) => "定时",
            (Self::TasksImported, Language::English) => "Previous history",
            (Self::TasksImported, Language::Chinese) => "此前记录",
            (Self::TasksStarted, Language::English) => "Started",
            (Self::TasksStarted, Language::Chinese) => "开始",
            (Self::TasksFinished, Language::English) => "Finished",
            (Self::TasksFinished, Language::Chinese) => "结束",
            (Self::TasksDetails, Language::English) => "Task details",
            (Self::TasksDetails, Language::Chinese) => "任务详情",
            (Self::TasksFailureReason, Language::English) => "Failure reason",
            (Self::TasksFailureReason, Language::Chinese) => "失败原因",
            (Self::TasksProgress, Language::English) => "Progress",
            (Self::TasksProgress, Language::Chinese) => "进度",
            (Self::TasksSource, Language::English) => "Source",
            (Self::TasksSource, Language::Chinese) => "来源",
            (Self::TasksScope, Language::English) => "Scope",
            (Self::TasksScope, Language::Chinese) => "范围",
            (Self::TasksAll, Language::English) => "All repositories",
            (Self::TasksAll, Language::Chinese) => "全部仓库",
            (Self::TasksApplication, Language::English) => "Application",
            (Self::TasksApplication, Language::Chinese) => "应用",
            (Self::TasksStorageError, Language::English) => "Task history error",
            (Self::TasksStorageError, Language::Chinese) => "任务记录错误",

            (Self::ExpandOperationResult, Language::English) => "Show full result",
            (Self::ExpandOperationResult, Language::Chinese) => "展开完整结果",
            (Self::OperationResultReaderHint, Language::English) => {
                "↑↓ / PgUp / PgDn scroll · Esc / Enter return"
            }
            (Self::OperationResultReaderHint, Language::Chinese) => {
                "↑↓ / PgUp / PgDn 滚动 · Esc / Enter 返回"
            }
            (Self::AgentPromptConflict, Language::English) => {
                "Configuration contains both legacy instructions and a replacement prompt; keep only one."
            }
            (Self::AgentPromptConflict, Language::Chinese) => {
                "配置同时包含旧补充要求和新生成指令，请保留一种。"
            }
            (Self::AgentDefaultPrompt, Language::English) => "Default prompt",
            (Self::AgentDefaultPrompt, Language::Chinese) => "默认指令",
            (Self::PreferenceSaved, Language::English) => "Settings saved.",
            (Self::PreferenceSaved, Language::Chinese) => "设置已保存。",
            (Self::Ai, Language::English | Language::Chinese) => "AI",
            (Self::AiHelp, Language::English) => include_str!("help/ai.en.md"),
            (Self::AiHelp, Language::Chinese) => include_str!("help/ai.zh.md"),
            (Self::AiSettings, Language::English) => "AI settings",
            (Self::AiSettings, Language::Chinese) => "AI 设置",
            (Self::AiEnabled, Language::English) => "Enable AI",
            (Self::AiEnabled, Language::Chinese) => "启用 AI",
            (Self::AiEnabledHint, Language::English) => {
                "Turn off to hide AI features. Change this switch here; commands cannot enable AI."
            }
            (Self::AiEnabledHint, Language::Chinese) => {
                "关闭后隐藏相关命令和功能，只能在此重新启用。"
            }
            (Self::AiDisabled, Language::English) => {
                "AI is disabled. Enable it on the AI settings page."
            }
            (Self::AiDisabled, Language::Chinese) => "AI 已禁用，请在 AI 设置页启用。",
            (Self::AgentNotFound, Language::English) => {
                "Agent not found in PATH; install and log in to the selected CLI"
            }
            (Self::AgentNotFound, Language::Chinese) => {
                "未在 PATH 中找到生成工具，请先安装并登录所选 CLI"
            }
            (Self::AgentSettings, Language::English) => "Commit message generation",
            (Self::AgentSettings, Language::Chinese) => "提交消息生成",
            (Self::AgentChoice, Language::English) => "Agent",
            (Self::AgentNameHint, Language::English) => {
                "Enter an agent name; Tab completes supported values."
            }
            (Self::AgentNameHint, Language::Chinese) => "填写工具名称，Tab 补全可用参数值。",
            (Self::UnknownAgent, Language::English) => "Unknown agent",
            (Self::UnknownAgent, Language::Chinese) => "未知工具",
            (Self::SupportedAgents, Language::English) => "Supported agents",
            (Self::SupportedAgents, Language::Chinese) => "支持的工具",
            (Self::ArgumentCandidates, Language::English) => "Argument values",
            (Self::ArgumentCandidates, Language::Chinese) => "参数候选",
            (Self::AgentChoice, Language::Chinese) => "生成工具",
            (Self::AgentChoiceHint, Language::English) => {
                "Use an installed CLI with its existing login."
            }
            (Self::AgentChoiceHint, Language::Chinese) => "调用本机 CLI，沿用其登录状态。",
            (Self::AgentModel, Language::English) => "Model",
            (Self::AgentModel, Language::Chinese) => "模型",
            (Self::AgentModelHint, Language::English) => {
                "Enter a model name, or default to use the agent's configured model."
            }
            (Self::AgentModelHint, Language::Chinese) => {
                "填写模型名；default 表示沿用 agent 自己的模型。"
            }
            (Self::AgentPrompt, Language::English) => "Prompt",
            (Self::AgentCustomPrompt, Language::English) => "Custom prompt · Enter to edit",
            (Self::AgentCustomPrompt, Language::Chinese) => "自定义指令 · Enter 编辑",
            (Self::AgentPrompt, Language::Chinese) => "生成指令",
            (Self::AgentPromptEditorHint, Language::English) => "Ctrl+S save · Esc cancel",
            (Self::AgentPromptEditorHint, Language::Chinese) => "Ctrl+S 保存 · Esc 放弃修改",
            (Self::AgentPromptHint, Language::English) => "Edit the commit message prompt.",
            (Self::AgentPromptHint, Language::Chinese) => "编辑提交消息的生成指令。",
            (Self::AgentDefault, Language::English | Language::Chinese) => "Default",
            (Self::AgentNone, Language::English) => "None",
            (Self::AgentNone, Language::Chinese) => "无",
            (Self::AgentSaved, Language::English) => "AI settings saved.",
            (Self::AgentSaved, Language::Chinese) => "AI 设置已保存。",
            (Self::AgentGenerate, Language::English) => {
                "Generate a commit message from staged changes with AI"
            }
            (Self::AgentGenerate, Language::Chinese) => "AI 根据暂存改动生成提交说明",
            (Self::AgentGenerating, Language::English) => "Generating commit message",
            (Self::AgentGenerating, Language::Chinese) => "正在生成提交消息",
            (Self::AgentGenerated, Language::English) => {
                "Commit message generated; edit and confirm before committing."
            }
            (Self::AgentGenerated, Language::Chinese) => "已生成提交消息，可编辑后确认提交。",
            (Self::AgentDraftChanged, Language::English) => {
                "Draft changed; generated text was not applied."
            }
            (Self::AgentDraftChanged, Language::Chinese) => "草稿已更改，未覆盖为生成结果。",
            (Self::AgentCancelled, Language::English) => "Generation cancelled; draft retained.",
            (Self::AgentCancelled, Language::Chinese) => "已取消生成，保留原草稿。",
            (Self::AgentHint, Language::English) => {
                "Enter ↵ · Shift+Tab options · Ctrl+S commit · Ctrl+R redo · Esc"
            }
            (Self::AgentHint, Language::Chinese) => {
                "Enter 换行 · Shift+Tab 选项 · Ctrl+S 提交 · Ctrl+R 生成 · Esc"
            }
            (Self::AgentRunningHint, Language::English) => {
                "Generating… Ctrl+R cancel · Ctrl+J newline · Enter review · Esc return"
            }
            (Self::AgentRunningHint, Language::Chinese) => {
                "生成中… Ctrl+R 取消 · Ctrl+J 换行 · Enter 确认 · Esc 返回"
            }
            (Self::AgentModelInvalid, Language::English) => {
                "Model must have no control characters and be at most 256 bytes."
            }
            (Self::AgentModelInvalid, Language::Chinese) => {
                "模型名不能包含控制字符，且最多 256 字节。"
            }
            (Self::AgentPromptInvalid, Language::English) => {
                "Prompt must be nonempty, contain no NUL and be at most 16384 bytes."
            }
            (Self::AgentPromptInvalid, Language::Chinese) => {
                "生成指令不能为空、不能包含 NUL，且最多 16384 字节。"
            }
            (Self::AgentDiffTooLarge, Language::English) => {
                "Staged patch exceeds 128 KiB. Stage a smaller change before generating a message."
            }
            (Self::AgentDiffTooLarge, Language::Chinese) => {
                "暂存差异超过 128 KiB，请缩小本次暂存范围后生成消息。"
            }
            (Self::AgentOutputTooLarge, Language::English) => {
                "Agent output exceeded the allowed size."
            }
            (Self::AgentOutputTooLarge, Language::Chinese) => "生成工具的输出超过大小限制。",
            (Self::AgentStructuredOutputMissing, Language::English) => {
                "The agent did not return a structured JSON result. Check that its version and model support JSON Schema."
            }
            (Self::AgentStructuredOutputMissing, Language::Chinese) => {
                "生成工具未返回结构化 JSON 结果，请确认其版本和模型支持 JSON Schema。"
            }
            (Self::AgentMessageInvalid, Language::English) => {
                "Expected a nonempty plain-text commit message (up to 64 KiB), without code fences or control characters other than newlines and tabs."
            }
            (Self::AgentMessageInvalid, Language::Chinese) => {
                "提交消息必须为非空纯文本，最多 64 KiB；不能有代码围栏及换行、制表以外的控制字符。"
            }
            (Self::SettingsChecks, Language::English) => "Background operations",
            (Self::SettingsChecks, Language::Chinese) => "后台操作",
            (Self::SettingsInput, Language::English) => "Input",
            (Self::SettingsInput, Language::Chinese) => "输入",
            (Self::SettingsConcurrency, Language::English) => "Concurrent repositories",
            (Self::SettingsConcurrency, Language::Chinese) => "并发仓库数",
            (Self::SettingsConcurrencyHint, Language::English) => {
                "Maximum repositories checked at once · 1–32"
            }
            (Self::SettingsConcurrencyHint, Language::Chinese) => "同时检查的仓库数量，上限 32",
            (Self::SettingsChineseTrigger, Language::English) => "Chinese command trigger",
            (Self::SettingsChineseTrigger, Language::Chinese) => "中文命令触发",
            (Self::SettingsChineseTriggerHint, Language::English) => {
                "Type 、 to open the command menu"
            }
            (Self::SettingsChineseTriggerHint, Language::Chinese) => "输入 、 打开命令菜单",
            (Self::SettingsPreciseClick, Language::English) => "Precise clicks",
            (Self::SettingsPreciseClick, Language::Chinese) => "精确点击",
            (Self::SettingsPreciseClickHint, Language::English) => {
                "Select items and seek scrollbars; double-click to enter"
            }
            (Self::SettingsPreciseClickHint, Language::Chinese) => {
                "点击选择条目和滚动条定位，双击进入"
            }
            (Self::SettingsScrollHint, Language::English) => {
                "Scroll lists and content with the wheel or trackpad"
            }
            (Self::SettingsScrollHint, Language::Chinese) => "使用鼠标滚轮或触控板滚动列表与内容",
            (Self::SettingsSelectPane, Language::English) => "Pane selection",
            (Self::SettingsSelectPane, Language::Chinese) => "选择子视图",
            (Self::SettingsSelectPaneHint, Language::English) => {
                "Focus the pane under the pointer when clicking or scrolling"
            }
            (Self::SettingsSelectPaneHint, Language::Chinese) => "点击或滚动时聚焦指针所在子视图",

            (Self::MutationStashConflicts, Language::English) => {
                "Stash restored with conflicts; the stash was retained. Resolve conflicts before continuing."
            }
            (Self::MutationStashConflicts, Language::Chinese) => {
                "恢复 stash 时产生冲突，原 stash 已保留。请先解决冲突。"
            }
            (Self::BranchCreateCommand, Language::English) => {
                "Create a local branch at the current commit"
            }
            (Self::BranchCreateCommand, Language::Chinese) => "在当前提交创建本地分支",
            (Self::BranchSwitchCommand, Language::English) => {
                "Switch to an existing local branch after review"
            }
            (Self::BranchSwitchCommand, Language::Chinese) => "预览后切换到已有本地分支",
            (Self::AutoFetch, Language::English) => "Automatic fetch",
            (Self::AutoFetch, Language::Chinese) => "自动获取远程",
            (Self::AutoFetchHint, Language::English) => {
                "Periodically fetch while the window is focused and repositories are idle"
            }
            (Self::AutoFetchHint, Language::Chinese) => "窗口聚焦且仓库空闲时，定期获取远程信息",
            (Self::AutoFetchInterval, Language::English) => "Automatic fetch interval",
            (Self::AutoFetchInterval, Language::Chinese) => "自动获取间隔",
            (Self::AutoFetchIntervalHint, Language::English) => {
                "Wait after each fetch completes; e.g. 3m or 1h (at least 60s)"
            }
            (Self::AutoFetchIntervalHint, Language::Chinese) => {
                "从获取完成后计算，例如 3m、1h，至少 60s"
            }
            (Self::AutoFetchEnabled, Language::English) => "Enable periodic fetch in this TUI",
            (Self::AutoFetchEnabled, Language::Chinese) => "启用界面内的周期获取",
            (Self::AutoFetchDisabled, Language::English) => "Stop scheduling periodic fetches",
            (Self::AutoFetchDisabled, Language::Chinese) => "关闭界面内的周期获取",
            (Self::StartupFetch, Language::English) => "Fetch on startup",
            (Self::StartupFetch, Language::Chinese) => "启动时获取远程",
            (Self::StartupFetchHint, Language::English) => {
                "Fetch repository remotes on the next launch"
            }
            (Self::StartupFetchHint, Language::Chinese) => "下次启动时在后台获取仓库远程信息",
            (Self::StartupFetchEnabled, Language::English) => "Fetch remotes on the next launch",
            (Self::StartupFetchEnabled, Language::Chinese) => "下次启动时自动获取远程",
            (Self::StartupFetchDisabled, Language::English) => {
                "Only refresh local state on startup"
            }
            (Self::StartupFetchDisabled, Language::Chinese) => "启动时只刷新本地状态",
            (Self::StartupFetchSaved, Language::English) => {
                "Startup fetch preference saved for the next launch"
            }
            (Self::StartupFetchSaved, Language::Chinese) => "启动获取偏好已保存，下次启动生效",
            (Self::FetchAllCommand, Language::English) => "Fetch all managed repositories",
            (Self::FetchAllCommand, Language::Chinese) => "获取全部仓库的远程信息",
            (Self::FetchCommand, Language::English) => "Fetch configured remote-tracking branches",
            (Self::FetchCommand, Language::Chinese) => "获取远程跟踪分支",
            (Self::PullCommand, Language::English) => "Fetch and review a fast-forward-only pull",
            (Self::PullCommand, Language::Chinese) => "获取并预览仅允许快进的拉取",
            (Self::SyncCommand, Language::English) => "Review and sync the current branch",
            (Self::SyncCommand, Language::Chinese) => "预览并同步当前分支",
            (Self::SyncPreparing, Language::English) => "Synchronization review",
            (Self::SyncPreparing, Language::Chinese) => "同步预览",
            (Self::ProgressWaitingRepository, Language::English) => "Waiting for repository",
            (Self::ProgressWaitingRepository, Language::Chinese) => "等待仓库空闲",
            (Self::ProgressWaitingCapacity, Language::English) => "Waiting for execution capacity",
            (Self::ProgressWaitingCapacity, Language::Chinese) => "等待并发名额",
            (Self::ProgressInspectingRepository, Language::English) => {
                "Checking repository configuration"
            }
            (Self::ProgressInspectingRepository, Language::Chinese) => "检查仓库配置",
            (Self::ProgressFetchingRemote, Language::English) => "Fetching remote updates",
            (Self::ProgressFetchingRemote, Language::Chinese) => "正在获取远程",
            (Self::ProgressComparingBranches, Language::English) => "Comparing branches",
            (Self::ProgressComparingBranches, Language::Chinese) => "正在比较分支",
            (Self::ProgressCheckingWorktree, Language::English) => "Checking local changes",
            (Self::ProgressCheckingWorktree, Language::Chinese) => "正在检查本地改动",
            (Self::ProgressVerifyingRemote, Language::English) => "Verifying remote branch",
            (Self::ProgressVerifyingRemote, Language::Chinese) => "正在核验远程分支",
            (Self::ProgressPreparingPreview, Language::English) => {
                "Preparing synchronization preview"
            }
            (Self::ProgressPreparingPreview, Language::Chinese) => "生成同步预览",
            (Self::ProgressRefreshingState, Language::English) => "Updating repository status",
            (Self::ProgressRefreshingState, Language::Chinese) => "更新仓库状态",
            (Self::ProgressReadingView, Language::English) => "Refreshing current view",
            (Self::ProgressReadingView, Language::Chinese) => "刷新当前视图",
            (Self::ProgressTimings, Language::English) => "Stage timings",
            (Self::ProgressTimings, Language::Chinese) => "阶段耗时",
            (Self::SyncCurrent, Language::English) => "Current branch is synchronized",
            (Self::SyncCurrent, Language::Chinese) => "当前分支已同步",
            (Self::SyncNoRemotes, Language::English) => {
                "No remote configured; synchronization is unavailable"
            }
            (Self::SyncNoRemotes, Language::Chinese) => "未配置远程仓库，无法同步",
            (Self::SyncNoUpstream, Language::English) => {
                "Set a remote upstream for the current branch before synchronizing"
            }
            (Self::SyncNoUpstream, Language::Chinese) => "当前分支没有上游，请先设置远程上游",
            (Self::SyncLocalUpstream, Language::English) => {
                "Current branch tracks a local branch; synchronization requires a remote upstream"
            }
            (Self::SyncLocalUpstream, Language::Chinese) => "当前上游是本地分支，同步需要远程上游",
            (Self::SyncDetached, Language::English) => {
                "Switch to a local branch before synchronizing"
            }
            (Self::SyncDetached, Language::Chinese) => "HEAD 已游离，请先切换到本地分支再同步",
            (Self::SyncUnborn, Language::English) => "Create the first commit before synchronizing",
            (Self::SyncUnborn, Language::Chinese) => "当前分支尚无提交，请先创建提交",
            (Self::SyncDiverged, Language::English) => {
                "Current branch and upstream have diverged; merge or rebase before synchronizing"
            }
            (Self::SyncDiverged, Language::Chinese) => {
                "当前分支与上游已分叉，请先合并或 rebase 后再同步"
            }
            (Self::SyncPullReview, Language::English) => {
                "Branch: {branch}\nUpstream: {upstream}\nSync: ↑0 ↓{commits}\n\nAction: fast-forward pull {commits} commits\nTarget commit: {oid}"
            }
            (Self::SyncPullReview, Language::Chinese) => {
                "分支：{branch}\n上游：{upstream}\n同步：↑0 ↓{commits}\n\n执行：快进拉取 {commits} 个提交\n目标提交：{oid}"
            }
            (Self::SyncPushReview, Language::English) => {
                "Branch: {branch}\nUpstream: {remote}/{reference}\nSync: ↑{commits} ↓0\n\nAction: push {commits} commits\nTarget commit: {oid}"
            }
            (Self::SyncPushReview, Language::Chinese) => {
                "分支：{branch}\n上游：{remote}/{reference}\n同步：↑{commits} ↓0\n\n执行：推送 {commits} 个提交\n目标提交：{oid}"
            }
            (Self::SyncPulled, Language::English) => "Synchronized: pulled {branch}\nCommit: {oid}",
            (Self::SyncPulled, Language::Chinese) => "同步完成：已拉取 {branch}\n提交：{oid}",
            (Self::SyncPushed, Language::English) => {
                "Synchronized: pushed {commits} commits from {branch}\nRemote: {remote}/{reference}\nCommit: {oid}"
            }
            (Self::SyncPushed, Language::Chinese) => {
                "同步完成：已推送 {branch} 的 {commits} 个提交\n远程：{remote}/{reference}\n提交：{oid}"
            }
            (Self::StashPreviewEmpty, Language::English) => "No file changes in this scope",
            (Self::StashPreviewEmpty, Language::Chinese) => "该范围没有文件改动",
            (Self::StashPreviewNoMatch, Language::English) => "No matching files",
            (Self::StashPreviewNoMatch, Language::Chinese) => "没有匹配的文件",
            (Self::StashPreviewAll, Language::English) => "All saved changes",
            (Self::StashPreviewAll, Language::Chinese) => "全部改动",
            (Self::StashPreviewStaged, Language::English) => "Saved staged changes",
            (Self::StashPreviewStaged, Language::Chinese) => "原暂存改动",
            (Self::StashPreviewUnstaged, Language::English) => "Saved unstaged changes",
            (Self::StashPreviewUnstaged, Language::Chinese) => "原未暂存改动",
            (Self::StashPreviewScope, Language::English) => "View scope",
            (Self::StashPreviewScope, Language::Chinese) => "查看范围",
            (Self::StashBase, Language::English) => "Stash base",
            (Self::StashBase, Language::Chinese) => "保存基准",
            (Self::StashSavedWorktree, Language::English) => "Saved worktree",
            (Self::StashSavedWorktree, Language::Chinese) => "保存时工作区",
            (Self::StashSavedIndex, Language::English) => "Saved index",
            (Self::StashSavedIndex, Language::Chinese) => "保存时暂存区",
            (Self::StashAbsent, Language::English) => "Absent",
            (Self::StashAbsent, Language::Chinese) => "不存在",
            (Self::StashSavedUntracked, Language::English) => "Saved untracked file",
            (Self::StashSavedUntracked, Language::Chinese) => "保存时未跟踪文件",
            (Self::StashInvalidStructure, Language::English) => {
                "The selected object is not a supported Git stash"
            }
            (Self::StashInvalidStructure, Language::Chinese) => "所选对象不是支持的 Git stash 结构",
            (Self::StashPreviewCommand, Language::English) => "Choose the saved changes to inspect",
            (Self::StashPreviewCommand, Language::Chinese) => "选择查看的保存内容",
            (Self::StashPreviewRequiresDetail, Language::English) => {
                "Open a stash to choose its view scope"
            }
            (Self::StashPreviewRequiresDetail, Language::Chinese) => "请先打开一份 stash 的内容",
            (Self::StashDiffHint, Language::English) => {
                "↑↓ scroll · ←→ pan · Esc stash details · /repo diff layout"
            }
            (Self::StashDiffHint, Language::Chinese) => {
                "↑↓滚动 · ←→平移 · Esc 返回 · /repo diff layout 布局"
            }
            (Self::StashWrappedDiffHint, Language::English) => {
                "↑↓ scroll · Esc stash details · /repo diff layout"
            }
            (Self::StashWrappedDiffHint, Language::Chinese) => {
                "↑↓滚动 · Esc 返回 · /repo diff layout 布局"
            }
            (Self::StashPreviewHint, Language::English) => {
                "↑↓ files · Enter diff · /focus panes · Esc back"
            }
            (Self::StashPreviewHint, Language::Chinese) => {
                "↑↓文件 · Enter 差异 · /focus 面板 · Esc 返回"
            }
            (Self::StashTitle, Language::English) => "Stashes",
            (Self::StashTitle, Language::Chinese) => "暂存工作",
            (Self::StashRead, Language::English) => "Read stashes",
            (Self::StashRead, Language::Chinese) => "读取 stash",
            (Self::StashCommand, Language::English) => {
                "Browse saved work and manage the selected stash"
            }
            (Self::StashCommand, Language::Chinese) => "浏览并管理保存的工作",
            (Self::StashSaveIncomplete, Language::English) => {
                "Stash {oid} was created, but saving did not complete; inspect the workspace and index"
            }
            (Self::StashSaveIncomplete, Language::Chinese) => {
                "已生成 stash {oid}，保存尚未完成，请查看工作区与暂存区的实际状态"
            }
            (Self::StashSelectFiles, Language::English) => {
                "Select files using the workspace checkboxes first"
            }
            (Self::StashSelectFiles, Language::Chinese) => "请先在工作区勾选文件",
            (Self::StashInvalidPath, Language::English) => {
                "Stash paths must be repository-relative files"
            }
            (Self::StashInvalidPath, Language::Chinese) => "保存路径必须是仓库内的相对文件路径",
            (Self::StashInvalidMessage, Language::English) => {
                "The stash message cannot contain NUL"
            }
            (Self::StashInvalidMessage, Language::Chinese) => "保存说明不能包含 NUL 字符",
            (Self::StashSelectionChanged, Language::English) => {
                "Selected files have changed; reopen the stash review"
            }
            (Self::StashSelectionChanged, Language::Chinese) => {
                "勾选文件的状态已变化，请重新打开保存"
            }
            (Self::StashScopeTracked, Language::English) => "All tracked changes",
            (Self::StashScopeTracked, Language::Chinese) => "全部已跟踪改动",
            (Self::StashScopeAll, Language::English) => "All changes, including untracked files",
            (Self::StashScopeAll, Language::Chinese) => "全部改动，包含未跟踪文件",
            (Self::StashScopeSelected, Language::English) => "Checked files",
            (Self::StashScopeSelected, Language::Chinese) => "勾选文件",
            (Self::StashScopeLabel, Language::English) => "Stash scope",
            (Self::StashScopeLabel, Language::Chinese) => "保存范围",
            (Self::StashFilesCount, Language::English) => "Files · {count}",
            (Self::StashFilesCount, Language::Chinese) => "文件 · {count}",
            (Self::StashMessageOptional, Language::English) => "Message · optional",
            (Self::StashMessageOptional, Language::Chinese) => "保存说明 · 可留空",
            (Self::StashDefaultMessage, Language::English) => "Use the default Git message",
            (Self::StashDefaultMessage, Language::Chinese) => "使用 Git 默认说明",
            (Self::StashSaveHint, Language::English) => {
                "Shift+Tab mode · ←→ select · Enter confirm · Esc Back · PgUp/PgDn Scroll"
            }
            (Self::StashSaveHint, Language::Chinese) => {
                "Shift+Tab 模式 · ←→选择 · Enter 确认 · Esc 返回 · PgUp/PgDn 滚动"
            }
            (Self::StashSaveCommand, Language::English) => "Choose changes to stash",
            (Self::StashSaveCommand, Language::Chinese) => "选择并保存当前改动",
            (Self::StashSaveAllCommand, Language::English) => "Save tracked and untracked changes",
            (Self::StashSaveAllCommand, Language::Chinese) => "保存已跟踪和未跟踪变更",
            (Self::StashApplyCommand, Language::English) => "Restore a stash and retain its entry",
            (Self::StashApplyCommand, Language::Chinese) => "恢复 stash 并保留记录",
            (Self::StashPopCommand, Language::English) => {
                "Restore a stash and remove it only after success"
            }
            (Self::StashPopCommand, Language::Chinese) => "恢复 stash，仅成功后删除记录",
            (Self::StashDropCommand, Language::English) => "Delete a stash",
            (Self::StashDropCommand, Language::Chinese) => "删除 stash",
            (Self::StashPosition, Language::English) => "Position",
            (Self::StashPosition, Language::Chinese) => "位置",
            (Self::StashEmpty, Language::English) => "No matching stashes",
            (Self::StashEmpty, Language::Chinese) => "没有匹配的 stash",
            (Self::StashDetails, Language::English) => "Stash details",
            (Self::StashDetails, Language::Chinese) => "Stash 详情",
            (Self::StashSearch, Language::English) => "Search stashes",
            (Self::StashSearch, Language::Chinese) => "搜索 stash",
            (Self::StashHint, Language::English) => {
                "↑↓ Select · Enter Details · ←/Esc Back · / Commands"
            }
            (Self::StashHint, Language::Chinese) => "↑↓选择 · Enter 详情 · ←/Esc 返回 · / 命令",
            (Self::StashCopyCommand, Language::English) => "Copy the selected stash object ID",
            (Self::StashCopyCommand, Language::Chinese) => "复制选中 stash 的对象 ID",
            (Self::StashObject, Language::English) => "Stash object ID",
            (Self::StashObject, Language::Chinese) => "Stash 对象 ID",
            (Self::BranchNameHint, Language::English) => {
                "Enter a local branch name · Enter Review · Esc Return"
            }
            (Self::BranchNameHint, Language::Chinese) => "输入本地分支名 · Enter 预览 · Esc 返回",
            (Self::StashMessageHint, Language::English) => {
                "Enter a message · Enter Review · Esc Return"
            }
            (Self::StashMessageHint, Language::Chinese) => "输入保存消息 · Enter 预览 · Esc 返回",
            (Self::StashIdHint, Language::English) => {
                "Enter the full 40-character stash object ID · Enter Review"
            }
            (Self::StashIdHint, Language::Chinese) => "输入完整的 40 位 stash 对象 ID · Enter 预览",
            (Self::StashIdRequired, Language::English) => {
                "A full 40-character hexadecimal stash object ID is required"
            }
            (Self::StashIdRequired, Language::Chinese) => {
                "请输入完整的 40 位十六进制 stash 对象 ID"
            }
            (Self::MutationRun, Language::English) => "Execute",
            (Self::MutationRun, Language::Chinese) => "执行",
            (Self::MutationComplete, Language::English) => {
                "Operation completed; repository status updated"
            }
            (Self::MutationComplete, Language::Chinese) => "操作完成，仓库状态已更新",
            (Self::PullCurrent, Language::English) => {
                "The local branch already contains the upstream commit"
            }
            (Self::PullCurrent, Language::Chinese) => "本地分支已包含上游提交",
            (Self::PullReview, Language::English) => {
                "Fast-forward {0} commits to {1}\nUpstream: {2}"
            }
            (Self::PullReview, Language::Chinese) => "快进 {0} 个提交至 {1}\n上游：{2}",
            (Self::StashDropWarning, Language::English) => "Permanently delete this stash entry.",
            (Self::StashDropWarning, Language::Chinese) => "将永久删除此 stash 记录。",
            (Self::MutationTarget, Language::English) => "Target",
            (Self::MutationTarget, Language::Chinese) => "目标",
            (Self::MutationRequiresCommit, Language::English) => {
                "This operation requires an existing commit"
            }
            (Self::MutationRequiresCommit, Language::Chinese) => "此操作要求仓库已有提交",
            (Self::MutationDirtySubmodule, Language::English) => {
                "A submodule has local changes; handle it in its own repository first"
            }
            (Self::MutationDirtySubmodule, Language::Chinese) => {
                "子模块存在本地改动，请先在子模块仓库中处理"
            }
            (Self::MutationNonRegularFile, Language::English) => {
                "A changed path is not a regular file or symlink"
            }
            (Self::MutationNonRegularFile, Language::Chinese) => "改动路径不是普通文件或符号链接",
            (Self::MutationChanged, Language::English) => {
                "The repository changed after review; reopen the operation"
            }
            (Self::MutationChanged, Language::Chinese) => "预览后仓库状态已变化，请重新准备操作",
            (Self::MutationCleanRequired, Language::English) => {
                "Commit or stash tracked changes before this operation"
            }
            (Self::MutationCleanRequired, Language::Chinese) => "请先提交或 stash 已跟踪文件的改动",
            (Self::MutationInvalidBranch, Language::English) => "Enter a valid local branch name",
            (Self::MutationInvalidBranch, Language::Chinese) => "请输入有效的本地分支名称",
            (Self::MutationLocalBranchRequired, Language::English) => {
                "Choose an existing local branch; remote branches are not created implicitly"
            }
            (Self::MutationLocalBranchRequired, Language::Chinese) => {
                "请选择已有本地分支；不会隐式创建远程跟踪分支"
            }
            (Self::MutationAlreadyOnBranch, Language::English) => {
                "This branch is already checked out"
            }
            (Self::MutationAlreadyOnBranch, Language::Chinese) => "当前已经检出此分支",
            (Self::MutationBranchExists, Language::English) => {
                "A branch with this name already exists"
            }
            (Self::MutationBranchExists, Language::Chinese) => "同名分支已存在",
            (Self::MutationBranchInOtherWorktree, Language::English) => {
                "This branch is checked out in another worktree"
            }
            (Self::MutationBranchInOtherWorktree, Language::Chinese) => {
                "此分支已在另一个 worktree 中检出"
            }
            (Self::MutationStashMessageRequired, Language::English) => {
                "Enter a nonempty stash message without NUL characters"
            }
            (Self::MutationStashMessageRequired, Language::Chinese) => {
                "请输入非空 stash 说明，且不能包含空字符"
            }
            (Self::MutationNothingToStash, Language::English) => "No changes match this stash mode",
            (Self::MutationNothingToStash, Language::Chinese) => "没有符合此 stash 范围的改动",
            (Self::MutationStashChanged, Language::English) => {
                "The stash selection or list changed; refresh it and select again"
            }
            (Self::MutationStashChanged, Language::Chinese) => {
                "stash 条目或列表已变化，请刷新后重新选择"
            }
            (Self::MutationUpstreamRequired, Language::English) => {
                "Configure one explicit upstream branch before pulling"
            }
            (Self::MutationUpstreamRequired, Language::Chinese) => "拉取前请为当前分支配置唯一上游",
            (Self::MutationPullAttached, Language::English) => {
                "Switch to a local branch with a commit before pulling"
            }
            (Self::MutationPullAttached, Language::Chinese) => "拉取前请切换到已有提交的本地分支",
            (Self::MutationPullDiverged, Language::English) => {
                "The branch has diverged; choose merge or rebase explicitly outside this operation"
            }
            (Self::MutationPullDiverged, Language::Chinese) => {
                "分支已分叉，请另行明确选择 merge 或 rebase"
            }
            (Self::MutationRemoteNotUtf8, Language::English) => {
                "The remote name is not valid UTF-8"
            }
            (Self::MutationRemoteNotUtf8, Language::Chinese) => "远程名称不是有效 UTF-8",
            (Self::MutationFetchMappingRequired, Language::English) => {
                "This remote has no supported remote-tracking fetch mapping"
            }
            (Self::MutationFetchMappingRequired, Language::Chinese) => {
                "此远程没有可用的远程跟踪 fetch 映射"
            }
            (Self::MutationNoRemote, Language::English) => "No eligible remote is configured",
            (Self::MutationNoRemote, Language::Chinese) => "没有配置可用的远程",
            (Self::NoRemoteConfiguredSkipped, Language::English) => "No remote configured; skipped",
            (Self::NoRemoteConfiguredSkipped, Language::Chinese) => "未配置远程，已跳过",
            (Self::CopyRepositoryCommand, Language::English) => "Copy the selected repository path",
            (Self::CopyRepositoryCommand, Language::Chinese) => "复制当前仓库的完整路径",
            (Self::CopyDiffCommand, Language::English) => {
                "Copy selected file paths or the focused diff preview"
            }
            (Self::CopyDiffCommand, Language::Chinese) => "复制选中的文件路径或焦点中的差异预览",
            (Self::CopyLogCommand, Language::English) => {
                "Copy the selected commit ID or historical file path"
            }
            (Self::CopyLogCommand, Language::Chinese) => "复制选中的提交 ID 或历史文件路径",
            (Self::CopyBranchCommand, Language::English) => "Copy the selected branch name",
            (Self::CopyBranchCommand, Language::Chinese) => "复制选中的分支名称",
            (Self::CopyErrorCommand, Language::English) => "Copy complete error details",
            (Self::CopyErrorCommand, Language::Chinese) => "复制完整错误详情",
            (Self::CopyRepositoryPath, Language::English) => "Repository path",
            (Self::CopyRepositoryPath, Language::Chinese) => "仓库路径",
            (Self::CopyFilePaths, Language::English) => "Selected file paths",
            (Self::CopyFilePaths, Language::Chinese) => "选中文件路径",
            (Self::CopyCommit, Language::English) => "Full commit ID",
            (Self::CopyCommit, Language::Chinese) => "完整提交 ID",
            (Self::CopyBranch, Language::English) => "Branch name",
            (Self::CopyBranch, Language::Chinese) => "分支名称",
            (Self::CopyDiff, Language::English) => "Original diff patch",
            (Self::CopyDiff, Language::Chinese) => "原始差异补丁",
            (Self::CopyError, Language::English) => "Error details",
            (Self::CopyError, Language::Chinese) => "错误详情",
            (Self::CopyWriting, Language::English) => "Copying",
            (Self::CopyWriting, Language::Chinese) => "正在复制",
            (Self::CopySucceeded, Language::English) => "Copied",
            (Self::CopySucceeded, Language::Chinese) => "已复制",
            (Self::CopyFailed, Language::English) => "Copy failed",
            (Self::CopyFailed, Language::Chinese) => "复制失败",
            (Self::CopyUnavailable, Language::English) => "No copyable selection",
            (Self::CopyUnavailable, Language::Chinese) => "无可复制内容",
            (Self::CopySelectionRequired, Language::English) => {
                "The current view has no selected content to copy"
            }
            (Self::CopySelectionRequired, Language::Chinese) => "当前视图没有选中的可复制内容",
            (Self::CopySelectionLoading, Language::English) => {
                "The selected content is still loading; reopen the command after it finishes"
            }
            (Self::CopySelectionLoading, Language::Chinese) => {
                "选中内容仍在加载，请完成后重新打开命令"
            }
            (Self::CopyPathNotUtf8, Language::English) => {
                "The selected path is not valid UTF-8 and cannot be copied as text"
            }
            (Self::CopyPathNotUtf8, Language::Chinese) => {
                "选中路径不是有效 UTF-8，无法作为文本复制"
            }
            (Self::CopyDiffRequired, Language::English) => {
                "This preview has no original text patch to copy"
            }
            (Self::CopyDiffRequired, Language::Chinese) => "此预览没有可复制的原始文本补丁",
            (Self::CopyDiffImage, Language::English) => "Image previews have no text patch to copy",
            (Self::CopyDiffImage, Language::Chinese) => "图片预览没有可复制的文本补丁",
            (Self::CopyDiffTruncated, Language::English) => {
                "This diff is truncated; a complete patch cannot be copied"
            }
            (Self::CopyDiffTruncated, Language::Chinese) => "此差异已截断，无法复制完整补丁",
            (Self::CopyErrorHint, Language::English) => {
                "↑↓/PgDn scroll · / copy error · Enter/Esc back"
            }
            (Self::CopyErrorHint, Language::Chinese) => {
                "↑↓/PgDn 滚动 · / 复制错误 · Enter/Esc 返回"
            }
            (Self::ClipboardOpenFailed, Language::English) => "Cannot access the system clipboard",
            (Self::ClipboardOpenFailed, Language::Chinese) => "无法访问系统剪贴板",
            (Self::ClipboardWriteFailed, Language::English) => {
                "Cannot write to the system clipboard"
            }
            (Self::ClipboardWriteFailed, Language::Chinese) => "无法写入系统剪贴板",
            (Self::ClipboardWorkerStopped, Language::English) => "The clipboard worker stopped",
            (Self::ClipboardWorkerStopped, Language::Chinese) => "剪贴板任务已停止",
            (Self::ClipboardBusy, Language::English) => "A clipboard write is already in progress",
            (Self::ClipboardBusy, Language::Chinese) => "已有复制任务正在进行",
            (Self::RefreshContextView, Language::English) => "Refresh this view",
            (Self::RefreshContextView, Language::Chinese) => "刷新当前视图",
            (Self::WorkflowLegacyConflict, Language::English) => {
                "Configure scheduled_actions without the former push_policy field."
            }
            (Self::WorkflowLegacyConflict, Language::Chinese) => {
                "push_policy 与 scheduled_actions 不能同时配置。"
            }
            (Self::WorkflowPolicyString, Language::English) => "push_policy must be a string.",
            (Self::WorkflowPolicyString, Language::Chinese) => "push_policy 必须是字符串。",
            (Self::WorkflowObjectExpected, Language::English) => "Configuration must be an object.",
            (Self::WorkflowObjectExpected, Language::Chinese) => "配置必须是对象。",
            (Self::WorkflowNoPush, Language::English) => "No strictly-ahead branches to push",
            (Self::WorkflowNoPush, Language::Chinese) => "没有可推送的领先分支",
            (Self::WorkflowFetch, Language::English) => "Fetch remotes",
            (Self::WorkflowFetch, Language::Chinese) => "获取远程",
            (Self::WorkflowPull, Language::English) => "Fast-forward pull",
            (Self::WorkflowPull, Language::Chinese) => "快进拉取",
            (Self::WorkflowPush, Language::English) => "Push commits",
            (Self::WorkflowPush, Language::Chinese) => "推送提交",
            (Self::WorkflowInspect, Language::English) => "Read local status",
            (Self::WorkflowInspect, Language::Chinese) => "读取本地状态",
            (Self::WorkflowFetchDescription, Language::English) => {
                "Download remote commits, refresh local remote-tracking branches, and remove records of deleted remote branches."
            }
            (Self::WorkflowFetchDescription, Language::Chinese) => {
                "获取远程提交，更新本地保存的远程跟踪分支，并清理远程已删除的分支记录。"
            }
            (Self::WorkflowPullDescription, Language::English) => {
                "Fetch the current upstream and fast-forward the checked-out branch and files. Stop for tracked changes, divergence, or conflicts."
            }
            (Self::WorkflowPullDescription, Language::Chinese) => {
                "获取当前分支的上游提交，以快进方式更新本地分支和文件。已跟踪文件有改动、分叉或冲突时停止。"
            }
            (Self::WorkflowPushDescription, Language::English) => {
                "Publish local branches that are strictly ahead of their remote upstreams; verify remote commits before and after pushing."
            }
            (Self::WorkflowPushDescription, Language::Chinese) => {
                "将仅领先上游的本地分支推送到对应远程分支，推送前后核验远程提交。"
            }
            (Self::WorkflowInspectDescription, Language::English) => {
                "Read worktree, index, branch and upstream status, then save the result."
            }
            (Self::WorkflowInspectDescription, Language::Chinese) => {
                "读取工作区、暂存区、分支和上游状态，保存本次结果。"
            }
            (Self::WorkflowStartup, Language::English) => "Startup",
            (Self::WorkflowStartup, Language::Chinese) => "启动任务",
            (Self::WorkflowScheduled, Language::English) => "Scheduled workflow",
            (Self::WorkflowScheduled, Language::Chinese) => "定时任务",
            (Self::WorkflowInspectLast, Language::English) => {
                "End the workflow with inspect (read local status)."
            }
            (Self::WorkflowInspectLast, Language::Chinese) => {
                "流程必须以 inspect（读取本地状态）结束。"
            }
            (Self::WorkflowDuplicateStep, Language::English) => {
                "A workflow cannot contain duplicate steps."
            }
            (Self::WorkflowDuplicateStep, Language::Chinese) => "同一流程中不能重复添加步骤。",
            (Self::WorkflowPushNeedsRefresh, Language::English) => "Add fetch or pull before push.",
            (Self::WorkflowPushNeedsRefresh, Language::Chinese) => {
                "push 前需要先执行 fetch 或 pull。"
            }
            (Self::WorkflowUnknownStep, Language::English) => {
                "Unknown step: {}; use fetch, pull, push, inspect."
            }
            (Self::WorkflowUnknownStep, Language::Chinese) => {
                "未知步骤：{}；可用 fetch、pull、push、inspect。"
            }
            (Self::WorkflowScheduledActions, Language::English) => "Workflow",
            (Self::WorkflowScheduledActions, Language::Chinese) => "执行流程",
            (Self::WorkflowToggle, Language::English) => "Toggle step",
            (Self::WorkflowToggle, Language::Chinese) => "勾选／取消",
            (Self::WorkflowEnableStep, Language::English) => "Include step",
            (Self::WorkflowEnableStep, Language::Chinese) => "加入流程",
            (Self::WorkflowDisableStep, Language::English) => "Exclude step",
            (Self::WorkflowDisableStep, Language::Chinese) => "移出流程",
            (Self::WorkflowRequired, Language::English) => "Fixed",
            (Self::WorkflowRequired, Language::Chinese) => "固定",
            (Self::WorkflowSequence, Language::English) => "Order",
            (Self::WorkflowSequence, Language::Chinese) => "执行顺序",
            (Self::WorkflowControls, Language::English) => "Actions",
            (Self::WorkflowControls, Language::Chinese) => "操作",
            (Self::WorkflowSelectedStep, Language::English) => "Selected step",
            (Self::WorkflowSelectedStep, Language::Chinese) => "当前步骤",
            (Self::WorkflowEditorHint, Language::English) => {
                "↑↓ step · Space mark · ←→ action · Enter run · Shift+Tab save/cancel · Esc back"
            }
            (Self::WorkflowEditorHint, Language::Chinese) => {
                "↑↓步骤 · Space 勾选 · ←→操作 · Enter 执行 · Shift+Tab 保存/取消 · Esc 返回"
            }
            (Self::WorkflowMoveUp, Language::English) => "Move up",
            (Self::WorkflowMoveUp, Language::Chinese) => "上移",
            (Self::WorkflowMoveDown, Language::English) => "Move down",
            (Self::WorkflowMoveDown, Language::Chinese) => "下移",
            (Self::WorkflowSave, Language::English) => "Save workflow",
            (Self::WorkflowSave, Language::Chinese) => "保存流程",
            (Self::WorkflowCancel, Language::English) => "Cancel",
            (Self::WorkflowCancel, Language::Chinese) => "取消",
            (Self::WorkflowSaved, Language::English) => {
                "Workflow saved; applies at the next trigger."
            }
            (Self::WorkflowSaved, Language::Chinese) => "执行流程已保存，下次触发时生效。",
            (Self::WorkflowEditing, Language::English) => {
                "Check to include; uncheck to remove. Save to apply."
            }
            (Self::WorkflowEditing, Language::Chinese) => {
                "勾选加入流程，取消勾选移除。保存后生效。"
            }
            (Self::WorkflowScope, Language::English) => "Scope: all registered repositories",
            (Self::WorkflowScope, Language::Chinese) => "执行范围：全部已登记仓库",
            (Self::WorkflowFixedInspect, Language::English) => {
                "The final step reads and saves actual status."
            }
            (Self::WorkflowFixedInspect, Language::Chinese) => "最后一步固定读取并保存实际状态。",
            (Self::WorkflowActionsArgument, Language::English) => {
                "Enter steps in order, ending with inspect"
            }
            (Self::WorkflowActionsArgument, Language::Chinese) => {
                "按执行顺序填写步骤，以 inspect 结尾"
            }
            (Self::WorkflowScheduleHint, Language::English) => {
                "Choose scheduled actions and their order."
            }
            (Self::WorkflowScheduleHint, Language::Chinese) => "选择定时触发时执行的步骤和顺序。",
            (Self::WorkflowStopped, Language::English) => "Not run: an earlier step failed",
            (Self::WorkflowStopped, Language::Chinese) => "未执行：前序步骤失败",
            (Self::ContextCommands, Language::English) => "View commands",
            (Self::ContextCommands, Language::Chinese) => "当前视图命令",
            (Self::ContextCommandPlaceholder, Language::English) => "Type a command for this view…",
            (Self::ContextCommandPlaceholder, Language::Chinese) => "输入当前视图的操作…",
            (Self::ContextCommandHint, Language::English) => {
                "Tab complete · ↑↓ candidates · Enter run · Esc close"
            }
            (Self::ContextCommandHint, Language::Chinese) => {
                "Tab 补全 · ↑↓候选 · Enter 执行 · Esc 关闭"
            }
            (Self::ContextGlobalCommandHint, Language::English) => {
                "Press / again for global commands · Tab complete · Enter run · ⌫ edit prefix"
            }
            (Self::ContextGlobalCommandHint, Language::Chinese) => {
                "再按 / 切换到全局命令 · Tab 补全 · Enter 执行 · ⌫ 编辑前缀"
            }
            (Self::ContextPrefixHint, Language::English) => {
                "←→ select · Type replace · ⌫ delete · Enter/Esc keep"
            }
            (Self::ContextPrefixHint, Language::Chinese) => {
                "←→ 选区 · 输入替换 · ⌫ 删除 · Enter/Esc 保留"
            }
            (Self::RepositoryFilterCommand, Language::English) => "Filter repositories",
            (Self::RepositoryFilterCommand, Language::Chinese) => "筛选仓库",
            (Self::RepositoryFilterLabel, Language::English) => "Filter",
            (Self::RepositoryFilterLabel, Language::Chinese) => "筛选",
            (Self::RepositoryFilterAll, Language::English) => "All registered repositories",
            (Self::RepositoryFilterAll, Language::Chinese) => "全部已注册仓库",
            (Self::RepositoryFilterProblems, Language::English) => {
                "Repositories needing attention or with execution failures"
            }
            (Self::RepositoryFilterProblems, Language::Chinese) => {
                "有工作区改动、当前分支待同步或执行失败的仓库"
            }
            (Self::RepositoryFilterClean, Language::English) => {
                "Normal repositories without pending changes"
            }
            (Self::RepositoryFilterClean, Language::Chinese) => "没有待处理改动且执行正常的仓库",
            (Self::RepositoryFilterFailed, Language::English) => "Last execution failed",
            (Self::RepositoryFilterFailed, Language::Chinese) => "上次执行失败的仓库",
            (Self::RepositoryFilterUnchecked, Language::English) => "No recorded repository status",
            (Self::RepositoryFilterUnchecked, Language::Chinese) => "尚无状态记录的仓库",
            (Self::RepositoryFilterNoUpstream, Language::English) => {
                "Branches without a remote upstream"
            }
            (Self::RepositoryFilterNoUpstream, Language::Chinese) => "存在未配置远程上游分支的仓库",
            (Self::RepositoryFilterPushes, Language::English) => {
                "Recorded push successes or failures"
            }
            (Self::RepositoryFilterPushes, Language::Chinese) => "有推送成功或失败记录的仓库",
            (Self::RepositorySortCommand, Language::English) => "Sort repositories",
            (Self::RepositorySortCommand, Language::Chinese) => "仓库排序",
            (Self::RepositorySortLabel, Language::English) => "Sort",
            (Self::RepositorySortLabel, Language::Chinese) => "排序",
            (Self::RepositorySortPriority, Language::English) => {
                "Failed, needs attention, unchecked, normal"
            }
            (Self::RepositorySortPriority, Language::Chinese) => "失败、需关注、未检查、正常",
            (Self::RepositorySortName, Language::English) => "Repository name, A to Z",
            (Self::RepositorySortName, Language::Chinese) => "按仓库名称升序，忽略英文大小写",
            (Self::RepositorySortPath, Language::English) => "Full repository path, ascending",
            (Self::RepositorySortPath, Language::Chinese) => "按完整仓库路径升序",
            (Self::RepositorySortUpdated, Language::English) => {
                "Latest state update first; unchecked last"
            }
            (Self::RepositorySortUpdated, Language::Chinese) => {
                "状态更新时间从新到旧，未检查排末尾"
            }
            (Self::RepositoryOrderPriority, Language::English) => "Priority",
            (Self::RepositoryOrderPriority, Language::Chinese) => "状态优先",
            (Self::RepositoryOrderName, Language::English) => "Name A→Z",
            (Self::RepositoryOrderName, Language::Chinese) => "名称升序",
            (Self::RepositoryOrderPath, Language::English) => "Path ↑",
            (Self::RepositoryOrderPath, Language::Chinese) => "路径升序",
            (Self::RepositoryOrderUpdated, Language::English) => "Recently updated",
            (Self::RepositoryOrderUpdated, Language::Chinese) => "最近更新",
            (Self::RepositoryListOnly, Language::English) => {
                "Use /repos filter and /repos sort on the repository home page"
            }
            (Self::RepositoryListOnly, Language::Chinese) => {
                "请在仓库首页使用 /repos filter 和 /repos sort"
            }
            (Self::RepositoryListHint, Language::English) => "↑↓ select · →/Enter open",
            (Self::RepositoryListHint, Language::Chinese) => "↑↓选择 · →/Enter 打开",
            (Self::RepositoryListNoMatches, Language::English) => {
                "No matches; edit search or /repos filter"
            }
            (Self::RepositoryListNoMatches, Language::Chinese) => {
                "没有匹配仓库，请修改搜索或 /repos filter 筛选"
            }
            (Self::InvalidScheduleAnchor, Language::English) => {
                "Use HH:MM[:SS], YYYY-MM-DD HH:MM[:SS], or auto"
            }
            (Self::InvalidScheduleAnchor, Language::Chinese) => {
                "基准格式为 HH:MM[:SS]、YYYY-MM-DD HH:MM[:SS] 或 auto"
            }
            (Self::AmbiguousScheduleAnchor, Language::English) => {
                "This local time is missing or repeated; choose an unambiguous time"
            }
            (Self::AmbiguousScheduleAnchor, Language::Chinese) => {
                "此本地时间不存在或重复，请选择没有歧义的时刻"
            }
            (Self::ScheduleEnableToSetAnchor, Language::English) => {
                "Enable the task first to establish its automatic time basis"
            }
            (Self::ScheduleEnableToSetAnchor, Language::Chinese) => {
                "请先启用任务，生成并保存自动时间基准"
            }
            (Self::ScheduleAnchorLabel, Language::English) => "Time basis",
            (Self::ScheduleAnchorLabel, Language::Chinese) => "时间基准",
            (Self::ScheduleNextLabel, Language::English) => "Next trigger",
            (Self::ScheduleNextLabel, Language::Chinese) => "下次触发",
            (Self::ScheduleAnchorAutomatic, Language::English) => "automatic",
            (Self::ScheduleAnchorAutomatic, Language::Chinese) => "自动",
            (Self::ScheduleAnchorUnset, Language::English) => "Generated on first enable",
            (Self::ScheduleAnchorUnset, Language::Chinese) => "首次启用时自动生成",
            (Self::ScheduleNextDisabled, Language::English) => "Disabled",
            (Self::ScheduleNextDisabled, Language::Chinese) => "已停用",
            (Self::ScheduleNextUnanchored, Language::English) => "Apply to establish a time basis",
            (Self::ScheduleNextUnanchored, Language::Chinese) => "应用后确定时间基准",
            (Self::ScheduleNextMissing, Language::English) => "Not installed",
            (Self::ScheduleNextMissing, Language::Chinese) => "未安装",
            (Self::EditScheduleAnchor, Language::English) => "Edit time basis",
            (Self::EditScheduleAnchor, Language::Chinese) => "修改时间基准",
            (Self::ScheduleEditButton, Language::English) => "Edit",
            (Self::ScheduleEditButton, Language::Chinese) => "修改",
            (Self::ScheduleWorkflowButton, Language::English) => "Edit",
            (Self::ScheduleWorkflowButton, Language::Chinese) => "编辑",
            (Self::ScheduleEnableButton, Language::English) => "Enable",
            (Self::ScheduleEnableButton, Language::Chinese) => "启用",
            (Self::ScheduleDisableButton, Language::English) => "Disable",
            (Self::ScheduleDisableButton, Language::Chinese) => "停用",
            (Self::ScheduleApplyButton, Language::English) => "Apply",
            (Self::ScheduleApplyButton, Language::Chinese) => "应用",
            (Self::ScheduleAnchorHint, Language::English) => {
                "HH:MM[:SS] · YYYY-MM-DD HH:MM[:SS] · auto resets on next enable"
            }
            (Self::ScheduleAnchorHint, Language::Chinese) => {
                "HH:MM[:SS] · YYYY-MM-DD HH:MM[:SS] · auto 在下次启用时重新生成"
            }
            (Self::SaveScheduleAnchor, Language::English) => {
                "Save the time basis; auto generates it on enable"
            }
            (Self::SaveScheduleAnchor, Language::Chinese) => "保存时间基准；auto 在启用时自动生成",
            (Self::LayoutVisible, Language::English) => "showing",
            (Self::LayoutVisible, Language::Chinese) => "当前",
            (Self::LayoutCurrent, Language::English) => "Current",
            (Self::LayoutCurrent, Language::Chinese) => "当前",

            (Self::ChangesOpen, Language::English) => "Open file changes with a live diff preview",
            (Self::ChangesOpen, Language::Chinese) => "打开工作区与差异预览",
            (Self::ChangesOpenStaged, Language::Chinese) => "查看已暂存改动",
            (Self::ChangesOpenStaged, Language::English) => "Show staged changes",
            (Self::ChangesOpenUnstaged, Language::Chinese) => "查看未暂存改动",
            (Self::ChangesOpenUnstaged, Language::English) => "Show unstaged changes",
            (Self::ChangesLayout, Language::English) => {
                "Choose automatic, split or unified diff layout"
            }
            (Self::ChangesLayout, Language::Chinese) => "选择自动、双栏或统一差异布局",
            (Self::ChangesTitle, Language::English) => "File changes",
            (Self::ChangesTitle, Language::Chinese) => "文件变更",
            (Self::ChangesUnstaged, Language::English) => "Unstaged",
            (Self::ChangesUnstaged, Language::Chinese) => "未暂存",
            (Self::ChangesStaged, Language::English) => "Staged",
            (Self::ChangesStaged, Language::Chinese) => "已暂存",
            (Self::FileListClean, Language::English) => "No working tree changes",
            (Self::FileListClean, Language::Chinese) => "工作区没有变更",
            (Self::ChangesNoStaged, Language::Chinese) => "没有已暂存改动",
            (Self::ChangesNoStaged, Language::English) => "No staged changes",
            (Self::ChangesNoUnstaged, Language::Chinese) => "没有未暂存改动",
            (Self::ChangesNoUnstaged, Language::English) => "No unstaged changes",
            (Self::FileListNoMatches, Language::English) => "No matching changed files",
            (Self::FileListNoMatches, Language::Chinese) => "没有匹配的变更文件",
            (Self::ChangesSelectFile, Language::English) => {
                "Select a changed file to preview its diff"
            }
            (Self::ChangesSelectFile, Language::Chinese) => "选择变更文件以预览差异",
            (Self::ChangesSelectionRequired, Language::English) => {
                "Open /repo diff and select files before staging or unstaging"
            }
            (Self::ChangesSelectionRequired, Language::Chinese) => {
                "请先打开 /repo diff 并选择文件，再暂存或取消暂存"
            }
            (Self::ChangesWrongArea, Language::English) => {
                "Stage requires only unstaged files; unstage requires only staged files. Review the selected groups."
            }
            (Self::ChangesWrongArea, Language::Chinese) => {
                "暂存只能选择未暂存文件，取消暂存只能选择已暂存文件；请核对勾选区域"
            }
            (Self::DirectoryInputHint, Language::English) => {
                "Enter fold/unfold · Space/Ins mark · /focus panes · Esc back"
            }
            (Self::DirectoryInputHint, Language::Chinese) => {
                "Enter 展开/收起 · Space/Ins 勾选 · /focus 切换 · Esc 返回"
            }
            (Self::ChangesInputHint, Language::English) => {
                "↑↓ navigate · ←→ pan preview · /focus panes · //zoom maximize · Esc back"
            }
            (Self::ChangesInputHint, Language::Chinese) => {
                "↑↓选择/滚动 · ←→平移预览 · /focus切换 · //zoom放大 · Esc返回"
            }

            (Self::RepositoryTaskCompleted, Language::English) => "{0} · Completed: {1}",
            (Self::RepositoryTaskCompleted, Language::Chinese) => "{0} · 已完成：{1}",
            (Self::InvalidToml, Language::English) => "Invalid TOML configuration",
            (Self::InvalidToml, Language::Chinese) => "TOML 配置无效",
            (Self::ConfigurationMigrationRequired, Language::English) => {
                "Legacy config.json exists; run the migrate config subcommand before reading or editing configuration"
            }
            (Self::ConfigurationMigrationRequired, Language::Chinese) => {
                "存在旧 config.json，请先运行 migrate config 子命令，再读取或修改配置"
            }
            (Self::ConfigurationMigrationConflict, Language::English) => {
                "JSON and TOML configurations conflict or changed during migration; no file was overwritten"
            }
            (Self::ConfigurationMigrationConflict, Language::Chinese) => {
                "JSON 与 TOML 配置冲突，或迁移期间文件发生变化，未覆盖任何文件"
            }
            (Self::TransferTargetExists, Language::English) => {
                "Destination configuration or database already exists; nothing was overwritten"
            }
            (Self::TransferTargetExists, Language::Chinese) => {
                "目标配置或数据库已存在，未覆盖任何数据"
            }
            (Self::TransferSameSource, Language::English) => {
                "Source and destination storage must be different"
            }
            (Self::TransferSameSource, Language::Chinese) => "来源与目标存储目录必须不同",
            (Self::MigrateFromUsage, Language::English) => {
                "Usage: migrate config | migrate --from ABSOLUTE_PATH; exit all source processes before importing"
            }
            (Self::MigrateFromUsage, Language::Chinese) => {
                "用法：migrate config 或 migrate --from 绝对路径；导入前请退出所有使用来源数据的进程"
            }
            (Self::StorageMigrationRequired, Language::English) => {
                "Legacy state JSON requires manual conversion"
            }
            (Self::StorageMigrationRequired, Language::Chinese) => {
                "检测到旧状态 JSON，需要手动转换"
            }
            (Self::InvalidStateDatabase, Language::English) => "Invalid state database",
            (Self::InvalidStateDatabase, Language::Chinese) => "状态数据库无效",
            (Self::UnsupportedStateDatabase, Language::English) => {
                "Unsupported state database identity or schema version"
            }
            (Self::UnsupportedStateDatabase, Language::Chinese) => {
                "状态数据库标识或结构版本不受支持"
            }
            (Self::StateDatabaseRequiresWal, Language::English) => {
                "State database must use WAL journal mode"
            }
            (Self::StateDatabaseRequiresWal, Language::Chinese) => {
                "状态数据库必须使用 WAL 日志模式"
            }
            (Self::InvalidHistoryLimit, Language::English) => {
                "History limit must be between 1 and 1000"
            }
            (Self::InvalidHistoryLimit, Language::Chinese) => "历史查询条数必须在 1 到 1000 之间",
            (Self::LegacyStateConflict, Language::English) => {
                "Legacy state or archive contains conflicting data; migration stopped"
            }
            (Self::LegacyStateConflict, Language::Chinese) => {
                "旧状态或备份存在数据冲突，已停止迁移"
            }
            (Self::LegacyStateChanged, Language::English) => {
                "Legacy JSON changed or its archive is missing; stop older versions before migrating"
            }
            (Self::LegacyStateChanged, Language::Chinese) => {
                "旧 JSON 已变化或备份缺失，请退出旧版本后迁移"
            }
            (Self::RepositoryStateSaveFailed, Language::English) => {
                "Git operation finished, but saving the repository state failed"
            }
            (Self::RepositoryStateSaveFailed, Language::Chinese) => {
                "Git 操作已结束，但仓库状态保存失败"
            }
            (Self::RepositoryTaskBusy, Language::English) => {
                "An operation is already running for this repository"
            }
            (Self::RepositoryTaskBusy, Language::Chinese) => "此仓库已有操作正在执行",
            (Self::RepositoryTaskLimit, Language::English) => {
                "Repository task limit reached; wait for an operation to finish"
            }
            (Self::RepositoryTaskLimit, Language::Chinese) => {
                "已达到仓库任务并发上限，请等待一个操作完成"
            }
            (Self::RepositoryTasksRunning, Language::English) => "  {0} · {1} repository tasks",
            (Self::RepositoryTasksRunning, Language::Chinese) => "  {0} · {1} 个仓库任务",
            (Self::ImageInputHint, Language::English) => "Fit to pane · Esc Back · / Commands",
            (Self::ImageInputHint, Language::Chinese) => "适应窗口 · Esc 返回 · / 命令",
            (Self::ImagePreview, Language::English) => "Image preview",
            (Self::ImagePreview, Language::Chinese) => "图片预览",
            (Self::ImageAbsent, Language::English) => "No image in this version",
            (Self::ImageAbsent, Language::Chinese) => "此版本中没有图片",
            (Self::ImageNotRaster, Language::English) => "This version is not a supported image",
            (Self::ImageNotRaster, Language::Chinese) => "此版本不是支持的图片",
            (Self::ImageEncodedLimit, Language::English) => {
                "Image exceeds the 16 MiB encoded size limit"
            }
            (Self::ImageEncodedLimit, Language::Chinese) => "图片编码文件超过 16 MiB 上限",
            (Self::ImageDecodedLimit, Language::English) => "Image exceeds the decoding limits",
            (Self::ImageDecodedLimit, Language::Chinese) => "图片超出解码限制",
            (Self::ImageDecodeFailed, Language::English) => "Cannot decode image",
            (Self::ImageDecodeFailed, Language::Chinese) => "无法解码图片",
            (Self::ImagePreparing, Language::English) => "Preparing image…",
            (Self::ImagePreparing, Language::Chinese) => "正在准备图片…",
            (Self::ImageFormats, Language::English) => "PNG · JPEG · WebP · Fit to pane",
            (Self::ImageFormats, Language::Chinese) => "PNG · JPEG · WebP · 适应面板",

            (Self::BranchTitle, Language::English) => "Branches",
            (Self::BranchTitle, Language::Chinese) => "分支",
            (Self::BranchCatalogRead, Language::English) => "Read local branches",
            (Self::BranchCatalogRead, Language::Chinese) => "读取本地分支",
            (Self::BranchCommand, Language::English) => "Browse local and remote-tracking branches",
            (Self::BranchCommand, Language::Chinese) => "查看本地与远程跟踪分支",
            (Self::BranchTipTextInvalid, Language::English) => "Branch tip text is not valid UTF-8",
            (Self::BranchTipTextInvalid, Language::Chinese) => "分支最新提交文本不是有效 UTF-8",
            (Self::BranchAll, Language::English) => "All",
            (Self::BranchAll, Language::Chinese) => "全部",
            (Self::BranchLocal, Language::English) => "Local",
            (Self::BranchLocal, Language::Chinese) => "本地",
            (Self::BranchRemote, Language::English) => "Remote",
            (Self::BranchRemote, Language::Chinese) => "远程",
            (Self::BranchEmpty, Language::English) => "No matching branches",
            (Self::BranchEmpty, Language::Chinese) => "没有匹配的分支",
            (Self::BranchSearch, Language::English) => "Search branches",
            (Self::BranchSearch, Language::Chinese) => "搜索分支",
            (Self::BranchHint, Language::English) => {
                "↑↓ select · Enter details · /repo branch filter · Esc back"
            }
            (Self::BranchHint, Language::Chinese) => {
                "↑↓选择 · Enter 详情 · /repo branch filter · Esc 返回"
            }
            (Self::BranchReference, Language::English) => "Reference",
            (Self::BranchReference, Language::Chinese) => "引用",
            (Self::BranchLocalRefs, Language::English) => "Upstream comparison uses local refs",
            (Self::BranchLocalRefs, Language::Chinese) => "上游对比基于本地引用",

            (Self::WorkspaceOverview, Language::English) => "Overview",
            (Self::WorkspaceOverview, Language::Chinese) => "概览",
            (Self::WorkspaceHistory, Language::English) => "History",
            (Self::WorkspaceHistory, Language::Chinese) => "提交历史",
            (Self::WorkspaceFocusHint, Language::English) => "//focus",
            (Self::WorkspaceFocusHint, Language::Chinese) => "//focus",
            (Self::WorkspaceMaximizeHint, Language::English) => "//zoom Fullscreen",
            (Self::WorkspaceMaximizeHint, Language::Chinese) => "//zoom 放大",
            (Self::WorkspaceRestoreHint, Language::English) => "//zoom Restore",
            (Self::WorkspaceRestoreHint, Language::Chinese) => "//zoom 恢复",

            (Self::OverviewUpstream, Language::English) => "Upstream (local refs)",
            (Self::OverviewUpstream, Language::Chinese) => "上游（本地引用）",
            (Self::OverviewMailboxOrRebase, Language::English) => {
                "Applying mailbox patches or rebasing"
            }
            (Self::OverviewMailboxOrRebase, Language::Chinese) => "正在应用邮件补丁或变基",

            (Self::OverviewTitle, Language::English) => "Repository overview",
            (Self::OverviewTitle, Language::Chinese) => "仓库概览",
            (Self::OverviewLoading, Language::English) => "Reading local repository…",
            (Self::OverviewLoading, Language::Chinese) => "正在读取本地仓库…",
            (Self::OverviewSnapshot, Language::English) => "Local snapshot",
            (Self::OverviewSnapshot, Language::Chinese) => "本地快照",
            (Self::OverviewCheckout, Language::English) => "Current checkout",
            (Self::OverviewCheckout, Language::Chinese) => "当前检出",
            (Self::OverviewUnborn, Language::English) => "No commits yet",
            (Self::OverviewUnborn, Language::Chinese) => "尚无提交",
            (Self::OverviewDetached, Language::English) => "Detached HEAD",
            (Self::OverviewDetached, Language::Chinese) => "游离 HEAD",
            (Self::OverviewOperation, Language::English) => "Git operation",
            (Self::OverviewOperation, Language::Chinese) => "Git 操作",
            (Self::OverviewStaged, Language::English) => "Staged",
            (Self::OverviewStaged, Language::Chinese) => "已暂存",
            (Self::OverviewUnstaged, Language::English) => "Unstaged",
            (Self::OverviewUnstaged, Language::Chinese) => "未暂存",
            (Self::OverviewUntracked, Language::English) => "Untracked",
            (Self::OverviewUntracked, Language::Chinese) => "未跟踪",
            (Self::OverviewConflicts, Language::English) => "Conflicts",
            (Self::OverviewConflicts, Language::Chinese) => "冲突",
            (Self::OverviewShallow, Language::English) => {
                "Shallow history; ahead/behind may be incomplete"
            }
            (Self::OverviewShallow, Language::Chinese) => "浅克隆；领先与落后数量可能不完整",
            (Self::OverviewGone, Language::English) => "Upstream ref missing locally",
            (Self::OverviewGone, Language::Chinese) => "本地缺少上游引用",
            (Self::OverviewCheck, Language::English) => "Last remote check",
            (Self::OverviewCheck, Language::Chinese) => "上轮远程检查",
            (Self::RepositoryPanesHint, Language::English) => {
                "Arrows select pane · Enter focus · ← at left edge/Esc home · / commands"
            }
            (Self::RepositoryPanesHint, Language::Chinese) => {
                "方向键选择面板 · Enter 聚焦 · ←左边界/Esc 首页 · / 命令"
            }
            (Self::OverviewInputHint, Language::English) => {
                "↑↓ Scroll · ←/Esc Back · Ctrl+R Refresh · / Commands"
            }
            (Self::OverviewInputHint, Language::Chinese) => {
                "↑↓滚动 · ←/Esc 返回 · Ctrl+R 刷新 · / 命令"
            }
            (Self::OverviewMerging, Language::English) => "Merging",
            (Self::OverviewMerging, Language::Chinese) => "正在合并",
            (Self::OverviewRebasing, Language::English) => "Rebasing",
            (Self::OverviewRebasing, Language::Chinese) => "正在变基",
            (Self::OverviewReverting, Language::English) => "Reverting",
            (Self::OverviewReverting, Language::Chinese) => "正在还原提交",
            (Self::OverviewCherryPicking, Language::English) => "Cherry-picking",
            (Self::OverviewCherryPicking, Language::Chinese) => "正在拣选提交",
            (Self::OverviewBisecting, Language::English) => "Bisecting",
            (Self::OverviewBisecting, Language::Chinese) => "正在二分查找",
            (Self::OverviewApplyingMail, Language::English) => "Applying mailbox patches",
            (Self::OverviewApplyingMail, Language::Chinese) => "正在应用邮件补丁",

            (Self::GraphicsWorkerStopped, Language::English) => {
                "Pixel preparation worker stopped unexpectedly"
            }
            (Self::GraphicsWorkerStopped, Language::Chinese) => "像素绘制工作线程异常终止",
            (Self::GraphicsUnsupported, Language::English) => {
                "Pixel graphics unavailable: this terminal has no supported image protocol"
            }
            (Self::GraphicsUnsupported, Language::Chinese) => {
                "像素图不可用：当前终端不支持可用的图形协议"
            }
            (Self::GraphicsMetricsUnavailable, Language::English) => {
                "Pixel graphics unavailable: terminal cell size could not be measured"
            }
            (Self::GraphicsMetricsUnavailable, Language::Chinese) => {
                "像素图不可用：无法取得终端字符格的实际像素尺寸"
            }
            (Self::GraphicsTooLarge, Language::English) => {
                "Pixel graph exceeds the bounded image cache; reduce the window or font size"
            }
            (Self::GraphicsTooLarge, Language::Chinese) => {
                "像素图超过图像缓存上限，请缩小窗口或字体"
            }
            (Self::GraphicsPreparing, Language::English) => "Preparing graph",
            (Self::GraphicsPreparing, Language::Chinese) => "正在绘制提交图",
            (Self::HistoryGraftedParents, Language::English) => {
                "Git grafts changed parent relationships; this graph requires original commit ancestry"
            }
            (Self::HistoryGraftedParents, Language::Chinese) => {
                "Git grafts 改写了父提交关系，当前图要求使用提交的原始祖先关系"
            }
            (Self::HistoryNoFilesMatch, Language::English) => {
                "No historical files match the search"
            }
            (Self::HistoryNoFilesMatch, Language::Chinese) => "没有匹配的历史文件",
            (Self::HistoryShallow, Language::English) => {
                "Shallow repository · available local history only"
            }
            (Self::HistoryShallow, Language::Chinese) => "浅克隆仓库 · 仅显示本地可用历史",
            (Self::HistoryParentUnavailable, Language::English) => {
                "Parent commit object is unavailable; the comparison cannot be read"
            }
            (Self::HistoryParentUnavailable, Language::Chinese) => {
                "父提交对象不可用，无法读取此比较"
            }
            (Self::HistoryGraphTooWide, Language::English) => {
                "The commit graph exceeds 64 active lanes; this layout cannot display it"
            }
            (Self::HistoryGraphTooWide, Language::Chinese) => {
                "提交图超过 64 条活跃轨道，当前布局无法显示"
            }
            (Self::HistoryOpenCommitFirst, Language::English) => {
                "Open a commit before selecting a historical file diff"
            }
            (Self::HistoryOpenCommitFirst, Language::Chinese) => {
                "请先进入提交详情，再选择历史文件差异"
            }
            (Self::HistoryTitle, Language::English) => "Commit graph",
            (Self::HistoryTitle, Language::Chinese) => "提交历史图",
            (Self::HistoryCommand, Language::English) => {
                "Browse the commit graph of local branches, remote-tracking branches and tags"
            }
            (Self::HistoryCommand, Language::Chinese) => "浏览本地分支、远程跟踪分支和标签的提交图",
            (Self::HistoryMoreCommand, Language::English) => {
                "Load the next page of the current history snapshot"
            }
            (Self::HistoryMoreCommand, Language::Chinese) => "加载当前历史快照的下一页",
            (Self::HistoryRead, Language::English) => "Read commit history",
            (Self::HistoryRead, Language::Chinese) => "读取提交历史",
            (Self::HistoryReadCommit, Language::English) => "Read commit details",
            (Self::HistoryReadCommit, Language::Chinese) => "读取提交详情",
            (Self::HistoryReadDiff, Language::English) => "Read historical file diff",
            (Self::HistoryReadDiff, Language::Chinese) => "读取历史文件差异",
            (Self::HistoryEmpty, Language::English) => {
                "No commits in the captured branches or tags"
            }
            (Self::HistoryEmpty, Language::Chinese) => "当前分支和标签中没有提交",
            (Self::HistoryEnd, Language::English) => "End of history",
            (Self::HistoryEnd, Language::Chinese) => "已到历史末尾",
            (Self::HistoryMoreSearch, Language::English) => "more commits · /repo log more",
            (Self::HistoryMoreSearch, Language::Chinese) => "还有提交 · /repo log more",
            (Self::HistoryMore, Language::English) => {
                "More commits · Down at the bottom or /repo log more"
            }
            (Self::HistoryMore, Language::Chinese) => "还有提交 · 在末尾按 ↓ 或执行 /repo log more",
            (Self::HistoryLoaded, Language::English) => "Loaded",
            (Self::HistoryLoaded, Language::Chinese) => "已加载",
            (Self::HistoryScope, Language::English) => {
                "Local branches · Remote-tracking branches · Tags"
            }
            (Self::HistoryScope, Language::Chinese) => "本地分支 · 远程跟踪分支 · 标签",
            (Self::HistoryNoMore, Language::English) => "All commits in this snapshot are loaded",
            (Self::HistoryNoMore, Language::Chinese) => "当前快照的提交已全部加载",
            (Self::HistoryRequiresOpen, Language::English) => "Open /repo log first",
            (Self::HistoryRequiresOpen, Language::Chinese) => "请先打开 /repo log",
            (Self::HistorySearch, Language::English) => "Locate commits",
            (Self::HistorySearch, Language::Chinese) => "定位提交",
            (Self::HistorySearchHint, Language::English) => {
                "Type to locate loaded commits, or / for commands"
            }
            (Self::HistorySearchHint, Language::Chinese) => {
                "输入关键词定位已加载提交，或输入 / 查看命令"
            }
            (Self::HistoryInputHint, Language::English) => {
                "↑↓ select · ←→ scroll · Enter details · Esc back"
            }
            (Self::HistoryInputHint, Language::Chinese) => {
                "↑↓选择 · ←→ 横向滚动 · Enter详情 · Esc返回"
            }
            (Self::HistoryCommitInputHint, Language::English) => {
                "↑↓ files · Enter diff · /repo log parent · /focus panes · Esc back"
            }
            (Self::HistoryCommitInputHint, Language::Chinese) => {
                "↑↓文件 · Enter 差异 · /repo log parent · /focus 面板 · Esc 返回"
            }
            (Self::HistoryReviewInputHint, Language::English) => {
                "↑↓ scroll · ←→ pan · Enter file diff · /focus panes · Esc back"
            }
            (Self::HistoryReviewInputHint, Language::Chinese) => {
                "↑↓滚动 · ←→平移 · Enter 文件差异 · /focus 面板 · Esc 返回"
            }
            (Self::HistoryDiffHint, Language::English) => {
                "↑↓ scroll · ←→ pan · Esc commit · /repo diff layout"
            }
            (Self::HistoryDiffHint, Language::Chinese) => {
                "↑↓滚动 · ←→平移 · Esc提交详情 · /repo diff layout 布局"
            }
            (Self::HistoryNoMatch, Language::English) => {
                "No match in loaded commits; /repo log more loads older history"
            }
            (Self::HistoryNoMatch, Language::Chinese) => {
                "已加载提交中无匹配；/repo log more 可加载更早历史"
            }
            (Self::HistoryHash, Language::English) => "Commit",
            (Self::HistoryHash, Language::Chinese) => "提交",
            (Self::HistoryParents, Language::English) => "Parents",
            (Self::HistoryParents, Language::Chinese) => "父提交",
            (Self::HistoryAuthor, Language::English) => "Author",
            (Self::HistoryAuthor, Language::Chinese) => "作者",
            (Self::HistoryCommitter, Language::English) => "Committer",
            (Self::HistoryCommitter, Language::Chinese) => "提交者",
            (Self::HistoryAuthorTime, Language::English) => "Authored at",
            (Self::HistoryAuthorTime, Language::Chinese) => "作者时间",
            (Self::HistoryCommitterTime, Language::English) => "Committed at",
            (Self::HistoryCommitterTime, Language::Chinese) => "提交时间",
            (Self::HistoryTime, Language::English) => "Time",
            (Self::HistoryTime, Language::Chinese) => "时间",
            (Self::HistoryMessage, Language::English) => "Message",
            (Self::HistoryMessage, Language::Chinese) => "提交说明",
            (Self::HistoryRefs, Language::English) => "References",
            (Self::HistoryRefs, Language::Chinese) => "引用",
            (Self::HistoryChanges, Language::English) => "Changed files",
            (Self::HistoryChanges, Language::Chinese) => "变更文件",
            (Self::HistoryNoChanges, Language::English) => {
                "No file changes relative to this parent"
            }
            (Self::HistoryNoChanges, Language::Chinese) => "相对此父提交没有文件变更",
            (Self::HistoryRoot, Language::English) => "Empty tree (root commit)",
            (Self::HistoryRoot, Language::Chinese) => "空树（根提交）",
            (Self::HistoryCompare, Language::English) => "Comparison",
            (Self::HistoryCompare, Language::Chinese) => "比较",
            (Self::HistoryTruncated, Language::English) => {
                "Message exceeds 64 KiB; showing the beginning"
            }
            (Self::HistoryTruncated, Language::Chinese) => "提交说明超过 64 KiB，仅显示开头",
            (Self::HistoryUnborn, Language::English) => "Unborn branch",
            (Self::HistoryUnborn, Language::Chinese) => "尚无提交的分支",
            (Self::HistoryInvalidTime, Language::English) => "Invalid Git timestamp",
            (Self::HistoryInvalidTime, Language::Chinese) => "无效的 Git 时间",
            (Self::HistoryReferenceNotUtf8, Language::English) => "Git reference name is not UTF-8",
            (Self::HistoryReferenceNotUtf8, Language::Chinese) => "Git 引用名称不是 UTF-8",
            (Self::HistoryReferenceNotCommit, Language::English) => {
                "Branch does not point to a commit"
            }
            (Self::HistoryReferenceNotCommit, Language::Chinese) => "分支没有指向提交",
            (Self::HistoryCursorRepositoryChanged, Language::English) => {
                "History cursor belongs to a different repository"
            }
            (Self::HistoryCursorRepositoryChanged, Language::Chinese) => "历史游标属于另一个仓库",
            (Self::HistorySnapshotUnavailable, Language::English) => {
                "Captured history is no longer available; refresh the graph"
            }
            (Self::HistorySnapshotUnavailable, Language::Chinese) => {
                "历史快照已不可用，请刷新提交图"
            }
            (Self::HistoryTextNotUtf8, Language::English) => "Historical commit text is not UTF-8",
            (Self::HistoryTextNotUtf8, Language::Chinese) => "历史提交文本不是 UTF-8",
            (Self::HistoryParentNotInCommit, Language::English) => {
                "Comparison must use an actual parent of this commit"
            }
            (Self::HistoryParentNotInCommit, Language::Chinese) => "比较必须使用此提交的真实父提交",
            (Self::HistoryUnsupportedChange, Language::English) => {
                "Unsupported historical change kind"
            }
            (Self::HistoryUnsupportedChange, Language::Chinese) => "不支持的历史变更类型",
            (Self::DiffAuto, Language::English) => "Auto",
            (Self::DiffAuto, Language::Chinese) => "自动",
            (Self::DiffSplit, Language::English) => "Split",
            (Self::DiffSplit, Language::Chinese) => "双栏",
            (Self::DiffUnified, Language::English) => "Unified",
            (Self::DiffUnified, Language::Chinese) => "统一",
            (Self::DiffChooseAuto, Language::English) => "Choose diff layout by available width",
            (Self::DiffChooseAuto, Language::Chinese) => "根据可用宽度自动选择差异布局",
            (Self::DiffChooseSplit, Language::English) => {
                "Compare old and new versions side by side"
            }
            (Self::DiffChooseSplit, Language::Chinese) => "左右对比旧版与新版",
            (Self::FilesLayout, Language::English) => "File list display",
            (Self::FilesLayout, Language::Chinese) => "文件列表显示方式",
            (Self::FilesTree, Language::English) => "Directory tree",
            (Self::FilesTree, Language::Chinese) => "目录树",
            (Self::FilesFlat, Language::English) => "Flat list",
            (Self::FilesFlat, Language::Chinese) => "平铺列表",
            (Self::DiffChooseUnified, Language::English) => "Show changes in a single unified diff",
            (Self::DiffChooseUnified, Language::Chinese) => "在统一视图中查看差异",
            (Self::PartialSelectionIdle, Language::English) => "No change selected",
            (Self::PartialSelectionIdle, Language::Chinese) => "未选择改动",
            (Self::DiffContextCommand, Language::English) => "Choose full source or changes only",
            (Self::DiffContextCommand, Language::Chinese) => "选择完整源码或仅差异",
            (Self::DiffContextFull, Language::English) => "Full source",
            (Self::DiffContextFull, Language::Chinese) => "完整源码",
            (Self::DiffContextChanges, Language::English) => "Changes only",
            (Self::DiffContextChanges, Language::Chinese) => "仅差异",
            (Self::DiffFullUnavailable, Language::English) => {
                "Full source exceeds the 1 MiB read limit. Choose context changes to view the patch."
            }
            (Self::DiffFullUnavailable, Language::Chinese) => {
                "完整源码超过 1 MiB 读取上限。可切换为仅差异查看补丁。"
            }
            (Self::DiffFullLineLimit, Language::English) => {
                "Full source exceeds 50,000 lines. Choose context changes to view the patch."
            }
            (Self::DiffFullLineLimit, Language::Chinese) => {
                "完整源码超过 50,000 行。可切换为仅差异查看补丁。"
            }
            (Self::DiffFullTruncated, Language::English) => {
                "The patch is truncated, so a complete comparison cannot be displayed. Choose context changes to view the available patch."
            }
            (Self::DiffFullTruncated, Language::Chinese) => {
                "补丁已截断，无法展示完整对比。可切换为仅差异查看已读取的补丁。"
            }
            (Self::DiffHeaderCommand, Language::English) => {
                "Show or hide the original Git file header"
            }
            (Self::DiffHeaderCommand, Language::Chinese) => "选择是否显示原始 Git 文件头",
            (Self::DiffHeaderUnavailable, Language::English) => {
                "There is no file header in the current preview"
            }
            (Self::DiffHeaderUnavailable, Language::Chinese) => "当前预览没有可查看的文件头",
            (Self::DiffHeaderCollapsed, Language::English) => "Collapsed",
            (Self::DiffHeaderCollapsed, Language::Chinese) => "已收起",
            (Self::DiffHeaderExpanded, Language::English) => "Expanded",
            (Self::DiffHeaderExpanded, Language::Chinese) => "已展开",
            (Self::DiffHeaderAdded, Language::English) => "Added file",
            (Self::DiffHeaderAdded, Language::Chinese) => "新增文件",
            (Self::DiffHeaderDeleted, Language::English) => "Deleted file",
            (Self::DiffHeaderDeleted, Language::Chinese) => "删除文件",
            (Self::DiffHeaderMode, Language::English) => "File mode",
            (Self::DiffHeaderMode, Language::Chinese) => "文件模式",
            (Self::DiffHeaderRenamed, Language::English) => "Renamed",
            (Self::DiffHeaderRenamed, Language::Chinese) => "重命名",
            (Self::DiffHeaderCopied, Language::English) => "Copied",
            (Self::DiffHeaderCopied, Language::Chinese) => "复制",
            (Self::DiffHeaderSimilarity, Language::English) => "Similarity",
            (Self::DiffHeaderSimilarity, Language::Chinese) => "相似度",
            (Self::DiffHeaderDissimilarity, Language::English) => "Dissimilarity",
            (Self::DiffHeaderDissimilarity, Language::Chinese) => "差异度",
            (Self::DiffModeRegular, Language::English) => "Regular file",
            (Self::DiffModeRegular, Language::Chinese) => "普通文件",
            (Self::DiffModeExecutable, Language::English) => "Regular file, executable",
            (Self::DiffModeExecutable, Language::Chinese) => "普通文件，可执行",
            (Self::DiffModeSymlink, Language::English) => "Symbolic link",
            (Self::DiffModeSymlink, Language::Chinese) => "符号链接",
            (Self::DiffModeSubmodule, Language::English) => "Submodule",
            (Self::DiffModeSubmodule, Language::Chinese) => "子模块",
            (Self::DiffLayoutSelected, Language::English) => "Diff layout: {}",
            (Self::DiffLayoutSelected, Language::Chinese) => "差异布局：{}",
            (Self::DiffBefore, Language::English) => "Before · {}",
            (Self::DiffBefore, Language::Chinese) => "旧版 · {}",
            (Self::DiffAfter, Language::English) => "After · {}",
            (Self::DiffAfter, Language::Chinese) => "新版 · {}",
            (Self::DiffTooNarrow, Language::English) => {
                "This diff layout needs at least {} columns. Enlarge the window or use /repo diff layout auto."
            }
            (Self::DiffTooNarrow, Language::Chinese) => {
                "此差异布局至少需要 {} 列。请扩大窗口或使用 /repo diff layout auto。"
            }
            (Self::InvalidDiffStructure, Language::English) => {
                "Invalid diff structure returned by libgit2"
            }
            (Self::InvalidDiffStructure, Language::Chinese) => "libgit2 返回了无效的差异结构",
            (Self::DiffSourceChanged, Language::English) => {
                "Source does not match the diff; refresh the file list and reopen the diff"
            }
            (Self::DiffSourceChanged, Language::Chinese) => {
                "源码与差异不一致，请刷新文件列表后重新打开差异"
            }
            (Self::SyntaxLabel, Language::English) => "Syntax",
            (Self::SyntaxLabel, Language::Chinese) => "语法",
            (Self::SyntaxUnknown, Language::English) => "Unrecognized language · diff colors only",
            (Self::SyntaxUnknown, Language::Chinese) => "未识别语言 · 仅显示差异颜色",
            (Self::SyntaxSourceLimit, Language::English) => {
                "Source exceeds 1 MiB · diff colors only"
            }
            (Self::SyntaxSourceLimit, Language::Chinese) => "源码超过 1 MiB · 仅显示差异颜色",
            (Self::SyntaxLineLimit, Language::English) => {
                "Source line exceeds 16 KiB · diff colors only"
            }
            (Self::SyntaxLineLimit, Language::Chinese) => "源码单行超过 16 KiB · 仅显示差异颜色",
            (Self::SyntaxLineCountLimit, Language::English) => {
                "Source exceeds 50,000 lines · diff colors only"
            }
            (Self::SyntaxLineCountLimit, Language::Chinese) => {
                "源码超过 50,000 行 · 仅显示差异颜色"
            }
            (Self::SyntaxBinary, Language::English) => "Binary content · no syntax highlighting",
            (Self::SyntaxBinary, Language::Chinese) => "二进制内容 · 不进行语法高亮",
            (Self::SyntaxNonRegular, Language::English) => {
                "Non-regular file · no syntax highlighting"
            }
            (Self::SyntaxNonRegular, Language::Chinese) => "非普通文件 · 不进行语法高亮",
            (Self::SystemdPathControlUnsupported, Language::English) => {
                "Unsupported control character in systemd path"
            }
            (Self::SystemdPathControlUnsupported, Language::Chinese) => {
                "systemd 路径包含不支持的控制字符"
            }
            (Self::UnexpectedSystemdTimerState, Language::English) => {
                "Unexpected systemd timer state"
            }
            (Self::UnexpectedSystemdTimerState, Language::Chinese) => {
                "systemd 定时器返回了非预期状态"
            }
            (Self::XdgDirectoryMustBeAbsolute, Language::English) => {
                "XDG directory must be an absolute path"
            }
            (Self::XdgDirectoryMustBeAbsolute, Language::Chinese) => "XDG 目录必须是绝对路径",
            (Self::SyncColumn, Language::English) => "Sync",
            (Self::SyncColumn, Language::Chinese) => "同步",
            (Self::RepositoryErrorsFilter, Language::Chinese) => "需关注或失败",
            (Self::RepositoryErrorsFilter, Language::English) => "Needs attention or failed",
            (Self::HomeCheckComplete, Language::English) => "Fetch complete",
            (Self::HomeCheckComplete, Language::Chinese) => "获取完成",
            (Self::Synced, Language::English) => "Synced",
            (Self::Synced, Language::Chinese) => "已同步",
            (Self::SyncPushedCount, Language::English) => "Pushed {}",
            (Self::SyncPushedCount, Language::Chinese) => "已推送 {}",
            (Self::SyncPushFailedCount, Language::English) => "Push failed · {}",
            (Self::SyncPushFailedCount, Language::Chinese) => "推送失败 · {}",
            (Self::InvalidRepositoryCheckReport, Language::English) => {
                "Invalid single-repository check report"
            }
            (Self::InvalidRepositoryCheckReport, Language::Chinese) => "单仓库检查报告格式无效",

            (Self::CheckAlreadyRunning, Language::English) => {
                "A fetch is already running; use /cancel to cancel it"
            }
            (Self::CheckAlreadyRunning, Language::Chinese) => {
                "已有获取任务正在进行，可用 /cancel 取消"
            }
            (Self::CheckCancelRequested, Language::English) => {
                "Cancelling the background fetch; navigation remains available"
            }
            (Self::CheckCancelRequested, Language::Chinese) => "正在取消后台获取，仍可继续浏览",
            (Self::NoBackgroundCheck, Language::English) => "No background fetch is running",
            (Self::NoBackgroundCheck, Language::Chinese) => "当前没有后台获取任务",
            (Self::CheckWriteConflict, Language::English) => {
                "This write operation is unavailable during a check; wait or use /cancel"
            }
            (Self::CheckWriteConflict, Language::Chinese) => {
                "检查期间暂不能执行写操作，请等待或使用 /cancel"
            }
            (Self::BackgroundCheckProgress, Language::English) => " {} Fetching {}/{} · /cancel",
            (Self::BackgroundCheckProgress, Language::Chinese) => " {} 获取中 {}/{} · /cancel",
            (Self::BackgroundCheckCancelling, Language::English) => " {} Cancelling {}/{}",
            (Self::BackgroundCheckCancelling, Language::Chinese) => " {} 取消中 {}/{}",
            (Self::CancelBackgroundCheck, Language::English) => {
                "Cancel the selected repository task or the batch check"
            }
            (Self::CancelBackgroundCheck, Language::Chinese) => "取消当前仓库任务或整轮检查",
            (Self::CheckOneComplete, Language::English) => "Repository fetched: {}",
            (Self::CheckOneComplete, Language::Chinese) => "仓库远程获取完成：{}",
            (Self::CheckCancelledRetained, Language::English) => {
                "Check cancelled; completed results are retained"
            }
            (Self::CheckCancelledRetained, Language::Chinese) => "获取已取消，已完成仓库的结果保留",
            (Self::CheckUpdatedAt, Language::English) => "Result updated",
            (Self::CheckUpdatedAt, Language::Chinese) => "结果更新",
            (Self::CheckAtMostOnePath, Language::English) => {
                "check accepts at most one repository path"
            }
            (Self::CheckAtMostOnePath, Language::Chinese) => "check 最多接受一个仓库路径",
            (Self::SingleReportSaveFailed, Language::English) => {
                "Repository check completed, but its report could not be saved"
            }
            (Self::SingleReportSaveFailed, Language::Chinese) => "单仓库检查已完成，但无法保存报告",
            (Self::LinuxTerminalLauncherRequired, Language::English) => {
                "Install xdg-terminal-exec and configure your preferred terminal before opening repository terminals"
            }
            (Self::LinuxTerminalLauncherRequired, Language::Chinese) => {
                "请先安装 xdg-terminal-exec 并配置首选终端，再打开仓库终端"
            }
            (Self::ReadDefaultTerminalFailed, Language::English) => {
                "Unable to read the system default terminal application"
            }
            (Self::ReadDefaultTerminalFailed, Language::Chinese) => "无法读取系统默认终端应用",
            (Self::DefaultTerminalNotConfigured, Language::English) => {
                "No application is registered to open Unix executables; configure a default terminal in macOS"
            }
            (Self::DefaultTerminalNotConfigured, Language::Chinese) => {
                "系统未配置 Unix 可执行文件的默认打开应用，请在 macOS 中设置默认终端"
            }
            (Self::OpenRepositoryTerminal, Language::English) => {
                "Open default terminal in this repository"
            }
            (Self::OpenRepositoryTerminal, Language::Chinese) => "在当前目录打开终端",
            (Self::RepositoryTerminalOpened, Language::English) => "Opened terminal: {}",
            (Self::RepositoryTerminalOpened, Language::Chinese) => "已打开终端：{}",
            (Self::TerminalDirectoryMissing, Language::English) => {
                "Repository directory does not exist"
            }
            (Self::TerminalDirectoryMissing, Language::Chinese) => "仓库目录不存在",
            (Self::NoUpstreamColumn, Language::English) => "No up",
            (Self::NoUpstreamColumn, Language::Chinese) => "无上游",
            (Self::InstallationPathNotUtf8, Language::English) => "Installation path is not UTF-8",
            (Self::InstallationPathNotUtf8, Language::Chinese) => "安装路径不是 UTF-8",
            (Self::CommandPathNotUtf8, Language::English) => "Command path is not UTF-8",
            (Self::CommandPathNotUtf8, Language::Chinese) => "命令路径不是 UTF-8",
            (Self::FieldLabel, Language::English) => "{label}: ",
            (Self::FieldLabel, Language::Chinese) => "{label}：",
            (Self::CliHelp, Language::English) => {
                concat!(
                    identity::command!(),
                    ": Git repository status and configuration\n\nUsage: ",
                    identity::command!(),
                    " [--home PATH] [COMMAND]\nWithout a command, opens the current Git repository, or the repository list outside Git. Use ",
                    identity::command!(),
                    " status for status queries. Relative --home paths use the current working directory.\n\n  tui                              Open the current repository, or the repository list\n  repos                            Always open the repository list\n  status [--json]                   Show schedule, configuration and last result\n  report [--json] [FILTER]          Read the complete report\n    --problems                     Only attention or failed repositories\n    --no-upstream                  Branches without remote upstream\n    --pushes                       Automatic push results\n  init                             Create configuration\n  paths                            Show configuration, database and log paths\n  config                           Show configuration JSON\n  list                             List repositories\n  add PATH...                      Add repository roots\n  remove PATH                      Unregister a repository\n  scan PATH                        Scan repositories using Git ignore rules\n  import-tree PATH                 Scan and import repositories\n  interval 6h                      Save interval (s, m, h, d)\n  autopush disabled|fast_forward    Save automatic push policy\n  concurrency 4                    Set concurrency limit (1–32)\n  startup-fetch true|false          Fetch remotes on startup (default false)\n  scheduled-actions STEPS...        Save scheduled action order\n  fetch [PATH]                     Fetch all, or one registered repository\n  scheduled-check                  Run the saved scheduled workflow\n  schedule enable|disable|apply    Enable, disable or apply configuration\n  migrate config|--from PATH        Convert legacy configuration or copy a compatible custom home\n  install                          Install command and schedule\n  uninstall                        Remove scheduled task\n\nLanguage: follows system preferences; unsupported languages use English.\nSet APP_LANG=en or APP_LANG=zh to select a language explicitly.\n"
                )
            }
            (Self::CliHelp, Language::Chinese) => {
                concat!(
                    identity::command!(),
                    "：Git 仓库状态查询与配置管理\n\n用法：",
                    identity::command!(),
                    " [--home 路径] [命令]\n不传命令时打开当前 Git 仓库；非仓库目录进入仓库列表。状态查询使用 ",
                    identity::command!(),
                    " status。--home 的相对路径按启动时的当前目录解析。\n\n  tui                              打开当前仓库，非仓库目录进入仓库列表\n  repos                            直接打开仓库列表首页\n  status [--json]                   查看任务、配置和上次结果\n  report [--json] [筛选]             读取完整报告\n    --problems                      只看需关注或检查失败\n    --no-upstream                   单独查看无远程上游的分支\n    --pushes                        只看自动推送结果\n  init                              初始化配置\n  paths                             查看配置、数据库和日志位置\n  config                            查看配置 JSON\n  list                              查看仓库列表\n  add 路径...                       添加仓库根目录\n  remove 路径                       移除仓库注册项\n  scan 路径                         按 Git 忽略规则扫描仓库\n  import-tree 路径                  扫描并导入列表\n  interval 6h                       保存频率（支持 s、m、h、d）\n  autopush disabled|fast_forward     保存自动推送策略\n  concurrency 4                     设置并发上限（1 至 32）\n  startup-fetch true|false           启动时获取远程（默认 false）\n  scheduled-actions STEPS...         保存定时步骤顺序\n  fetch [路径]                      获取全部或单个仓库的远程信息\n  scheduled-check                   执行已保存的定时流程\n  schedule enable|disable|apply     启停定时任务或应用配置\n  migrate config|--from 路径          转换旧配置，或复制兼容的自定义数据目录\n  install                           安装命令并应用调度\n  uninstall                         卸载定时任务\n"
                )
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
            (Self::ConfigurationMissingRunInit, Language::English) => {
                concat!(
                    "Configuration is missing; run ",
                    identity::command!(),
                    " init or specify --home"
                )
            }
            (Self::ConfigurationMissingRunInit, Language::Chinese) => {
                concat!(
                    "配置不存在，请运行 ",
                    identity::command!(),
                    " init，或显式传入 --home"
                )
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
            (Self::ConfigurationSavedApplySchedule, Language::English) => {
                concat!(
                    "Configuration saved; run ",
                    identity::command!(),
                    " install to apply schedule changes"
                )
            }
            (Self::ConfigurationSavedApplySchedule, Language::Chinese) => {
                concat!(
                    "配置已保存；间隔修改需运行 ",
                    identity::command!(),
                    " install 应用调度"
                )
            }
            (Self::FoundRepositoriesFailedCouldNotBeRead, Language::English) => {
                "Found {} repositories; {failed} could not be read"
            }
            (Self::FoundRepositoriesFailedCouldNotBeRead, Language::Chinese) => {
                "发现 {} 个仓库，其中 {failed} 个无法读取"
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
            (Self::InstalledCommandAndSystemSchedule, Language::English) => {
                concat!(
                    "Installed native ",
                    identity::command!(),
                    " and system schedule; interval {}. No check started"
                )
            }
            (Self::InstalledCommandAndSystemSchedule, Language::Chinese) => {
                concat!(
                    "已安装原生 ",
                    identity::command!(),
                    " 命令与系统任务，检查间隔 {}；未触发检查"
                )
            }
            (Self::ScheduleRemovedConfigurationReportsAndCommandRetained, Language::English) => {
                "Scheduled task removed"
            }
            (Self::ScheduleRemovedConfigurationReportsAndCommandRetained, Language::Chinese) => {
                "定时任务已卸载"
            }
            (Self::UnknownCommandShowHelp, Language::English) => {
                concat!(
                    "Unknown command {command:?}; run ",
                    identity::command!(),
                    " --help"
                )
            }
            (Self::UnknownCommandShowHelp, Language::Chinese) => {
                concat!(
                    "未知命令 {command:?}；运行 ",
                    identity::command!(),
                    " --help 查看帮助"
                )
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
                "System schedule: {}\nCheck process: {}\nConfigured interval: {}\nRepositories: {}\nConcurrency: {}\nAutomatic push: {}\n"
            }
            (Self::SystemScheduleCheckProcessConfiguredIntervalEvery, Language::Chinese) => {
                "系统任务：{}\n检查进程：{}\n配置频率：{}\n仓库数量：{}\n检查并发：{}\n自动推送：{}\n"
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
            (Self::ScheduledIntervalEveryHours, Language::English) => "Scheduled interval: {}\n",
            (Self::ScheduledIntervalEveryHours, Language::Chinese) => "系统任务频率：{}\n",
            (Self::IntervalNotAppliedApplySchedule, Language::English) => {
                concat!(
                    "Interval not applied; run ",
                    identity::command!(),
                    " install\n"
                )
            }
            (Self::IntervalNotAppliedApplySchedule, Language::Chinese) => {
                concat!("频率尚未应用，请运行 ", identity::command!(), " install\n")
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
            (Self::SystemNotificationsFailedSeePushReport, Language::English) => {
                concat!(
                    "{} system notifications failed; see ",
                    identity::command!(),
                    " report --pushes\n"
                )
            }
            (Self::SystemNotificationsFailedSeePushReport, Language::Chinese) => {
                concat!(
                    "系统通知发送失败：{} 条，详情见 ",
                    identity::command!(),
                    " report --pushes\n"
                )
            }
            (Self::LatestReportDidNotPushScheduledPolicy, Language::English) => {
                "Latest report did not enable automatic push; the next scheduled check will use the saved policy\n"
            }
            (Self::LatestReportDidNotPushScheduledPolicy, Language::Chinese) => {
                "最近报告未启用自动推送；下次定时检查将应用已保存的推送策略\n"
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
            (Self::PushReportDetails, Language::English) => {
                concat!("{}\nDetails: ", identity::command!(), " report --pushes")
            }
            (Self::PushReportDetails, Language::Chinese) => {
                concat!("{}\n详情：", identity::command!(), " report --pushes")
            }
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
            (Self::Failed, Language::English) => "Failed",
            (Self::Failed, Language::Chinese) => "失败",
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
                "Interval requires a unit: s, m, h, d or w"
            }
            (Self::IntervalRequiresAUnitSMH, Language::Chinese) => {
                "间隔必须带 s、m、h、d 或 w 单位（秒、分钟、小时、天、周）"
            }
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
            (Self::InstalledCommandOwnershipMismatch, Language::English) => {
                concat!(
                    "Existing ",
                    identity::command!(),
                    " does not match the installation manifest; ownership cannot be verified"
                )
            }
            (Self::InstalledCommandOwnershipMismatch, Language::Chinese) => {
                concat!(
                    "现有 ",
                    identity::command!(),
                    " 命令与安装清单不匹配，无法确认归属"
                )
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
                "File {path} uses external filter {name:?} (such as Git LFS), which is not registered in this libgit2 index backend; staging refused. Handle it explicitly with Git"
            }
            (Self::FilePathUsesExternalFilterNameSuch, Language::Chinese) => {
                "文件 {path} 使用外部 filter {name:?}（例如 Git LFS），当前 libgit2 索引后端未注册该过滤器，已拒绝暂存；请使用 Git 显式处理"
            }
            (Self::FilePathUsesWorkingTreeEncodingWhich, Language::English) => {
                "File {path} uses working-tree-encoding, which this backend does not support; staging refused"
            }
            (Self::FilePathUsesWorkingTreeEncodingWhich, Language::Chinese) => {
                "文件 {path} 使用 working-tree-encoding，当前后端不支持该转换，已拒绝暂存"
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
            (Self::NewSymlink, Language::English) => "New symlink {} → {}",
            (Self::NewSymlink, Language::Chinese) => "新符号链接 {} → {}",
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

            (Self::ReloadReportOrFileList, Language::English) => "Reload report or file list",
            (Self::ReloadReportOrFileList, Language::Chinese) => "重新读取报告或文件列表",
            (Self::OpenSettings, Language::English) => {
                "Configure general settings such as concurrency"
            }
            (Self::OpenSettings, Language::Chinese) => "配置并发数等通用设置",
            (Self::AddARepositoryRoot, Language::English) => "Add a repository root",
            (Self::AddARepositoryRoot, Language::Chinese) => "添加仓库根目录",
            (Self::UnregisterRepositoryKeepFilesOnDisk, Language::English) => {
                "Unregister repository"
            }
            (Self::UnregisterRepositoryKeepFilesOnDisk, Language::Chinese) => "移除仓库注册项",
            (Self::ExcludeDirectoryAndUnregisterRepositoriesWithinIt, Language::English) => {
                "Exclude directory and unregister repositories within it"
            }
            (Self::ExcludeDirectoryAndUnregisterRepositoriesWithinIt, Language::Chinese) => {
                "排除目录并移除其下注册项"
            }
            (Self::RemoveDirectoryExclusionWithoutImporting, Language::English) => {
                "Remove directory exclusion"
            }
            (Self::RemoveDirectoryExclusionWithoutImporting, Language::Chinese) => "取消目录排除",
            (Self::PreviewScanUsingGitIgnoreRules, Language::English) => {
                "Scan and select repositories"
            }
            (Self::PreviewScanUsingGitIgnoreRules, Language::Chinese) => "扫描并选择仓库",
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
                "Apply system schedule"
            }
            (Self::ApplySystemScheduleWithoutStartingACheck, Language::Chinese) => "应用系统调度",
            (Self::ShowHelp, Language::English) => "Show help",
            (Self::ShowHelp, Language::Chinese) => "查看帮助",
            (Self::PressCtrlCAgainToQuit, Language::English) => {
                "Press Ctrl+C again to quit · Any other input continues"
            }
            (Self::PressCtrlCAgainToQuit, Language::Chinese) => {
                "再按一次 Ctrl+C 退出 · 其它操作继续使用"
            }
            (Self::Quit, Language::English) => "Quit",
            (Self::Quit, Language::Chinese) => "退出",
            (Self::InteractionCommand, Language::English) => {
                "Choose pointer interaction mode (Ctrl+G toggles)"
            }
            (Self::InteractionCommand, Language::Chinese) => "选择指针交互模式（Ctrl+G 切换）",
            (Self::InteractionApplicationCommand, Language::English) => {
                "Application handles clicks, scrolling and dragging"
            }
            (Self::InteractionApplicationCommand, Language::Chinese) => "程序处理点击、滚动和拖动",
            (Self::InteractionTextCommand, Language::English) => {
                "Terminal handles text selection, Copy and context menus"
            }
            (Self::InteractionTextCommand, Language::Chinese) => "终端处理选字、复制和右键菜单",
            (Self::InteractionApplication, Language::English) => "Interactive",
            (Self::InteractionApplication, Language::Chinese) => "交互模式",
            (Self::InteractionText, Language::English) => "Text selection",
            (Self::InteractionText, Language::Chinese) => "选字模式",
            (Self::InteractionSetting, Language::English) => "Pointer mode",
            (Self::InteractionSetting, Language::Chinese) => "指针模式",
            (Self::InteractionHint, Language::English) => {
                "Let the terminal handle text selection and context menus. Ctrl+G switches modes."
            }
            (Self::InteractionHint, Language::Chinese) => "由终端处理选字与右键菜单，Ctrl+G 切换。",
            (Self::InteractionSaveFailed, Language::English) => "Could not save interaction mode",
            (Self::InteractionSaveFailed, Language::Chinese) => "交互模式保存失败",
            (Self::ViewPreferenceSaveFailed, Language::English) => "Could not save view preference",
            (Self::ViewPreferenceSaveFailed, Language::Chinese) => "视图偏好保存失败",
            (Self::InteractiveTerminalRequired, Language::English) => {
                concat!(
                    identity::command!(),
                    " tui requires an interactive terminal; use ",
                    identity::command!(),
                    " status or ",
                    identity::command!(),
                    " report"
                )
            }
            (Self::InteractiveTerminalRequired, Language::Chinese) => {
                concat!(
                    identity::command!(),
                    " tui 需要交互式终端；请使用 ",
                    identity::command!(),
                    " status 或 ",
                    identity::command!(),
                    " report 查询"
                )
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
            (Self::CommittedRepositoryUpdated, Language::English) => {
                "Committed {}{}; repository state updated"
            }
            (Self::CommittedRepositoryUpdated, Language::Chinese) => "已提交 {}{}；仓库状态已更新",
            (Self::ContentsChangedVerifyTheCommit, Language::English) => {
                " (contents changed; verify the commit)"
            }
            (Self::ContentsChangedVerifyTheCommit, Language::Chinese) => {
                "（实际内容发生变化，请核对提交）"
            }
            (Self::PushedRepositoryUpdated, Language::English) => {
                "Pushed {} → {}/{}; repository state updated"
            }
            (Self::PushedRepositoryUpdated, Language::Chinese) => {
                "已推送 {} → {}/{}；仓库状态已更新"
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
            (Self::ApplySystemSchedule, Language::English) => "Apply system schedule",
            (Self::ApplySystemSchedule, Language::Chinese) => "应用系统调度",
            (Self::SystemScheduleAppliedNoCheckStarted, Language::English) => {
                "System schedule applied"
            }
            (Self::SystemScheduleAppliedNoCheckStarted, Language::Chinese) => "系统调度已应用",
            (Self::UnregisterRepository, Language::English) => "Unregister repository",
            (Self::UnregisterRepository, Language::Chinese) => "移除注册项",
            (Self::RepositoryUnregisteredFilesRetained, Language::English) => {
                "Repository unregistered"
            }
            (Self::RepositoryUnregisteredFilesRetained, Language::Chinese) => "已移除仓库注册项",
            (Self::AddedRepositories, Language::English) => "Added {} repositories",
            (Self::AddedRepositories, Language::Chinese) => "已添加 {} 个仓库",
            (Self::FoundRepositoriesScanSaved, Language::English) => "Found {} repositories",
            (Self::FoundRepositoriesScanSaved, Language::Chinese) => "发现 {} 个仓库",
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
            (Self::ScheduleOwnedByAnotherInstallation, Language::English) => {
                "The scheduled task belongs to another installation directory"
            }
            (Self::ScheduleOwnedByAnotherInstallation, Language::Chinese) => {
                "定时任务属于另一个安装目录"
            }
            (Self::ScheduleDefinitionMissing, Language::English) => {
                "Scheduled task definition is missing; enable the task first"
            }
            (Self::ScheduleDefinitionMissing, Language::Chinese) => {
                "定时任务定义不存在，请先启用任务"
            }
            (Self::EnableScheduledTask, Language::English) => "Enable scheduled task",
            (Self::EnableScheduledTask, Language::Chinese) => "启用定时任务",
            (Self::DisableScheduledTask, Language::English) => "Disable scheduled task",
            (Self::DisableScheduledTask, Language::Chinese) => "停用定时任务",
            (Self::InvalidScheduleOperation, Language::English) => {
                "Use schedule enable, disable or apply"
            }
            (Self::InvalidScheduleOperation, Language::Chinese) => {
                "schedule 仅支持 enable、disable 或 apply"
            }
            (Self::ScheduleEnabledNoCheck, Language::English) => "Scheduled task enabled",
            (Self::ScheduleEnabledNoCheck, Language::Chinese) => "定时任务已启用",
            (Self::ScheduleDisabledPreserved, Language::English) => "Scheduled task disabled",
            (Self::ScheduleDisabledPreserved, Language::Chinese) => "定时任务已停用",
            (Self::ScheduleAppliedPreserved, Language::English) => "Schedule configuration applied",
            (Self::ScheduleAppliedPreserved, Language::Chinese) => "调度配置已应用",
            (Self::ScheduleEnableConfirmation, Language::English) => {
                "Enable the scheduled task using the saved interval."
            }
            (Self::ScheduleEnableConfirmation, Language::Chinese) => "按已保存的间隔启用定时任务。",
            (Self::ScheduleDisableConfirmation, Language::English) => {
                "Disable future scheduled runs."
            }
            (Self::ScheduleDisableConfirmation, Language::Chinese) => "停用定时任务。",
            (Self::ScheduleApplyConfirmation, Language::English) => {
                "Apply the saved check interval."
            }
            (Self::ScheduleApplyConfirmation, Language::Chinese) => "应用已保存的检查间隔。",
            (Self::TaskStatusLabel, Language::English) => "Status",
            (Self::TaskStatusLabel, Language::Chinese) => "状态",
            (Self::CheckProcessLabel, Language::English) => "Checks",
            (Self::CheckProcessLabel, Language::Chinese) => "检查进程",
            (Self::TaskEnabled, Language::English) => "Enabled",
            (Self::TaskEnabled, Language::Chinese) => "已启用",
            (Self::TaskDisabled, Language::English) => "Disabled",
            (Self::TaskDisabled, Language::Chinese) => "已停用",
            (Self::TaskMissing, Language::English) => "Not installed",
            (Self::TaskMissing, Language::Chinese) => "未安装",
            (Self::IntervalLabel, Language::English) => "Interval",
            (Self::IntervalLabel, Language::Chinese) => "执行间隔",
            (Self::AutopushLabel, Language::English) => "Auto push",
            (Self::AutopushLabel, Language::Chinese) => "自动推送",
            (Self::ApplicationLabel, Language::English) => "Schedule config",
            (Self::ApplicationLabel, Language::Chinese) => "调度配置",
            (Self::IntervalApplied, Language::English) => "Applied",
            (Self::IntervalApplied, Language::Chinese) => "已应用",
            (Self::IntervalUninstalled, Language::English) => "Not installed",
            (Self::IntervalUninstalled, Language::Chinese) => "尚未安装",
            (Self::IntervalPending, Language::English) => "Pending · system: {}",
            (Self::IntervalPending, Language::Chinese) => "待应用 · 系统：{}",
            (Self::ScheduleActions, Language::English) => "Actions",
            (Self::ScheduleActions, Language::Chinese) => "操作",
            (Self::EditInterval, Language::English) => "Change interval",
            (Self::EditInterval, Language::Chinese) => "修改间隔",
            (Self::EditAutopush, Language::English) => "Change push policy",
            (Self::EditAutopush, Language::Chinese) => "修改推送策略",
            (Self::ScheduleHelpHint, Language::English) => {
                "↑↓ select · →/Enter open · ←/Esc back · Ctrl+R"
            }
            (Self::ScheduleHelpHint, Language::Chinese) => {
                "↑↓ 选择 · →/Enter 打开 · ←/Esc 返回 · Ctrl+R 刷新"
            }
            (Self::PolicyAppliesNextRun, Language::English) => {
                "Push policy is read on each scheduled run."
            }
            (Self::PolicyAppliesNextRun, Language::Chinese) => "推送策略在每次定时执行时读取。",
            (Self::DirectoryCandidates, Language::English) => "Directories",
            (Self::DirectoryCandidates, Language::Chinese) => "目录补全",
            (Self::DirectoryCompletionHint, Language::English) => {
                "↑↓ choose · Tab/Enter complete · Esc close"
            }
            (Self::DirectoryCompletionHint, Language::Chinese) => {
                "↑↓ 选择 · Tab/Enter 补全 · Esc 关闭"
            }
            (Self::ReadCompletionDirectory, Language::English) => "Cannot read directory: {}",
            (Self::ReadCompletionDirectory, Language::Chinese) => "无法读取目录：{}",
            (Self::NoMatchingDirectories, Language::English) => "No matching directories",
            (Self::NoMatchingDirectories, Language::Chinese) => "没有匹配目录",
            (Self::UneditableDirectoryName, Language::English) => {
                "Directory name is not UTF-8 text or contains control characters; cannot complete it"
            }
            (Self::UneditableDirectoryName, Language::Chinese) => {
                "目录名不是 UTF-8 文本或包含控制字符，无法补全"
            }
            (Self::UnsupportedNamedHome, Language::English) => {
                "Only ~ and ~/… are supported; named-user paths (~user) are not supported"
            }
            (Self::UnsupportedNamedHome, Language::Chinese) => {
                "仅支持 ~ 和 ~/…，不支持 ~user 用户路径"
            }
            (Self::ExpandedPathNotUtf8, Language::English) => "Expanded path is not UTF-8 text",
            (Self::ExpandedPathNotUtf8, Language::Chinese) => "展开后的路径不是 UTF-8 文本",
            (Self::MouseSettings, Language::English) => "Mouse & Trackpad",
            (Self::MouseSettings, Language::Chinese) => "鼠标与触控板",
            (Self::MousePreciseClick, Language::English) => {
                "Precise clicks · select items and seek scrollbars"
            }
            (Self::MousePreciseClick, Language::Chinese) => "精确点击 · 选择条目和滚动条定位",
            (Self::MouseScroll, Language::English) => "Wheel and trackpad scrolling",
            (Self::MouseScroll, Language::Chinese) => "滚轮与触控板滚动",
            (Self::MouseSelectPane, Language::English) => "Select pane under the pointer",
            (Self::MouseSelectPane, Language::Chinese) => "选择指针所在子视图",
            (Self::MouseSettingsSaved, Language::English) => "Mouse & Trackpad settings saved",
            (Self::MouseSettingsSaved, Language::Chinese) => "鼠标与触控板设置已保存",
            (Self::ChineseCommandTrigger, Language::English) => {
                "Use 、 to open commands in Chinese UI"
            }
            (Self::ChineseCommandTrigger, Language::Chinese) => "输入 、 打开命令菜单",
            (Self::ChineseCommandTriggerSaved, Language::English) => {
                "Chinese command trigger preference saved"
            }
            (Self::ChineseCommandTriggerSaved, Language::Chinese) => {
                "中文命令触发设置已保存，立即生效"
            }
            (Self::AutoPushDisabledOption, Language::English) => {
                "Do not push during scheduled checks"
            }
            (Self::AutoPushDisabledOption, Language::Chinese) => "定时检查时不自动推送",
            (Self::AutoPushFastForwardOption, Language::English) => {
                "Push branches that can fast-forward safely"
            }
            (Self::AutoPushFastForwardOption, Language::Chinese) => "自动推送可安全快进的分支",
            (Self::AgentCodexOption, Language::English) => "Use Codex from PATH",
            (Self::AgentCodexOption, Language::Chinese) => "使用 PATH 中的 Codex",
            (Self::AgentClaudeOption, Language::English) => "Use Claude Code from PATH",
            (Self::AgentClaudeOption, Language::Chinese) => "使用 PATH 中的 Claude Code",
            (Self::BooleanArgumentRequired, Language::English) => "Expected true or false",
            (Self::BooleanArgumentRequired, Language::Chinese) => "请输入 true 或 false",
            (Self::BrowserWrappedPreviewHint, Language::English) => {
                "↑↓ scroll · /focus panes · Esc files · / commands"
            }
            (Self::BrowserWrappedPreviewHint, Language::Chinese) => {
                "↑↓滚动 · /focus面板 · Esc文件 · /命令"
            }
            (Self::HistoryWrappedDiffHint, Language::English) => {
                "↑↓ scroll · Esc commit · /repo diff layout"
            }
            (Self::HistoryWrappedDiffHint, Language::Chinese) => {
                "↑↓滚动 · Esc提交详情 · /repo diff layout 布局"
            }
            (Self::WrappedChangesInputHint, Language::English) => {
                "↑↓ navigate · /focus panes · //zoom maximize · Esc back"
            }
            (Self::WrappedChangesInputHint, Language::Chinese) => {
                "↑↓选择/滚动 · /focus切换 · //zoom放大 · Esc返回"
            }
            (Self::WrappedDiffPreviewHint, Language::English) => {
                "↑↓ scroll · /focus files · Esc back · / commands"
            }
            (Self::WrappedDiffPreviewHint, Language::Chinese) => {
                "↑↓滚动 · /focus文件 · Esc返回 · /命令"
            }
            (Self::FileIcons, Language::English) => "File icons",
            (Self::FileIcons, Language::Chinese) => "文件图标",
            (Self::FileIconsHint, Language::English) => {
                "Show file-type icons using Nerd Font glyphs"
            }
            (Self::FileIconsHint, Language::Chinese) => "按文件类型显示图标，需 Nerd Font 字形支持",
            (Self::TextWrap, Language::English) => "Text wrapping",
            (Self::TextWrap, Language::Chinese) => "自动换行",
            (Self::TextWrapHint, Language::English) => {
                "Wrap file text and diff lines to the panel width"
            }
            (Self::TextWrapHint, Language::Chinese) => "文件文本与 diff 随面板宽度换行",
            (Self::TextWrapEnabled, Language::English) => "Wrap long lines",
            (Self::TextWrapEnabled, Language::Chinese) => "长行自动换行",
            (Self::TextWrapDisabled, Language::English) => "Scroll horizontally",
            (Self::TextWrapDisabled, Language::Chinese) => "使用横向滚动",
            (Self::SettingsDisplay, Language::English) => "Display",
            (Self::SettingsDisplay, Language::Chinese) => "显示",
            (Self::PathArgumentHint, Language::English) => {
                "Tab directories · Enter review · Esc clear"
            }
            (Self::PathArgumentHint, Language::Chinese) => "Tab 补全目录 · Enter 预览 · Esc 清空",
            (Self::DurationArgumentHint, Language::English) => {
                "Duration: 60s / 30m / 6h / 1d / 1w · Enter review · Esc clear"
            }
            (Self::DurationArgumentHint, Language::Chinese) => {
                "间隔：60s / 30m / 6h / 1d / 1w · Enter 预览 · Esc 清空"
            }
            (Self::PolicyArgumentHint, Language::English) => {
                "disabled | fast_forward · Enter review · Esc clear"
            }
            (Self::PolicyArgumentHint, Language::Chinese) => {
                "disabled | fast_forward · Enter 预览 · Esc 清空"
            }
            (Self::CountArgumentHint, Language::English) => {
                "Count: 1–32 · Enter review · Esc clear"
            }
            (Self::CountArgumentHint, Language::Chinese) => "并发数 1–32 · Enter 预览 · Esc 清空",
            (Self::MissingCommandArgument, Language::English) => "Required argument: {}",
            (Self::MissingCommandArgument, Language::Chinese) => "请填写参数：{}",
            (Self::UnclosedPathQuote, Language::English) => {
                "The quoted path must end with the same quote"
            }
            (Self::UnclosedPathQuote, Language::Chinese) => "路径两端的引号必须匹配",
            (Self::GeneralSettings, Language::English) => "General settings",
            (Self::GeneralSettings, Language::Chinese) => "通用设置",
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
                "Repository: {}\nBranch: {}\nRemote: {}\nUpstream: {}\nURL: {}\nPending: {} commits"
            }
            (Self::RepositoryBranchRemoteUpstreamUrlPendingCommits, Language::Chinese) => {
                "仓库：{}\n分支：{}\n远程：{}\n上游：{}\n地址：{}\n待推送：{} 个提交"
            }
            (Self::ConfirmPush, Language::English) => "Confirm push",
            (Self::ConfirmPush, Language::Chinese) => "确认推送",
            (Self::OperationResult, Language::English) => "Operation result",
            (Self::OperationResult, Language::Chinese) => "操作结果",
            (Self::ApplySavedIntervalAndReloadTheSystem, Language::English) => {
                "Apply the saved interval and reload the system schedule."
            }
            (Self::ApplySavedIntervalAndReloadTheSystem, Language::Chinese) => {
                "应用已保存的检查间隔，重新加载系统任务。"
            }
            (Self::UnregisterRepositoryFilesOnDiskAreRetained, Language::English) => {
                "Unregister repository: {}"
            }
            (Self::UnregisterRepositoryFilesOnDiskAreRetained, Language::Chinese) => {
                "移除仓库注册项：{}"
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
            (Self::BrowserTitle, Language::English) => "Files",
            (Self::BrowserTitle, Language::Chinese) => "文件浏览",
            (Self::BrowserDescription, Language::English) => {
                "Browse repository files and preview their contents"
            }
            (Self::BrowserDescription, Language::Chinese) => "浏览仓库文件并预览内容",
            (Self::BrowserDiffDescription, Language::English) => "Open the selected file's changes",
            (Self::BrowserDiffDescription, Language::Chinese) => "查看选中文件的差异",
            (Self::BrowserRefreshDescription, Language::English) => {
                "Refresh loaded directories and file preview"
            }
            (Self::BrowserRefreshDescription, Language::Chinese) => "刷新已加载目录与文件预览",
            (Self::BrowserSearch, Language::English) => "Find files",
            (Self::BrowserSearch, Language::Chinese) => "搜索文件",
            (Self::BrowserPlaceholder, Language::English) => {
                "Search repository paths, or / for commands"
            }
            (Self::BrowserPlaceholder, Language::Chinese) => "搜索仓库文件路径，或输入 / 查看命令",
            (Self::BrowserHint, Language::English) => {
                "↑↓ select · ←→ pan · Enter/Space open/toggle · /focus panes · //zoom maximize · Esc back"
            }
            (Self::BrowserHint, Language::Chinese) => {
                "↑↓选择 · ←→横移 · Enter/Space 打开/折叠 · /focus 面板 · /zoom 放大 · Esc 返回"
            }
            (Self::BrowserLoading, Language::English) => "Loading directory…",
            (Self::BrowserLoading, Language::Chinese) => "正在读取目录…",
            (Self::BrowserReading, Language::English) => "Loading file…",
            (Self::BrowserReading, Language::Chinese) => "正在读取文件…",
            (Self::BrowserSearching, Language::English) => "Searching repository files…",
            (Self::BrowserSearching, Language::Chinese) => "正在搜索仓库文件…",
            (Self::BrowserSearchLimited, Language::English) => {
                "Showing 1,000 matches; refine your search"
            }
            (Self::BrowserSearchLimited, Language::Chinese) => "已显示 1,000 项，请缩小搜索范围",
            (Self::BrowserSelectFile, Language::English) => "Select a file to preview",
            (Self::BrowserSelectFile, Language::Chinese) => "选择文件以预览内容",
            (Self::BrowserEmpty, Language::English) => "No files to display",
            (Self::BrowserEmpty, Language::Chinese) => "没有可显示的文件",
            (Self::BrowserInvalidPath, Language::English) => {
                "Browser paths must stay inside the repository without traversing symlinks"
            }
            (Self::BrowserInvalidPath, Language::Chinese) => {
                "浏览路径必须位于仓库内，且不能穿越符号链接目录"
            }
            (Self::BrowserSpecialFile, Language::English) => "Special files cannot be previewed",
            (Self::BrowserSpecialFile, Language::Chinese) => "特殊文件不支持内容预览",
            (Self::BrowserNotUtf8, Language::English) => "This file is not UTF-8 text",
            (Self::BrowserNotUtf8, Language::Chinese) => "该文件不是 UTF-8 文本",
            (Self::BrowserMissing, Language::English) => {
                "This tracked file is missing from the worktree; use /repo files diff to inspect its deletion"
            }
            (Self::BrowserMissing, Language::Chinese) => {
                "该跟踪文件已从工作区删除，可用 /repo files diff 查看删除差异"
            }
            (Self::BrowserBinary, Language::English) => "Binary file; text preview unavailable",
            (Self::BrowserBinary, Language::Chinese) => "二进制文件，不提供文本预览",
            (Self::BrowserTooLarge, Language::English) => {
                "Preview limit exceeded (text: 1 MiB; images: 16 MiB)"
            }
            (Self::BrowserTooLarge, Language::Chinese) => {
                "文件超过预览上限（文本 1 MiB，图片 16 MiB）"
            }
            (Self::BrowserDirectory, Language::English) => "Directory or submodule",
            (Self::BrowserDirectory, Language::Chinese) => "目录或子模块",
            (Self::BrowserSymlink, Language::English) => "Symlink",
            (Self::BrowserSymlink, Language::Chinese) => "符号链接",
            (Self::BrowserHighlightLimit, Language::English) => {
                "Syntax highlighting limit exceeded"
            }
            (Self::BrowserHighlightLimit, Language::Chinese) => "超过语法高亮处理上限",
            (Self::BrowserNoDiff, Language::English) => "Select a changed file to open its diff",
            (Self::BrowserNoDiff, Language::Chinese) => "请选择有改动的文件以查看差异",
            (Self::LinkOpen, Language::English) => "Open link",
            (Self::LinkOpen, Language::Chinese) => "打开链接",
            (Self::LinkOpening, Language::English) => "Opening link…",
            (Self::LinkOpening, Language::Chinese) => "正在打开链接…",
            (Self::LinkRequested, Language::English) => "Link sent to the system",
            (Self::LinkRequested, Language::Chinese) => "已请求系统打开链接",
            (Self::LinkOpenFailed, Language::English) => "Could not open link",
            (Self::LinkOpenFailed, Language::Chinese) => "打开链接失败",
            (Self::LinkPathMissing, Language::English) => "Path does not exist: {}",
            (Self::LinkPathMissing, Language::Chinese) => "路径不存在：{}",
            (Self::FileSource, Language::English) => "Source",
            (Self::FileSource, Language::Chinese) => "源码",
            (Self::FileMarkdownPreview, Language::English) => "Markdown preview",
            (Self::FileMarkdownPreview, Language::Chinese) => "Markdown 预览",
            (Self::FileSvgPreview, Language::English) => "Image preview",
            (Self::FileSvgPreview, Language::Chinese) => "图像预览",
            (Self::FileSingle, Language::English) => "Single",
            (Self::FileSingle, Language::Chinese) => "单栏",
            (Self::FileSplit, Language::English) => "Split",
            (Self::FileSplit, Language::Chinese) => "分屏",
            (Self::FileSourceCommand, Language::English) => "Open the selected file as source",
            (Self::FileSourceCommand, Language::Chinese) => "以源码打开当前文件",
            (Self::FileModeCommand, Language::English) => {
                "Choose source or preview for the selected file"
            }
            (Self::FileModeCommand, Language::Chinese) => "选择当前文件的源码或预览",
            (Self::FileLayoutCommand, Language::English) => {
                "Choose a single pane or source/preview split"
            }
            (Self::FileLayoutCommand, Language::Chinese) => "选择单栏或源码与预览分屏",
            (Self::ComposeReorderCommand, Language::English) => {
                "Move the selected planned commit earlier or later"
            }
            (Self::ComposeReorderCommand, Language::Chinese) => "将所选计划提交前移或后移",
            (Self::ScheduleEnabledCommand, Language::English) => {
                "Choose whether scheduled execution is enabled"
            }
            (Self::ScheduleEnabledCommand, Language::Chinese) => "选择是否启用定时执行",
            (Self::ComposeFirstStep, Language::English) => "Already the first planned commit",
            (Self::ComposeFirstStep, Language::Chinese) => "已经是第一条计划提交",
            (Self::ComposeLastStep, Language::English) => "Already the last planned commit",
            (Self::ComposeLastStep, Language::Chinese) => "已经是最后一条计划提交",
            (Self::FilePreviewCommand, Language::English) => {
                "Open the selected file's rendered preview"
            }
            (Self::FilePreviewCommand, Language::Chinese) => "打开当前文件的渲染预览",
            (Self::SvgInvalid, Language::English) => "SVG preview could not be rendered",
            (Self::SvgInvalid, Language::Chinese) => "无法渲染 SVG 预览",
            (Self::SvgExternal, Language::English) => {
                "SVG references external images; use a self-contained SVG or view its source"
            }
            (Self::SvgExternal, Language::Chinese) => {
                "SVG 引用了外部图片；请使用内嵌资源的 SVG，或切回源码查看"
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
            (Self::TuiHelp, Language::English) => include_str!("help/en.md"),
            (Self::TuiHelp, Language::Chinese) => include_str!("help/zh.md"),
            (Self::Repos, Language::English) => " {} repos",
            (Self::Repos, Language::Chinese) => " {}库",
            (Self::Search, Language::English) => " Search: {}",
            (Self::Search, Language::Chinese) => " 搜索：{}",
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
            (Self::NoRepositoriesMatchAddToAddOne, Language::English) => {
                "No repositories match; /repos add to add one"
            }
            (Self::NoRepositoriesMatchAddToAddOne, Language::Chinese) => {
                "当前筛选没有仓库；/repos add 添加仓库"
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
            (Self::RemoteRepositories, Language::English) => "Remotes",
            (Self::RemoteRepositories, Language::Chinese) => "远程仓库",
            (Self::RemotePush, Language::English) => "push",
            (Self::RemotePush, Language::Chinese) => "推送",
            (Self::RemoteNone, Language::English) => "Not configured",
            (Self::RemoteNone, Language::Chinese) => "未配置",
            (Self::RemoteUpstream, Language::English) => "Remote upstream",
            (Self::RemoteUpstream, Language::Chinese) => "远程上游",
            (Self::BranchSync, Language::English) => "Branch sync",
            (Self::BranchSync, Language::Chinese) => "分支同步",
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
                "Tab 补全 · ↑↓候选 · Enter 执行选中命令 · Esc 清空"
            }
            (Self::SearchEditingHint, Language::English) => {
                "←→ cursor · ↑↓ navigate · Esc clear · /commands"
            }
            (Self::SearchEditingHint, Language::Chinese) => {
                "←→ 移动光标 · ↑↓ 浏览 · Esc 清空 · / 命令"
            }
            (Self::SelectSpaceInsMarkCtrlAAll, Language::English) => {
                "→/Enter diff · ←/Esc back · Space/Ins mark · /focus panes"
            }
            (Self::SelectSpaceInsMarkCtrlAAll, Language::Chinese) => {
                "→/Enter 差异 · ←/Esc 返回 · Space/Ins 勾选 · /focus 面板"
            }
            (Self::ScrollEscFilesCommands, Language::English) => {
                "↑↓ scroll · ←/Esc files · /repo diff layout"
            }
            (Self::ScrollEscFilesCommands, Language::Chinese) => {
                "↑↓滚动 · ←/Esc 返回 · /repo diff layout 布局"
            }
            (Self::SelectEnterEditBackCommands, Language::English) => {
                "↑↓ select · →/Enter edit · ←/Esc back · /commands"
            }
            (Self::SelectEnterEditBackCommands, Language::Chinese) => {
                "↑↓ 选择 · →/Enter 修改 · ←/Esc 返回 · /命令"
            }
            (Self::PgdnScrollEscBackCommands, Language::English) => {
                "↑↓/PgDn scroll · ←/Esc back · /commands"
            }
            (Self::PgdnScrollEscBackCommands, Language::Chinese) => {
                "↑↓/PgDn 滚动 · ←/Esc 返回 · /命令"
            }
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
            (Self::CommitMessageAlt, Language::English) => " Commit message ",
            (Self::CommitMessageAlt, Language::Chinese) => " 提交说明 ",
            (Self::DescribeThisCommit, Language::English) => "Describe this commit",
            (Self::DescribeThisCommit, Language::Chinese) => "请输入本次提交的说明",
            (Self::EnterConfirmEscCancelPgupPgdnReview, Language::English) => {
                "Ctrl+J newline · Enter review · Esc cancel · PgUp/PgDn review"
            }
            (Self::EnterConfirmEscCancelPgupPgdnReview, Language::Chinese) => {
                "Ctrl+J 换行 · Enter 确认 · Esc 取消 · PgUp/PgDn 查看"
            }
            (Self::OpenRepositoryOverview, Language::English) => {
                "Open the selected repository overview"
            }
            (Self::OpenRepositoryOverview, Language::Chinese) => "打开选中仓库的概览面板",
            (Self::ManageRepositories, Language::English) => "Repository list and management",
            (Self::RepositoryManagementSettings, Language::English) => {
                "Manage repository registration and exclusions"
            }
            (Self::RepositoryManagementSettings, Language::Chinese) => "管理已添加的仓库",
            (Self::RefreshRepositoryReport, Language::English) => {
                "Reload the saved repository report"
            }
            (Self::RefreshRepositoryReport, Language::Chinese) => "重新读取仓库报告",
            (Self::ManageRepositories, Language::Chinese) => "仓库列表与管理",
            (Self::RepositoryManagement, Language::English) => "Repository management",
            (Self::RepositoryManagement, Language::Chinese) => "仓库管理",
            (Self::ConfigureSchedule, Language::English) => {
                "Configure interval, automatic push and system schedule"
            }
            (Self::ConfigureSchedule, Language::Chinese) => "配置执行间隔、自动推送和系统调度",
            (Self::ScheduleSettings, Language::English) => "Scheduled task",
            (Self::ScheduleSettings, Language::Chinese) => "定时任务",
            (Self::OverlappingFetchDestinations, Language::English) => {
                "Overlapping fetch destinations: {} ({}) and {} ({})"
            }
            (Self::OverlappingFetchDestinations, Language::Chinese) => {
                "Fetch 清理范围重叠：{}（{}）与 {}（{}）"
            }
            (Self::FetchRefspecNotUtf8, Language::English) => "Fetch refspec is not UTF-8",
            (Self::FetchRefspecNotUtf8, Language::Chinese) => "Fetch refspec 不是 UTF-8",
            (Self::InvalidFetchRefspec, Language::English) => "Invalid fetch refspec: {}",
            (Self::InvalidFetchRefspec, Language::Chinese) => "无效的 fetch refspec：{}",
            (Self::UpstreamExcludedFromFetch, Language::English) => {
                "Remote {} excludes upstream {} from fetch"
            }
            (Self::UpstreamExcludedFromFetch, Language::Chinese) => {
                "远程 {} 的 fetch 规则排除了上游 {}"
            }
            (Self::UnsupportedFetchDestination, Language::English) => {
                "Upstream fetch destination must be under refs/remotes/: {}"
            }
            (Self::UnsupportedFetchDestination, Language::Chinese) => {
                "上游 fetch 目标必须位于 refs/remotes/ 下：{}"
            }
            (Self::UpstreamFetchMappingNotUnique, Language::English) => {
                "Remote {} must have one fetch destination for upstream {}"
            }
            (Self::UpstreamFetchMappingNotUnique, Language::Chinese) => {
                "远程 {} 必须为上游 {} 配置唯一的 fetch 目标"
            }
            (Self::IncompleteBranchUpstream, Language::English) => {
                "Branch {} has incomplete upstream configuration"
            }
            (Self::IncompleteBranchUpstream, Language::Chinese) => "分支 {} 的上游配置不完整",
            (Self::UpstreamRemoteFetchSkipped, Language::English) => {
                "Upstream remote {} is excluded by skipFetchAll"
            }
            (Self::UpstreamRemoteFetchSkipped, Language::Chinese) => {
                "上游远程 {} 被 skipFetchAll 配置跳过"
            }
            (Self::ConfirmSelectHint, Language::English) => "←→ select · Enter confirm · Esc back",
            (Self::ConfirmSelectHint, Language::Chinese) => "←→选择 · Enter 确认 · Esc 返回",
            (Self::ConfirmReviewHint, Language::English) => {
                "←→ select · Enter confirm · Esc back · PgUp/PgDn review"
            }
            (Self::ConfirmReviewHint, Language::Chinese) => {
                "←→选择 · Enter 确认 · Esc 返回 · PgUp/PgDn 查看"
            }
            (Self::ActionCommit, Language::English) => "Commit",
            (Self::ActionCommit, Language::Chinese) => "提交",
            (Self::ActionPush, Language::English) => "Push",
            (Self::ActionPush, Language::Chinese) => "推送",
            (Self::ActionApply, Language::English) => "Apply schedule",
            (Self::ActionApply, Language::Chinese) => "应用调度",
            (Self::ActionUnregister, Language::English) => "Unregister",
            (Self::ActionUnregister, Language::Chinese) => "移除注册",
            (Self::ActionSave, Language::English) => "Save",
            (Self::ActionSave, Language::Chinese) => "保存",
            (Self::ActionAdd, Language::English) => "Add",
            (Self::ActionAdd, Language::Chinese) => "添加",
            (Self::ActionExclude, Language::English) => "Exclude",
            (Self::ActionExclude, Language::Chinese) => "排除",
            (Self::ActionUnexclude, Language::English) => "Remove exclusion",
            (Self::ActionUnexclude, Language::Chinese) => "取消排除",
            (Self::ActionScan, Language::English) => "Scan",
            (Self::ActionScan, Language::Chinese) => "扫描",
            (Self::ActionImport, Language::English) => "Import",
            (Self::ActionImport, Language::Chinese) => "导入",
            (Self::ActionReturnEdit, Language::English) => "Back to editing",
            (Self::ActionReturnEdit, Language::Chinese) => "返回编辑",
            (Self::ActionCancel, Language::English) => "Cancel",
            (Self::ActionCancel, Language::Chinese) => "取消",
            (Self::ConfirmOperationAlt, Language::English) => " Confirm operation ",
            (Self::ConfirmOperationAlt, Language::Chinese) => " 确认操作 ",
            (Self::ConfirmCommitAlt, Language::English) => " Confirm commit ",
            (Self::ConfirmCommitAlt, Language::Chinese) => " 确认提交 ",
            (Self::ConfirmPushAlt, Language::English) => " Confirm push ",
            (Self::ConfirmPushAlt, Language::Chinese) => " 确认推送 ",
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
            (Self::AutoPushFailureTitle, Language::English) => {
                concat!(identity::display_name!(), " automatic push failed")
            }
            (Self::AutoPushFailureTitle, Language::Chinese) => {
                concat!(identity::display_name!(), " 自动推送失败")
            }
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
    (ReadCommandHistoryFailed, $error:expr) => { match $crate::language() { $crate::Language::English => format!("Could not read command history: {}", $error), $crate::Language::Chinese => format!("读取命令历史失败：{}", $error) } };
    (SaveCommandHistoryFailed, $error:expr) => { match $crate::language() { $crate::Language::English => format!("Could not save command history: {}", $error), $crate::Language::Chinese => format!("保存命令历史失败：{}", $error) } };

    (ConflictBlockCount $(, $($args:tt)*)?) => { match $crate::language() { $crate::Language::English => format!("Conflict {current}/{total}" $(, $($args)*)?), $crate::Language::Chinese => format!("冲突块 {current}/{total}" $(, $($args)*)?), } };
    (ExternalEditorOpened $(, $($args:tt)*)?) => { match $crate::language() { $crate::Language::English => format!("Opened in editor: {0}" $(, $($args)*)?), $crate::Language::Chinese => format!("已用编辑器打开：{0}" $(, $($args)*)?), } };
    (ExternalEditorMissing $(, $($args:tt)*)?) => { match $crate::language() { $crate::Language::English => format!("Editor not found: {0}" $(, $($args)*)?), $crate::Language::Chinese => format!("未找到编辑器：{0}" $(, $($args)*)?), } };
    (LocalObservationFailed $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Local refresh failed · {0}: {1}" $(, $($args)*)?),
            $crate::Language::Chinese => format!("本地刷新失败 · {0}：{1}" $(, $($args)*)?),
        }
    };
    (StashSaveIncomplete $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Stash {oid} was created, but saving did not complete; inspect the workspace and index" $(, $($args)*)?),
            $crate::Language::Chinese => format!("已生成 stash {oid}，保存尚未完成，请查看工作区与暂存区的实际状态" $(, $($args)*)?),
        }
    };
    (StashFilesCount $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Files · {count}" $(, $($args)*)?),
            $crate::Language::Chinese => format!("文件 · {count}" $(, $($args)*)?),
        }
    };
    (DiscardFileInterrupted $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Discard stopped at {path} ({count}/{total} files completed)" $(, $($args)*)?),
            $crate::Language::Chinese => format!("丢弃在 {path} 处中止，已处理 {count}/{total} 个文件" $(, $($args)*)?),
        }
    };
    (CommandTargetFiles $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("{count} files" $(, $($args)*)?),
            $crate::Language::Chinese => format!("{count} 个文件" $(, $($args)*)?),
        }
    };
    (CommandTargetLines $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("{count} lines" $(, $($args)*)?),
            $crate::Language::Chinese => format!("{count} 行" $(, $($args)*)?),
        }
    };
    (CommandTargetCommits $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("{count} commits" $(, $($args)*)?),
            $crate::Language::Chinese => format!("{count} 个提交" $(, $($args)*)?),
        }
    };
    (ReplayStats $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("{} files · +{} −{}" $(, $($args)*)?),
            $crate::Language::Chinese => format!("{} 个文件 · +{} −{}" $(, $($args)*)?),
        }
    };
    (SyncPullReview $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Branch: {branch}\nUpstream: {upstream}\nSync: ↑0 ↓{commits}\n\nAction: fast-forward pull {commits} commits\nTarget commit: {oid}" $(, $($args)*)?),
            $crate::Language::Chinese => format!("分支：{branch}\n上游：{upstream}\n同步：↑0 ↓{commits}\n\n执行：快进拉取 {commits} 个提交\n目标提交：{oid}" $(, $($args)*)?),
        }
    };
    (SyncPushReview $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Branch: {branch}\nUpstream: {remote}/{reference}\nSync: ↑{commits} ↓0\n\nAction: push {commits} commits\nTarget commit: {oid}" $(, $($args)*)?),
            $crate::Language::Chinese => format!("分支：{branch}\n上游：{remote}/{reference}\n同步：↑{commits} ↓0\n\n执行：推送 {commits} 个提交\n目标提交：{oid}" $(, $($args)*)?),
        }
    };
    (SyncPulled $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Synchronized: pulled {branch}\nCommit: {oid}" $(, $($args)*)?),
            $crate::Language::Chinese => format!("同步完成：已拉取 {branch}\n提交：{oid}" $(, $($args)*)?),
        }
    };
    (HistoryActionsCount $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Selected commits: {count}" $(, $($args)*)?),
            $crate::Language::Chinese => format!("已选 {count} 条提交" $(, $($args)*)?),
        }
    };
    (SyncPushed $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Synchronized: pushed {commits} commits from {branch}\nRemote: {remote}/{reference}\nCommit: {oid}" $(, $($args)*)?),
            $crate::Language::Chinese => format!("同步完成：已推送 {branch} 的 {commits} 个提交\n远程：{remote}/{reference}\n提交：{oid}" $(, $($args)*)?),
        }
    };
    (WorkflowUnknownStep, $token:expr) => {
        match $crate::language() {
            $crate::Language::English => format!("Unknown step: {}; use fetch, pull, push, inspect.", $token),
            $crate::Language::Chinese => format!("未知步骤：{}；可用 fetch、pull、push、inspect。", $token),
        }
    };
    (UnknownOptionValue $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Unknown value: {value}. Allowed values: {choices}" $(, $($args)*)?),
            $crate::Language::Chinese => format!("未知参数：{value}。可选值：{choices}" $(, $($args)*)?),
        }
    };
    (SearchInputTitle $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("{mode} {target}" $(, $($args)*)?),
            $crate::Language::Chinese => format!("{mode}{target}" $(, $($args)*)?),
        }
    };
    (TaskFailuresCount $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Unresolved failures: {pending}" $(, $($args)*)?),
            $crate::Language::Chinese => format!("{pending} 未处理失败" $(, $($args)*)?),
        }
    };
    (TaskRunningCount $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Running: {running}" $(, $($args)*)?),
            $crate::Language::Chinese => format!("{running} 运行中" $(, $($args)*)?),
        }
    };
    (TaskCounts $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Unresolved failures: {pending} · Running: {running}" $(, $($args)*)?),
            $crate::Language::Chinese => format!("{pending} 未处理失败 · {running} 运行中" $(, $($args)*)?),
        }
    };
    (TasksSummary $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Unresolved failures: {pending} · Running: {running} · /tasks" $(, $($args)*)?),
            $crate::Language::Chinese => format!("{pending} 未处理失败 · {running} 运行中 · /tasks 任务" $(, $($args)*)?),
        }
    };
    (LinkPathMissing $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Path does not exist: {}" $(, $($args)*)?),
            $crate::Language::Chinese => format!("路径不存在：{}" $(, $($args)*)?),
        }
    };
    (PullReview $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Fast-forward {0} commits to {1}\nUpstream: {2}" $(, $($args)*)?),
            $crate::Language::Chinese => format!("快进 {0} 个提交至 {1}\n上游：{2}" $(, $($args)*)?),
        }
    };
    (RepositoryTasksRunning $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("  {0} · {1} repository tasks" $(, $($args)*)?),
            $crate::Language::Chinese => format!("  {0} · {1} 个仓库任务" $(, $($args)*)?),
        }
    };
    (RepositoryTaskCompleted $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("{0} · Completed: {1}" $(, $($args)*)?),
            $crate::Language::Chinese => format!("{0} · 已完成：{1}" $(, $($args)*)?),
        }
    };

    (DiffLayoutSelected $(, $($args:tt)*)?) => { match $crate::language() {
        $crate::Language::English => format!("Diff layout: {}" $(, $($args)*)?),
        $crate::Language::Chinese => format!("差异布局：{}" $(, $($args)*)?),
    } };
    (DiffBefore $(, $($args:tt)*)?) => { match $crate::language() {
        $crate::Language::English => format!("Before · {}" $(, $($args)*)?),
        $crate::Language::Chinese => format!("旧版 · {}" $(, $($args)*)?),
    } };
    (DiffAfter $(, $($args:tt)*)?) => { match $crate::language() {
        $crate::Language::English => format!("After · {}" $(, $($args)*)?),
        $crate::Language::Chinese => format!("新版 · {}" $(, $($args)*)?),
    } };
    (DiffTooNarrow $(, $($args:tt)*)?) => { match $crate::language() {
        $crate::Language::English => format!("This diff layout needs at least {} columns. Enlarge the window or use /repo diff layout auto." $(, $($args)*)?),
        $crate::Language::Chinese => format!("此差异布局至少需要 {} 列。请扩大窗口或使用 /repo diff layout auto。" $(, $($args)*)?),
    } };
    (ReadCompletionDirectory $(, $($args:tt)*)?) => { match $crate::language() { $crate::Language::English => format!("Cannot read directory: {}" $(, $($args)*)?), $crate::Language::Chinese => format!("无法读取目录：{}" $(, $($args)*)?), } };
    (MissingCommandArgument $(, $($args:tt)*)?) => { match $crate::language() { $crate::Language::English => format!("Required argument: {}" $(, $($args)*)?), $crate::Language::Chinese => format!("请填写参数：{}" $(, $($args)*)?), } };
    (IntervalPending $(, $($args:tt)*)?) => { match $crate::language() { $crate::Language::English => format!("Pending · system: {}" $(, $($args)*)?), $crate::Language::Chinese => format!("待应用 · 系统：{}" $(, $($args)*)?), } };
    (OverlappingFetchDestinations $(, $($args:tt)*)?) => { match $crate::language() {
        $crate::Language::English => format!("Overlapping fetch destinations: {} ({}) and {} ({})" $(, $($args)*)?),
        $crate::Language::Chinese => format!("Fetch 清理范围重叠：{}（{}）与 {}（{}）" $(, $($args)*)?),
    } };

    (InvalidFetchRefspec $(, $($args:tt)*)?) => { match $crate::language() { $crate::Language::English => format!("Invalid fetch refspec: {}" $(, $($args)*)?), $crate::Language::Chinese => format!("无效的 fetch refspec：{}" $(, $($args)*)?), } };
    (UpstreamExcludedFromFetch $(, $($args:tt)*)?) => { match $crate::language() { $crate::Language::English => format!("Remote {} excludes upstream {} from fetch" $(, $($args)*)?), $crate::Language::Chinese => format!("远程 {} 的 fetch 规则排除了上游 {}" $(, $($args)*)?), } };
    (UnsupportedFetchDestination $(, $($args:tt)*)?) => { match $crate::language() { $crate::Language::English => format!("Upstream fetch destination must be under refs/remotes/: {}" $(, $($args)*)?), $crate::Language::Chinese => format!("上游 fetch 目标必须位于 refs/remotes/ 下：{}" $(, $($args)*)?), } };
    (UpstreamFetchMappingNotUnique $(, $($args:tt)*)?) => { match $crate::language() { $crate::Language::English => format!("Remote {} must have one fetch destination for upstream {}" $(, $($args)*)?), $crate::Language::Chinese => format!("远程 {} 必须为上游 {} 配置唯一的 fetch 目标" $(, $($args)*)?), } };
    (IncompleteBranchUpstream $(, $($args:tt)*)?) => { match $crate::language() { $crate::Language::English => format!("Branch {} has incomplete upstream configuration" $(, $($args)*)?), $crate::Language::Chinese => format!("分支 {} 的上游配置不完整" $(, $($args)*)?), } };
    (UpstreamRemoteFetchSkipped $(, $($args:tt)*)?) => { match $crate::language() { $crate::Language::English => format!("Upstream remote {} is excluded by skipFetchAll" $(, $($args)*)?), $crate::Language::Chinese => format!("上游远程 {} 被 skipFetchAll 配置跳过" $(, $($args)*)?), } };
    (CheckOneComplete $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Repository fetched: {}" $(, $($args)*)?),
            $crate::Language::Chinese => format!("仓库远程获取完成：{}" $(, $($args)*)?),
        }
    };
    (BackgroundCheckCancelling $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!(" {} Cancelling {}/{}" $(, $($args)*)?),
            $crate::Language::Chinese => format!(" {} 取消中 {}/{}" $(, $($args)*)?),
        }
    };
    (BackgroundCheckProgress $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!(" {} Fetching {}/{} · /cancel" $(, $($args)*)?),
            $crate::Language::Chinese => format!(" {} 获取中 {}/{} · /cancel" $(, $($args)*)?),
        }
    };
    (SyncPushFailedCount $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Push failed · {}" $(, $($args)*)?),
            $crate::Language::Chinese => format!("推送失败 · {}" $(, $($args)*)?),
        }
    };
    (SyncPushedCount $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Pushed {}" $(, $($args)*)?),
            $crate::Language::Chinese => format!("已推送 {}" $(, $($args)*)?),
        }
    };
    (RepositoryTerminalOpened $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Opened terminal: {}" $(, $($args)*)?),
            $crate::Language::Chinese => format!("已打开终端：{}" $(, $($args)*)?),
        }
    };
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
            $crate::Language::English => format!("Found {} repositories; {failed} could not be read" $(, $($args)*)?),
            $crate::Language::Chinese => format!("发现 {} 个仓库，其中 {failed} 个无法读取" $(, $($args)*)?),
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
    (InstalledCommandAndSystemSchedule $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!(concat!("Installed native ", $crate::__identity::command!(), " and system schedule; interval {}. No check started") $(, $($args)*)?),
            $crate::Language::Chinese => format!(concat!("已安装原生 ", $crate::__identity::command!(), " 命令与系统任务，检查间隔 {}；未触发检查") $(, $($args)*)?),
        }
    };
    (UnknownCommandShowHelp $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!(concat!("Unknown command {command:?}; run ", $crate::__identity::command!(), " --help") $(, $($args)*)?),
            $crate::Language::Chinese => format!(concat!("未知命令 {command:?}；运行 ", $crate::__identity::command!(), " --help 查看帮助") $(, $($args)*)?),
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
            $crate::Language::English => format!("System schedule: {}\nCheck process: {}\nConfigured interval: {}\nRepositories: {}\nConcurrency: {}\nAutomatic push: {}\n" $(, $($args)*)?),
            $crate::Language::Chinese => format!("系统任务：{}\n检查进程：{}\n配置频率：{}\n仓库数量：{}\n检查并发：{}\n自动推送：{}\n" $(, $($args)*)?),
        }
    };
    (ScheduledIntervalEveryHours $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Scheduled interval: {}\n" $(, $($args)*)?),
            $crate::Language::Chinese => format!("系统任务频率：{}\n" $(, $($args)*)?),
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
    (SystemNotificationsFailedSeePushReport $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!(concat!("{} system notifications failed; see ", $crate::__identity::command!(), " report --pushes\n") $(, $($args)*)?),
            $crate::Language::Chinese => format!(concat!("系统通知发送失败：{} 条，详情见 ", $crate::__identity::command!(), " report --pushes\n") $(, $($args)*)?),
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
    (PushReportDetails $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!(concat!("{}\nDetails: ", $crate::__identity::command!(), " report --pushes") $(, $($args)*)?),
            $crate::Language::Chinese => format!(concat!("{}\n详情：", $crate::__identity::command!(), " report --pushes") $(, $($args)*)?),
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
            $crate::Language::English => format!("File {path} uses external filter {name:?} (such as Git LFS), which is not registered in this libgit2 index backend; staging refused. Handle it explicitly with Git" $(, $($args)*)?),
            $crate::Language::Chinese => format!("文件 {path} 使用外部 filter {name:?}（例如 Git LFS），当前 libgit2 索引后端未注册该过滤器，已拒绝暂存；请使用 Git 显式处理" $(, $($args)*)?),
        }
    };
    (FilePathUsesWorkingTreeEncodingWhich $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("File {path} uses working-tree-encoding, which this backend does not support; staging refused" $(, $($args)*)?),
            $crate::Language::Chinese => format!("文件 {path} 使用 working-tree-encoding，当前后端不支持该转换，已拒绝暂存" $(, $($args)*)?),
        }
    };
    (NewSymlink $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("New symlink {} → {}" $(, $($args)*)?),
            $crate::Language::Chinese => format!("新符号链接 {} → {}" $(, $($args)*)?),
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
    (CommittedRepositoryUpdated $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Committed {}{}; repository state updated" $(, $($args)*)?),
            $crate::Language::Chinese => format!("已提交 {}{}；仓库状态已更新" $(, $($args)*)?),
        }
    };
    (PushedRepositoryUpdated $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Pushed {} → {}/{}; repository state updated" $(, $($args)*)?),
            $crate::Language::Chinese => format!("已推送 {} → {}/{}；仓库状态已更新" $(, $($args)*)?),
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
    (FoundRepositoriesScanSaved $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Found {} repositories" $(, $($args)*)?),
            $crate::Language::Chinese => format!("发现 {} 个仓库" $(, $($args)*)?),
        }
    };
    (RepositoryBranchScopeStagedContentsOnlyStaged $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Repository: {}\nBranch: {}\nScope: staged contents only\n\nStaged files\n" $(, $($args)*)?),
            $crate::Language::Chinese => format!("仓库：{}\n分支：{}\n范围：仅已暂存内容\n\n暂存文件\n" $(, $($args)*)?),
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
            $crate::Language::English => format!("Repository: {}\nBranch: {}\nRemote: {}\nUpstream: {}\nURL: {}\nPending: {} commits" $(, $($args)*)?),
            $crate::Language::Chinese => format!("仓库：{}\n分支：{}\n远程：{}\n上游：{}\n地址：{}\n待推送：{} 个提交" $(, $($args)*)?),
        }
    };
    (UnregisterRepositoryFilesOnDiskAreRetained $(, $($args:tt)*)?) => {
        match $crate::language() {
            $crate::Language::English => format!("Unregister repository: {}" $(, $($args)*)?),
            $crate::Language::Chinese => format!("移除仓库注册项：{}" $(, $($args)*)?),
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
