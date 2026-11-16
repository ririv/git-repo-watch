//! Fixed option values belong to a parameter catalog rather than executable commands.
use super::{Action, Command, Input};
use crate::{
    Editor, Screen,
    diff::Preference,
    files::Layout,
    repository_list::{Filter, Order},
};
use engine::{ai::Agent, domain::PushPolicy, state::InteractionMode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    ConflictAccept,
    ConflictBlock,
    Conflict,
    StashPreview,
    StashSave,
    DiffSelection,
    BranchFilter,
    Filter,
    Sort,
    FileView,
    FileMode,
    FileLayout,
    DiffHeader,
    ComposeOrder,
    ScheduleEnabled,
    DiffLayout,
    DiffContext,
    Search,
    Interaction,
    AutoFetch,
    StartupFetch,
    TextWrap,
    Autopush,
    AiAgent,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Value {
    ConflictAccept(engine::local::files::conflict::resolve::Choice),
    ConflictBlock(crate::conflict::resolution::Direction),
    Conflict(crate::conflict::DisplayMode),
    StashPreview(engine::local::stash::preview::Scope),
    StashSave(crate::stash_save::Scope),
    DiffSelection(crate::changes::edit::Granularity),
    BranchFilter(crate::branches::Filter),
    Filter(Filter),
    Sort(Order),
    FileView(Layout),
    FileMode(crate::browser::opening::TextMode),
    FileLayout(crate::browser::opening::Layout),
    DiffHeader(crate::diff::HeaderVisibility),
    ComposeOrder(crate::composition::Direction),
    ScheduleEnabled(engine::scheduler::Registration),
    DiffLayout(Preference),
    DiffContext(crate::diff::Context),
    Search(crate::search::Mode),
    Interaction(InteractionMode),
    AutoFetch(bool),
    StartupFetch(bool),
    TextWrap(bool),
    Autopush(PushPolicy),
    AiAgent(Agent),
}
#[derive(Debug)]
pub(crate) struct Option {
    pub value: Value,
    pub token: &'static str,
    pub description: i18n::Message,
}
use i18n::Message as M;
const OPTIONS: &[Option] = &[
    Option {
        value: Value::ConflictAccept(engine::local::files::conflict::resolve::Choice::Left),
        token: "left",
        description: M::ConflictAcceptLeft,
    },
    Option {
        value: Value::ConflictAccept(engine::local::files::conflict::resolve::Choice::Right),
        token: "right",
        description: M::ConflictAcceptRight,
    },
    Option {
        value: Value::ConflictAccept(engine::local::files::conflict::resolve::Choice::Both),
        token: "both",
        description: M::ConflictAcceptBoth,
    },
    Option {
        value: Value::ConflictBlock(crate::conflict::resolution::Direction::Previous),
        token: "previous",
        description: M::ConflictBlockPrevious,
    },
    Option {
        value: Value::ConflictBlock(crate::conflict::resolution::Direction::Next),
        token: "next",
        description: M::ConflictBlockNext,
    },
    Option {
        value: Value::Conflict(crate::conflict::DisplayMode::Overview),
        token: "overview",
        description: M::ConflictOverview,
    },
    Option {
        value: Value::Conflict(crate::conflict::DisplayMode::IncludeBase),
        token: "include-base",
        description: M::ConflictIncludeBase,
    },
    Option {
        value: Value::Conflict(crate::conflict::DisplayMode::Pair(
            crate::conflict::pair::Pair::BaseCurrent,
        )),
        token: "base-left",
        description: M::ConflictCompareCurrent,
    },
    Option {
        value: Value::Conflict(crate::conflict::DisplayMode::Pair(
            crate::conflict::pair::Pair::BaseIncoming,
        )),
        token: "base-right",
        description: M::ConflictCompareIncoming,
    },
    Option {
        value: Value::Conflict(crate::conflict::DisplayMode::Pair(
            crate::conflict::pair::Pair::BaseResult,
        )),
        token: "base-result",
        description: M::ConflictCompareResult,
    },
    Option {
        value: Value::Conflict(crate::conflict::DisplayMode::Pair(
            crate::conflict::pair::Pair::CurrentIncoming,
        )),
        token: "left-right",
        description: M::ConflictCompareSides,
    },
    Option {
        value: Value::Conflict(crate::conflict::DisplayMode::Pair(
            crate::conflict::pair::Pair::CurrentResult,
        )),
        token: "left-result",
        description: M::ConflictCompareLeftResult,
    },
    Option {
        value: Value::Conflict(crate::conflict::DisplayMode::Pair(
            crate::conflict::pair::Pair::ResultIncoming,
        )),
        token: "result-right",
        description: M::ConflictCompareResultRight,
    },
    Option {
        value: Value::BranchFilter(crate::branches::Filter::All),
        token: "all",
        description: M::BranchAll,
    },
    Option {
        value: Value::BranchFilter(crate::branches::Filter::Local),
        token: "local",
        description: M::BranchLocal,
    },
    Option {
        value: Value::BranchFilter(crate::branches::Filter::Remote),
        token: "remote",
        description: M::BranchRemote,
    },
    Option {
        value: Value::StashPreview(engine::local::stash::preview::Scope::All),
        token: "all",
        description: M::StashPreviewAll,
    },
    Option {
        value: Value::StashPreview(engine::local::stash::preview::Scope::Staged),
        token: "staged",
        description: M::StashPreviewStaged,
    },
    Option {
        value: Value::StashPreview(engine::local::stash::preview::Scope::Unstaged),
        token: "unstaged",
        description: M::StashPreviewUnstaged,
    },
    Option {
        value: Value::StashSave(crate::stash_save::Scope::Tracked),
        token: "tracked",
        description: M::StashScopeTracked,
    },
    Option {
        value: Value::StashSave(crate::stash_save::Scope::All),
        token: "all",
        description: M::StashScopeAll,
    },
    Option {
        value: Value::StashSave(crate::stash_save::Scope::Selected),
        token: "selected",
        description: M::StashScopeSelected,
    },
    Option {
        value: Value::FileLayout(crate::browser::opening::Layout::Single),
        token: "single",
        description: M::FileSingle,
    },
    Option {
        value: Value::FileLayout(crate::browser::opening::Layout::Split),
        token: "split",
        description: M::FileSplit,
    },
    Option {
        value: Value::DiffHeader(crate::diff::HeaderVisibility::Expanded),
        token: "show",
        description: M::DiffHeaderExpanded,
    },
    Option {
        value: Value::DiffHeader(crate::diff::HeaderVisibility::Collapsed),
        token: "hide",
        description: M::DiffHeaderCollapsed,
    },
    Option {
        value: Value::ComposeOrder(crate::composition::Direction::Up),
        token: "up",
        description: M::ComposeUp,
    },
    Option {
        value: Value::ComposeOrder(crate::composition::Direction::Down),
        token: "down",
        description: M::ComposeDown,
    },
    Option {
        value: Value::ScheduleEnabled(engine::scheduler::Registration::Loaded),
        token: "true",
        description: M::EnableScheduledTask,
    },
    Option {
        value: Value::ScheduleEnabled(engine::scheduler::Registration::NotLoaded),
        token: "false",
        description: M::DisableScheduledTask,
    },
    Option {
        value: Value::FileMode(crate::browser::opening::TextMode::Source),
        token: "source",
        description: M::FileSourceCommand,
    },
    Option {
        value: Value::FileMode(crate::browser::opening::TextMode::Preview),
        token: "preview",
        description: M::FilePreviewCommand,
    },
    Option {
        value: Value::DiffSelection(crate::changes::edit::Granularity::Hunk),
        token: "hunk",
        description: M::PartialHunk,
    },
    Option {
        value: Value::DiffSelection(crate::changes::edit::Granularity::Line),
        token: "line",
        description: M::PartialLine,
    },
    Option {
        value: Value::AutoFetch(true),
        token: "true",
        description: M::AutoFetchEnabled,
    },
    Option {
        value: Value::AutoFetch(false),
        token: "false",
        description: M::AutoFetchDisabled,
    },
    Option {
        value: Value::StartupFetch(true),
        token: "true",
        description: M::StartupFetchEnabled,
    },
    Option {
        value: Value::StartupFetch(false),
        token: "false",
        description: M::StartupFetchDisabled,
    },
    Option {
        value: Value::TextWrap(true),
        token: "true",
        description: M::TextWrapEnabled,
    },
    Option {
        value: Value::TextWrap(false),
        token: "false",
        description: M::TextWrapDisabled,
    },
    Option {
        value: Value::Filter(Filter::All),
        token: "all",
        description: M::RepositoryFilterAll,
    },
    Option {
        value: Value::Filter(Filter::Problems),
        token: "problems",
        description: M::RepositoryFilterProblems,
    },
    Option {
        value: Value::Filter(Filter::Clean),
        token: "clean",
        description: M::RepositoryFilterClean,
    },
    Option {
        value: Value::Filter(Filter::Failed),
        token: "failed",
        description: M::RepositoryFilterFailed,
    },
    Option {
        value: Value::Filter(Filter::Unchecked),
        token: "unchecked",
        description: M::RepositoryFilterUnchecked,
    },
    Option {
        value: Value::Filter(Filter::WithoutUpstream),
        token: "no-upstream",
        description: M::RepositoryFilterNoUpstream,
    },
    Option {
        value: Value::Filter(Filter::Pushes),
        token: "pushes",
        description: M::RepositoryFilterPushes,
    },
    Option {
        value: Value::Sort(Order::Priority),
        token: "priority",
        description: M::RepositorySortPriority,
    },
    Option {
        value: Value::Sort(Order::Name),
        token: "name",
        description: M::RepositorySortName,
    },
    Option {
        value: Value::Sort(Order::Path),
        token: "path",
        description: M::RepositorySortPath,
    },
    Option {
        value: Value::Sort(Order::Updated),
        token: "updated",
        description: M::RepositorySortUpdated,
    },
    Option {
        value: Value::FileView(Layout::Tree),
        token: "tree",
        description: M::FilesTree,
    },
    Option {
        value: Value::FileView(Layout::Flat),
        token: "flat",
        description: M::FilesFlat,
    },
    Option {
        value: Value::DiffContext(crate::diff::Context::Full),
        token: "full",
        description: M::DiffContextFull,
    },
    Option {
        value: Value::DiffContext(crate::diff::Context::Changes),
        token: "changes",
        description: M::DiffContextChanges,
    },
    Option {
        value: Value::DiffLayout(Preference::Auto),
        token: "auto",
        description: M::DiffChooseAuto,
    },
    Option {
        value: Value::DiffLayout(Preference::Split),
        token: "split",
        description: M::DiffChooseSplit,
    },
    Option {
        value: Value::DiffLayout(Preference::Unified),
        token: "unified",
        description: M::DiffChooseUnified,
    },
    Option {
        value: Value::Search(crate::search::Mode::Filter),
        token: "filter",
        description: M::SearchFilterCommand,
    },
    Option {
        value: Value::Search(crate::search::Mode::Locate),
        token: "locate",
        description: M::SearchLocateCommand,
    },
    Option {
        value: Value::Interaction(InteractionMode::Application),
        token: "interactive",
        description: M::InteractionApplicationCommand,
    },
    Option {
        value: Value::Interaction(InteractionMode::TextSelection),
        token: "text",
        description: M::InteractionTextCommand,
    },
    Option {
        value: Value::Autopush(PushPolicy::Disabled),
        token: "disabled",
        description: M::AutoPushDisabledOption,
    },
    Option {
        value: Value::Autopush(PushPolicy::FastForward),
        token: "fast_forward",
        description: M::AutoPushFastForwardOption,
    },
    Option {
        value: Value::AiAgent(Agent::Codex),
        token: "codex",
        description: M::AgentCodexOption,
    },
    Option {
        value: Value::AiAgent(Agent::Claude),
        token: "claude",
        description: M::AgentClaudeOption,
    },
];
impl Value {
    pub fn action(self) -> Action {
        match self {
            Self::StashPreview(scope) => Action::StashPreview(scope),
            Self::Conflict(value) => Action::ConflictView(value),
            Self::ConflictAccept(choice) => Action::ConflictAccept(choice),
            Self::ConflictBlock(direction) => Action::ConflictBlock(direction),
            Self::StashSave(scope) => Action::StashSaveMode(scope),
            Self::DiffSelection(mode) => Action::PartialSelect(mode),
            Self::BranchFilter(value) => Action::BranchFilter(value),
            Self::Filter(value) => Action::Filter(value),
            Self::Sort(value) => Action::Sort(value),
            Self::FileView(value) => Action::FileLayout(value),
            Self::FileMode(value) => Action::BrowserMode(value),
            Self::FileLayout(value) => Action::BrowserLayout(value),
            Self::DiffHeader(value) => Action::DiffHeaderVisibility(value),
            Self::ComposeOrder(value) => Action::Compose(value.action()),
            Self::ScheduleEnabled(engine::scheduler::Registration::Loaded) => {
                Action::EnableSchedule
            }
            Self::ScheduleEnabled(engine::scheduler::Registration::NotLoaded) => {
                Action::DisableSchedule
            }
            Self::DiffLayout(value) => Action::DiffLayout(value),
            Self::DiffContext(value) => Action::DiffContext(value),
            Self::Search(value) => Action::Search(value),
            Self::Interaction(value) => Action::Interaction(value),
            Self::AutoFetch(value) => Action::AutoFetch(value),
            Self::StartupFetch(value) => Action::StartupFetch(value),
            Self::TextWrap(value) => Action::Wrap(value),
            Self::Autopush(_) => Action::Autopush,
            Self::AiAgent(_) => Action::AiAgent,
        }
    }
    fn kind(self) -> Kind {
        match self {
            Self::StashPreview(_) => Kind::StashPreview,
            Self::Conflict(_) => Kind::Conflict,
            Self::ConflictAccept(_) => Kind::ConflictAccept,
            Self::ConflictBlock(_) => Kind::ConflictBlock,
            Self::StashSave(_) => Kind::StashSave,
            Self::DiffSelection(_) => Kind::DiffSelection,
            Self::BranchFilter(_) => Kind::BranchFilter,
            Self::Filter(_) => Kind::Filter,
            Self::Sort(_) => Kind::Sort,
            Self::FileView(_) => Kind::FileView,
            Self::FileMode(_) => Kind::FileMode,
            Self::FileLayout(_) => Kind::FileLayout,
            Self::DiffHeader(_) => Kind::DiffHeader,
            Self::ComposeOrder(_) => Kind::ComposeOrder,
            Self::ScheduleEnabled(_) => Kind::ScheduleEnabled,
            Self::DiffLayout(_) => Kind::DiffLayout,
            Self::DiffContext(_) => Kind::DiffContext,
            Self::Search(_) => Kind::Search,
            Self::Interaction(_) => Kind::Interaction,
            Self::AutoFetch(_) => Kind::AutoFetch,
            Self::StartupFetch(_) => Kind::StartupFetch,
            Self::TextWrap(_) => Kind::TextWrap,
            Self::Autopush(_) => Kind::Autopush,
            Self::AiAgent(_) => Kind::AiAgent,
        }
    }
}
impl Kind {
    pub fn action(self) -> Action {
        match self {
            Self::StashPreview => Action::StashPreviewMenu,
            Self::Conflict => Action::ConflictMenu,
            Self::ConflictAccept => Action::ConflictAcceptMenu,
            Self::ConflictBlock => Action::ConflictBlockMenu,
            Self::StashSave => Action::StashSave,
            Self::DiffSelection => Action::PartialSelectMenu,
            Self::BranchFilter => Action::BranchFilterMenu,
            Self::Filter => Action::FilterMenu,
            Self::Sort => Action::SortMenu,
            Self::FileView => Action::FileLayoutMenu,
            Self::FileMode => Action::BrowserModeMenu,
            Self::FileLayout => Action::BrowserLayoutMenu,
            Self::DiffHeader => Action::DiffHeader,
            Self::ComposeOrder => Action::ComposeOrderMenu,
            Self::ScheduleEnabled => Action::ScheduleEnabledMenu,
            Self::DiffLayout => Action::DiffLayoutMenu,
            Self::DiffContext => Action::DiffContextMenu,
            Self::Search => Action::SearchMenu,
            Self::Interaction => Action::InteractionMenu,
            Self::AutoFetch => Action::AutoFetchMenu,
            Self::StartupFetch => Action::StartupFetchMenu,
            Self::TextWrap => Action::WrapMenu,
            Self::Autopush => Action::Autopush,
            Self::AiAgent => Action::AiAgent,
        }
    }
    pub fn command(self) -> &'static Command {
        super::COMMANDS
            .iter()
            .find(|command| command.action == self.action())
            .expect("option command catalog")
    }
    pub fn current(self, screen: &Screen) -> std::option::Option<Value> {
        Some(match self {
            Self::StashPreview => {
                let inspect = screen.mode.inspection()?;
                let crate::inspection::Source::Stash(detail) = &inspect.detail else {
                    return None;
                };
                Value::StashPreview(detail.scope)
            }
            Self::DiffSelection => Value::DiffSelection(screen.partial_mode()?),
            Self::ConflictAccept | Self::ConflictBlock => return None,
            Self::ComposeOrder | Self::StashSave => return None,
            Self::DiffHeader => Value::DiffHeader(screen.diff_header),
            Self::ScheduleEnabled => Value::ScheduleEnabled(screen.snapshot.registration),
            Self::FileLayout => {
                Value::FileLayout(screen.bar.selection()?.opener()?.presentation.layout())
            }
            Self::BranchFilter => match &screen.mode {
                crate::Mode::Branches(view) => Value::BranchFilter(view.session().filter),
                _ => return None,
            },
            Self::Conflict => match &screen.mode {
                crate::Mode::Changes(session) => match &session.preview {
                    crate::changes::Preview::Ready { content, .. } => match content.as_ref() {
                        crate::document::View::Conflict(view) => Value::Conflict(view.mode),
                        _ => return None,
                    },
                    _ => return None,
                },
                _ => return None,
            },
            Self::Filter => Value::Filter(screen.filter),
            Self::Sort => Value::Sort(screen.order),
            Self::FileView => Value::FileView(screen.file_layout),
            Self::FileMode => match screen.bar.selection()?.opener()?.presentation {
                crate::browser::opening::Presentation::Single(mode) => Value::FileMode(mode),
                crate::browser::opening::Presentation::Split(_) => return None,
            },
            Self::DiffLayout => Value::DiffLayout(screen.diff_preference),
            Self::DiffContext => Value::DiffContext(screen.diff_context),
            Self::Search => Value::Search(screen.search_support().mode()?),
            Self::Interaction => Value::Interaction(screen.config.interaction_mode),
            Self::AutoFetch => Value::AutoFetch(screen.config.auto_fetch.enabled),
            Self::StartupFetch => Value::StartupFetch(screen.config.fetch_on_startup),
            Self::TextWrap => Value::TextWrap(screen.config.text_wrap),
            Self::Autopush => Value::Autopush(screen.config.scheduled_actions.push_policy()),
            Self::AiAgent => Value::AiAgent(screen.config.ai.agent),
        })
    }
    pub fn parse(self, raw: &str) -> anyhow::Result<Value> {
        self.options()
            .find(|option| option.token.eq_ignore_ascii_case(raw.trim()))
            .map(|option| option.value)
            .ok_or_else(|| {
                anyhow::anyhow!(i18n::tf!(
                    UnknownOptionValue,
                    value = raw.trim(),
                    choices = self
                        .options()
                        .map(|option| option.token)
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
            })
    }
    pub fn options(self) -> impl Iterator<Item = &'static Option> {
        OPTIONS
            .iter()
            .filter(move |option| option.value.kind() == self)
    }
}
pub(crate) fn description(action: Action) -> std::option::Option<&'static str> {
    if action.argument().is_some() {
        return None;
    }
    OPTIONS
        .iter()
        .find(|option| option.value.action() == action)
        .map(|option| option.description.text())
}

