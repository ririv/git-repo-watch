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
    LogSelection,
    Filter,
    Sort,
    FileView,
    DiffLayout,
    Search,
    Interaction,
    StartupCheck,
    TextWrap,
    Autopush,
    AiAgent,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Value {
    LogSelection(crate::history::SelectionAction),
    Filter(Filter),
    Sort(Order),
    FileView(Layout),
    DiffLayout(Preference),
    Search(crate::search::Mode),
    Interaction(InteractionMode),
    StartupCheck(bool),
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
        value: Value::LogSelection(crate::history::SelectionAction::Start),
        token: "start",
        description: M::HistorySelect,
    },
    Option {
        value: Value::LogSelection(crate::history::SelectionAction::Toggle),
        token: "toggle",
        description: M::HistorySelectToggle,
    },
    Option {
        value: Value::LogSelection(crate::history::SelectionAction::All),
        token: "all",
        description: M::HistorySelectAll,
    },
    Option {
        value: Value::LogSelection(crate::history::SelectionAction::Clear),
        token: "clear",
        description: M::HistorySelectClear,
    },
    Option {
        value: Value::LogSelection(crate::history::SelectionAction::Done),
        token: "done",
        description: M::HistorySelectDone,
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
        value: Value::StartupCheck(true),
        token: "true",
        description: M::StartupCheckEnabledOption,
    },
    Option {
        value: Value::StartupCheck(false),
        token: "false",
        description: M::StartupCheckDisabledOption,
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
            Self::LogSelection(value) => Action::LogSelection(value),
            Self::Filter(value) => Action::Filter(value),
            Self::Sort(value) => Action::Sort(value),
            Self::FileView(value) => Action::FileLayout(value),
            Self::DiffLayout(value) => Action::DiffLayout(value),
            Self::Search(value) => Action::Search(value),
            Self::Interaction(value) => Action::Interaction(value),
            Self::StartupCheck(_) => Action::StartupCheck,
            Self::TextWrap(value) => Action::Wrap(value),
            Self::Autopush(_) => Action::Autopush,
            Self::AiAgent(_) => Action::AiAgent,
        }
    }
    fn kind(self) -> Kind {
        match self {
            Self::LogSelection(_) => Kind::LogSelection,
            Self::Filter(_) => Kind::Filter,
            Self::Sort(_) => Kind::Sort,
            Self::FileView(_) => Kind::FileView,
            Self::DiffLayout(_) => Kind::DiffLayout,
            Self::Search(_) => Kind::Search,
            Self::Interaction(_) => Kind::Interaction,
            Self::StartupCheck(_) => Kind::StartupCheck,
            Self::TextWrap(_) => Kind::TextWrap,
            Self::Autopush(_) => Kind::Autopush,
            Self::AiAgent(_) => Kind::AiAgent,
        }
    }
}
impl Kind {
    pub fn action(self) -> Action {
        match self {
            Self::LogSelection => Action::LogSelect,
            Self::Filter => Action::FilterMenu,
            Self::Sort => Action::SortMenu,
            Self::FileView => Action::FileLayoutMenu,
            Self::DiffLayout => Action::DiffLayoutMenu,
            Self::Search => Action::SearchMenu,
            Self::Interaction => Action::InteractionMenu,
            Self::StartupCheck => Action::StartupCheck,
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
            Self::LogSelection => return None,
            Self::Filter => Value::Filter(screen.filter),
            Self::Sort => Value::Sort(screen.order),
            Self::FileView => Value::FileView(screen.file_layout),
            Self::DiffLayout => Value::DiffLayout(screen.diff_preference),
            Self::Search => Value::Search(screen.search_support().mode()?),
            Self::Interaction => Value::Interaction(screen.config.interaction_mode),
            Self::StartupCheck => Value::StartupCheck(screen.config.check_on_startup),
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
    pub fn new(input: &Input, kind: Kind, screen: &Screen) -> std::option::Option<Self> {
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
                screen.search_support().admits(option.value.action())
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
