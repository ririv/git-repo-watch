//! Typed parameter commands and literal single-value parsing; no shell evaluation.
use super::{Action, COMMANDS, Command};
use anyhow::{Context, Result, ensure};

pub(crate) enum ArgumentSyntax {
    Input(&'static str),
    Choices(Vec<String>),
}
impl ArgumentSyntax {
    pub fn placeholder(&self) -> String {
        match self {
            Self::Input(text) => (*text).into(),
            Self::Choices(values) => format!("[{}]", values.join("|")),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgumentAction {
    Add,
    BranchCreate,
    BranchSwitch,
    StashSave,
    StashSaveAll,
    StashApply,
    StashPop,
    StashDrop,
    Scan,
    Interval,
    Anchor,
    Autopush,
    Concurrency,
    StartupCheck,
    AiAgent,
    AiModel,
    AiPrompt,
    Option(super::options::Kind),
}
impl ArgumentAction {
    pub fn options(self) -> Option<super::options::Kind> {
        use super::options::Kind;
        match self {
            Self::Option(kind) => Some(kind),
            Self::StartupCheck => Some(Kind::StartupCheck),
            Self::Autopush => Some(Kind::Autopush),
            Self::AiAgent => Some(Kind::AiAgent),
            _ => None,
        }
    }
    pub fn is_path(self) -> bool {
        matches!(self, Self::Add | Self::Scan)
    }
    pub fn action(self) -> Action {
        match self {
            Self::Add => Action::Add,
            Self::BranchCreate => Action::BranchCreate,
            Self::BranchSwitch => Action::BranchSwitch,
            Self::StashSave => Action::StashSave,
            Self::StashSaveAll => Action::StashSaveAll,
            Self::StashApply => Action::StashApply,
            Self::StashPop => Action::StashPop,
            Self::StashDrop => Action::StashDrop,

            Self::Scan => Action::Scan,
            Self::Interval => Action::Interval,
            Self::Anchor => Action::Anchor,
            Self::Autopush => Action::Autopush,
            Self::Concurrency => Action::Concurrency,
            Self::StartupCheck => Action::StartupCheck,
            Self::AiAgent => Action::AiAgent,
            Self::AiModel => Action::AiModel,
            Self::AiPrompt => Action::AiPrompt,
            Self::Option(kind) => kind.action(),
        }
    }
    pub fn command(self) -> &'static Command {
        COMMANDS
            .iter()
            .find(|command| command.action == self.action())
            .expect("argument command catalog")
    }
    pub fn placeholder(self) -> String {
        self.syntax().placeholder()
    }
    pub fn syntax(self) -> ArgumentSyntax {
        let open = match self {
            Self::Add => "<path>",
            Self::Scan => "<path> [--exclude <path>…]",
            Self::Interval => "<duration>",
            Self::Anchor => "[<datetime>|auto]",
            Self::Concurrency => "<count>",
            Self::Option(_) | Self::Autopush | Self::StartupCheck | Self::AiAgent => {
                return ArgumentSyntax::Choices(
                    self.options()
                        .expect("fixed-value argument")
                        .options()
                        .map(|option| option.token.into())
                        .collect(),
                );
            }
            Self::AiModel => "[<model>|default]",
            Self::AiPrompt => "<text>",
            Self::BranchCreate | Self::BranchSwitch => "<name>",
            Self::StashSave | Self::StashSaveAll => "<message>",
            Self::StashApply | Self::StashPop | Self::StashDrop => "<stash-id>",
        };
        ArgumentSyntax::Input(open)
    }
    pub fn hint(self) -> &'static str {
        match self {
            Self::Add | Self::Scan => i18n::tr!(PathArgumentHint),
            Self::BranchCreate | Self::BranchSwitch => i18n::tr!(BranchNameHint),
            Self::StashSave | Self::StashSaveAll => i18n::tr!(StashMessageHint),
            Self::StashApply | Self::StashPop | Self::StashDrop => i18n::tr!(StashIdHint),
            Self::Interval => i18n::tr!(DurationArgumentHint),
            Self::Anchor => i18n::tr!(ScheduleAnchorHint),
            Self::Concurrency => i18n::tr!(CountArgumentHint),
            Self::AiModel => i18n::tr!(TabCompleteCandidatesEnterRunEscClear),
            Self::AiPrompt => i18n::tr!(TabCompleteCandidatesEnterRunEscClear),
            Self::Option(_) | Self::Autopush | Self::StartupCheck | Self::AiAgent => {
                i18n::tr!(OptionCompletionHint)
            }
        }
    }
    pub fn parse(self, value: &str, user_home: &std::path::Path) -> Result<String> {
        let value = value.trim();
        ensure!(
            !value.is_empty(),
            i18n::tf!(MissingCommandArgument, self.placeholder())
        );
        match self {
            Self::Option(kind) => {
                let value = kind.parse(value)?;
                Ok(kind
                    .options()
                    .find(|option| option.value == value)
                    .expect("parsed option")
                    .token
                    .into())
            }
            Self::Scan => {
                super::scan::request(value, user_home)?;
                Ok(value.into())
            }
            Self::Add => {
                let value = match value.chars().next() {
                    Some(quote @ ('\'' | '"')) => {
                        ensure!(
                            value.len() >= 2 && value.ends_with(quote),
                            i18n::tr!(UnclosedPathQuote)
                        );
                        &value[1..value.len() - 1]
                    }
                    _ => value,
                };
                let path = engine::paths::expand_home(value, user_home)?;
                ensure!(path.is_absolute(), i18n::tr!(EnterAnAbsolutePath));
                path.into_os_string()
                    .into_string()
                    .ok()
                    .context(i18n::tr!(ExpandedPathNotUtf8))
            }
            Self::BranchCreate | Self::BranchSwitch | Self::StashSave | Self::StashSaveAll => {
                ensure!(!value.contains('\0'), "NUL is not allowed");
                Ok(value.into())
            }
            Self::StashApply | Self::StashPop | Self::StashDrop => {
                ensure!(
                    value.len() == 40 && value.bytes().all(|b| b.is_ascii_hexdigit()),
                    i18n::tr!(StashIdRequired)
                );
                Ok(value.into())
            }
            Self::Interval => {
                engine::state::interval(value)?;
                Ok(value.into())
            }
            Self::Anchor => {
                let anchor = engine::scheduler::timing::parse_anchor(
                    value,
                    chrono::Local::now().date_naive(),
                )?;
                Ok(match anchor {
                    engine::scheduler::timing::Anchor::Unset => "auto".into(),
                    engine::scheduler::timing::Anchor::Set(basis) => {
                        basis.at.format("%Y-%m-%d %H:%M:%S").to_string()
                    }
                })
            }
            Self::Autopush => {
                value.parse::<engine::domain::PushPolicy>()?;
                Ok(value.into())
            }
            Self::StartupCheck => {
                ensure!(
                    matches!(value, "true" | "false"),
                    i18n::tr!(BooleanArgumentRequired)
                );
                Ok(value.into())
            }
            Self::AiAgent => Ok(value.parse::<engine::ai::Agent>()?.name().into()),
            Self::AiModel | Self::AiPrompt => {
                let mut config = engine::ai::Config::default();
                if self == Self::AiModel {
                    config.program_mut().model = value.into();
                } else {
                    config.prompt = engine::ai::Prompt::Custom { text: value.into() };
                }
                config.validate()?;
                Ok(value.into())
            }
            Self::Concurrency => {
                let count = value.parse::<usize>()?;
                ensure!(
                    (1..=32).contains(&count),
                    i18n::tr!(ConcurrencyMustBeBetween1And32)
                );
                Ok(value.into())
            }
        }
    }
}

impl Action {
    pub fn argument(self) -> Option<ArgumentAction> {
        match self {
            Self::PartialSelectMenu => {
                Some(ArgumentAction::Option(super::options::Kind::DiffSelection))
            }
            Self::LogSelect => Some(ArgumentAction::Option(super::options::Kind::LogSelection)),
            Self::FilterMenu => Some(ArgumentAction::Option(super::options::Kind::Filter)),
            Self::SortMenu => Some(ArgumentAction::Option(super::options::Kind::Sort)),
            Self::WrapMenu => Some(ArgumentAction::Option(super::options::Kind::TextWrap)),
            Self::FileLayoutMenu => Some(ArgumentAction::Option(super::options::Kind::FileView)),
            Self::DiffLayoutMenu => Some(ArgumentAction::Option(super::options::Kind::DiffLayout)),
            Self::SearchMenu => Some(ArgumentAction::Option(super::options::Kind::Search)),
            Self::InteractionMenu => {
                Some(ArgumentAction::Option(super::options::Kind::Interaction))
            }
            Self::Add => Some(ArgumentAction::Add),
            Self::BranchCreate => Some(ArgumentAction::BranchCreate),
            Self::BranchSwitch => Some(ArgumentAction::BranchSwitch),
            Self::StashSave => Some(ArgumentAction::StashSave),
            Self::StashSaveAll => Some(ArgumentAction::StashSaveAll),
            Self::StashApply => Some(ArgumentAction::StashApply),
            Self::StashPop => Some(ArgumentAction::StashPop),
            Self::StashDrop => Some(ArgumentAction::StashDrop),

            Self::Scan => Some(ArgumentAction::Scan),
            Self::Interval => Some(ArgumentAction::Interval),
            Self::Anchor => Some(ArgumentAction::Anchor),
            Self::Autopush => Some(ArgumentAction::Autopush),
            Self::Concurrency => Some(ArgumentAction::Concurrency),
            Self::StartupCheck => Some(ArgumentAction::StartupCheck),
            Self::AiAgent => Some(ArgumentAction::AiAgent),
            Self::AiModel => Some(ArgumentAction::AiModel),
            Self::AiPrompt => Some(ArgumentAction::AiPrompt),
            _ => None,
        }
    }
}
impl Command {
    pub fn usage(&self) -> String {
        match self.action.argument() {
            Some(argument) => format!("{} {}", self.name, argument.placeholder()),
            None => self.name.into(),
        }
    }
    pub fn completion(&self) -> String {
        format!(
            "/{}{}",
            self.name,
            if self.action.argument().is_some() || self.action == Action::DiffLayoutMenu {
                " "
            } else {
                ""
            }
        )
    }
}
pub fn argument_input(input: &str) -> Option<(ArgumentAction, &str)> {
    let canonical = COMMANDS.iter().find_map(|command| {
        let argument = command.action.argument()?;
        let input = input.trim_start();
        if !input
            .get(..command.name.len())?
            .eq_ignore_ascii_case(command.name)
        {
            return None;
        }
        let value = input.get(command.name.len()..)?.strip_prefix(' ')?;
        Some((argument, value))
    });
    if canonical.is_some() {
        return canonical;
    }
    // 保留命令菜单原有的唯一后缀输入；只解析明确的选项命令，不猜测参数内容。
    let input = input.trim_start();
    let mut aliases = COMMANDS.iter().filter_map(|command| {
        let argument = command.action.argument()?;
        argument.options()?;
        command.name.match_indices(' ').find_map(|(index, _)| {
            let alias = &command.name[index + 1..];
            input
                .get(..alias.len())?
                .eq_ignore_ascii_case(alias)
                .then_some(())?;
            Some((argument, input.get(alias.len()..)?.strip_prefix(' ')?))
        })
    });
    let matched = aliases.next()?;
    aliases.next().is_none().then_some(matched)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigArgument {
    Add,
    Interval,
    Anchor,
    Autopush,
    Concurrency,
    StartupCheck,
    AiModel,
    AiPrompt,
}
impl ConfigArgument {
    pub fn action(self) -> Action {
        self.argument().action()
    }
    pub fn command(self) -> &'static Command {
        self.argument().command()
    }
    fn argument(self) -> ArgumentAction {
        match self {
            Self::Add => ArgumentAction::Add,
            Self::Interval => ArgumentAction::Interval,
            Self::Anchor => ArgumentAction::Anchor,
            Self::Autopush => ArgumentAction::Autopush,
            Self::Concurrency => ArgumentAction::Concurrency,
            Self::StartupCheck => ArgumentAction::StartupCheck,
            Self::AiModel => ArgumentAction::AiModel,
            Self::AiPrompt => ArgumentAction::AiPrompt,
        }
    }
}
impl ArgumentAction {
    pub fn configuration(self) -> Option<ConfigArgument> {
        match self {
            Self::Add => Some(ConfigArgument::Add),
            Self::Interval => Some(ConfigArgument::Interval),
            Self::Anchor => Some(ConfigArgument::Anchor),
            Self::Autopush => Some(ConfigArgument::Autopush),
            Self::Concurrency => Some(ConfigArgument::Concurrency),
            Self::StartupCheck => Some(ConfigArgument::StartupCheck),
            Self::AiModel => Some(ConfigArgument::AiModel),
            Self::AiPrompt => Some(ConfigArgument::AiPrompt),
            _ => None,
        }
    }
}
