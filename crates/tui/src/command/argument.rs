//! Typed parameter commands and literal single-value parsing; no shell evaluation.
use super::{Action, COMMANDS, Command};
use anyhow::{Context, Result, ensure};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgumentAction {
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
            Self::Exclude => Action::Exclude,
            Self::Unexclude => Action::Unexclude,
            Self::Scan => Action::Scan,
            Self::Import => Action::Import,
            Self::Interval => Action::Interval,
            Self::Anchor => Action::Anchor,
            Self::Autopush => Action::Autopush,
            Self::Concurrency => Action::Concurrency,
            Self::StartupCheck => Action::StartupCheck,
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
        }
    }
    pub fn hint(self) -> &'static str {
        match self {
            Self::Add | Self::Exclude | Self::Unexclude | Self::Scan | Self::Import => {
                i18n::tr!(PathArgumentHint)
            }
            Self::Interval => i18n::tr!(DurationArgumentHint),
            Self::Anchor => i18n::tr!(ScheduleAnchorHint),
            Self::Autopush => i18n::tr!(PolicyArgumentHint),
            Self::Concurrency => i18n::tr!(CountArgumentHint),
            Self::StartupCheck => i18n::tr!(StartupCheckArgumentHint),
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
            Self::Exclude => Some(ArgumentAction::Exclude),
            Self::Unexclude => Some(ArgumentAction::Unexclude),
            Self::Scan => Some(ArgumentAction::Scan),
            Self::Import => Some(ArgumentAction::Import),
            Self::Interval => Some(ArgumentAction::Interval),
            Self::Anchor => Some(ArgumentAction::Anchor),
            Self::Autopush => Some(ArgumentAction::Autopush),
            Self::Concurrency => Some(ArgumentAction::Concurrency),
            Self::StartupCheck => Some(ArgumentAction::StartupCheck),
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
        let value = input
            .trim_start()
            .strip_prefix(command.name)?
            .strip_prefix(' ')?;
        Some((argument, value))
    })
}