#[derive(Debug)]
pub(crate) struct Candidates {
    command: String,
    pub options: Vec<&'static Option>,
    pub current: std::option::Option<Value>,
    pub selected: usize,
}
impl Candidates {
    pub fn new(
        input: &Input,
        kind: Kind,
        screen: &Screen,
        path: std::option::Option<&std::path::Path>,
        selection: &crate::clipboard::Selection,
    ) -> std::option::Option<Self> {
        let editor = input.canonical_editor();
        let (_, raw) = super::argument_input(editor.text.strip_prefix('/')?)?;
        let start = editor.text.len() - raw.len();
        let needle = raw
            .get(..editor.cursor.checked_sub(start)?)?
            .trim()
            .to_lowercase();
        let mut options = kind
            .options()
            .filter(|option| {
                screen.command_visible(option.value.action(), path, selection)
                    && (option.token.starts_with(&needle)
                        || [i18n::Language::Chinese, i18n::Language::English]
                            .iter()
                            .any(|language| {
                                option
                                    .description
                                    .in_language(*language)
                                    .to_lowercase()
                                    .contains(&needle)
                            }))
            })
            .collect::<Vec<_>>();
        options.sort_by_key(|option| {
            if option.token == needle {
                0
            } else if option.token.starts_with(&needle) {
                1
            } else {
                2
            }
        });
        let current = kind.current(screen);
        let selected = if needle.is_empty() {
            options
                .iter()
                .position(|option| Some(option.value) == current)
                .unwrap_or(0)
        } else {
            0
        };
        Some(Self {
            command: editor.text[..start].into(),
            options,
            current,
            selected,
        })
    }
    pub fn advance(&mut self, backwards: bool) {
        if !self.options.is_empty() {
            self.selected = (self.selected + if backwards { self.options.len() - 1 } else { 1 })
                % self.options.len();
        }
    }
    pub fn choose(&self) -> std::option::Option<Editor> {
        Some(Editor::new(format!(
            "{}{}",
            self.command,
            self.options.get(self.selected)?.token
        )))
    }
}
