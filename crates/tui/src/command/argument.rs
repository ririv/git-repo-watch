//! Typed parameter commands and literal single-value parsing; no shell evaluation.
use super::{Action, COMMANDS, Command};
use anyhow::{Context, Result, ensure};
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
    Exclude,
    Unexclude,
    Scan,
    Import,
    Interval,
    Anchor,
    Autopush,
    Concurrency,
    StartupCheck,
    AiAgent,
    AiModel,
    AiPrompt,
}
impl ArgumentAction {
    pub fn is_path(self) -> bool {
        matches!(
            self,
            Self::Add | Self::Exclude | Self::Unexclude | Self::Scan | Self::Import
        )
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

            Self::Exclude => Action::Exclude,
            Self::Unexclude => Action::Unexclude,
            Self::Scan => Action::Scan,
            Self::Import => Action::Import,
            Self::Interval => Action::Interval,
            Self::Anchor => Action::Anchor,
            Self::Autopush => Action::Autopush,
            Self::Concurrency => Action::Concurrency,
            Self::StartupCheck => Action::StartupCheck,
            Self::AiAgent => Action::AiAgent,
            Self::AiModel => Action::AiModel,
            Self::AiPrompt => Action::AiPrompt,
        }
    }
    pub fn command(self) -> &'static Command {
        COMMANDS
            .iter()
            .find(|command| command.action == self.action())
            .expect("argument command catalog")
    }
    pub fn placeholder(self) -> &'static str {
        match self {
            Self::Add | Self::Exclude | Self::Unexclude | Self::Scan | Self::Import => "<path>",
            Self::Interval => "<duration>",
            Self::Anchor => "<datetime|auto>",
            Self::Autopush => "<policy>",
            Self::Concurrency => "<count>",
            Self::StartupCheck => "<true|false>",
            Self::AiAgent => "<name>",
            Self::AiModel => "<model|default>",
            Self::AiPrompt => "<text>",
            Self::BranchCreate | Self::BranchSwitch => "<name>",
            Self::StashSave | Self::StashSaveAll => "<message>",
            Self::StashApply | Self::StashPop | Self::StashDrop => "<stash-id>",
        }
    }
    pub fn hint(self) -> &'static str {
        match self {
            Self::Add | Self::Exclude | Self::Unexclude | Self::Scan | Self::Import => {
                i18n::tr!(PathArgumentHint)
            }
            Self::BranchCreate | Self::BranchSwitch => i18n::tr!(BranchNameHint),
            Self::StashSave | Self::StashSaveAll => i18n::tr!(StashMessageHint),
            Self::StashApply | Self::StashPop | Self::StashDrop => i18n::tr!(StashIdHint),
            Self::Interval => i18n::tr!(DurationArgumentHint),
            Self::Anchor => i18n::tr!(ScheduleAnchorHint),
            Self::Autopush => i18n::tr!(PolicyArgumentHint),
            Self::Concurrency => i18n::tr!(CountArgumentHint),
            Self::StartupCheck => i18n::tr!(StartupCheckArgumentHint),
            Self::AiAgent => i18n::tr!(AgentNameHint),
            Self::AiModel => i18n::tr!(AgentModelHint),
            Self::AiPrompt => i18n::tr!(AgentPromptHint),
        }
    }
    pub fn parse(self, value: &str, user_home: &std::path::Path) -> Result<String> {
        let value = value.trim();
        ensure!(
            !value.is_empty(),
            i18n::tf!(MissingCommandArgument, self.placeholder())
        );
        match self {
            Self::Add | Self::Exclude | Self::Unexclude | Self::Scan | Self::Import => {
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
            Self::Add => Some(ArgumentAction::Add),
            Self::BranchCreate => Some(ArgumentAction::BranchCreate),
            Self::BranchSwitch => Some(ArgumentAction::BranchSwitch),
            Self::StashSave => Some(ArgumentAction::StashSave),
            Self::StashSaveAll => Some(ArgumentAction::StashSaveAll),
            Self::StashApply => Some(ArgumentAction::StashApply),
            Self::StashPop => Some(ArgumentAction::StashPop),
            Self::StashDrop => Some(ArgumentAction::StashDrop),

            Self::Exclude => Some(ArgumentAction::Exclude),
            Self::Unexclude => Some(ArgumentAction::Unexclude),
            Self::Scan => Some(ArgumentAction::Scan),
            Self::Import => Some(ArgumentAction::Import),
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
    COMMANDS.iter().find_map(|command| {
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
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigArgument {
    Add,
    Exclude,
    Unexclude,
    Scan,
    Import,
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
            Self::Exclude => ArgumentAction::Exclude,
            Self::Unexclude => ArgumentAction::Unexclude,
            Self::Scan => ArgumentAction::Scan,
            Self::Import => ArgumentAction::Import,
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
            Self::Exclude => Some(ConfigArgument::Exclude),
            Self::Unexclude => Some(ConfigArgument::Unexclude),
            Self::Scan => Some(ConfigArgument::Scan),
            Self::Import => Some(ConfigArgument::Import),
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
