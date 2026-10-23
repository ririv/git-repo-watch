//! Explicit command groups are captured from the focused view when opening a draft.
use super::{Action, Command, Configuration};
use crate::{Mode, Page, Screen, history, workspace};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DiffContext {
    Worktree,
    History,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Context {
    Diff(DiffContext),
    History,
    Branches,
    Stash,
    Repositories,
    Settings,
    Schedule,
    Error,
}
impl Context {
    pub fn name(self) -> &'static str {
        match self {
            Self::Diff(_) => "diff",
            Self::History => "log",
            Self::Branches => "branch",
            Self::Stash => "stash",
            Self::Repositories => "repo",
            Self::Settings => "settings",
            Self::Schedule => "schedule",
            Self::Error => "error",
        }
    }
    pub fn relative(self, command: &Command) -> Option<&'static str> {
        command.name.strip_prefix(self.name())?.strip_prefix(' ')
    }
    pub fn admits(self, command: &Command) -> bool {
        self.relative(command).is_some()
            && !(self == Self::Diff(DiffContext::History)
                && matches!(
                    command.action,
                    Action::Stage | Action::Unstage | Action::Refresh
                ))
    }
    pub fn matches(self, input: &str) -> Vec<&'static Command> {
        let canonical = format!("{} {input}", self.name());
        if let Some((argument, _)) = super::argument_input(&canonical) {
            return self
                .admits(argument.command())
                .then_some(argument.command())
                .into_iter()
                .collect();
        }
        let input = input.trim_start();
        let query = input.trim_end().to_lowercase();
        let parent = input
            .strip_suffix(' ')
            .filter(|parent| {
                super::COMMANDS.iter().any(|c| {
                    self.admits(c)
                        && self
                            .relative(c)
                            .is_some_and(|name| name.eq_ignore_ascii_case(parent))
                })
            })
            .map(|parent| format!("{} ", parent.to_ascii_lowercase()));
        let mut candidates = super::COMMANDS
            .iter()
            .filter(|c| self.admits(c))
            .filter_map(|c| {
                let relative = self.relative(c).expect("admitted group member");
                if parent
                    .as_ref()
                    .is_some_and(|parent| !relative.starts_with(parent))
                {
                    return None;
                }
                c.match_name(relative, &query).map(|rank| (rank, c))
            })
            .collect::<Vec<_>>();
        candidates.sort_by_key(|(rank, _)| *rank);
        candidates.into_iter().map(|(_, command)| command).collect()
    }
}
impl Screen {
    pub(crate) fn command_context(&self) -> Option<Context> {
        match &self.mode {
            Mode::Changes(_) | Mode::Files(_) => Some(Context::Diff(DiffContext::Worktree)),
            Mode::History(view) => Some(match view.as_ref() {
                history::View::Diff { .. } => Context::Diff(DiffContext::History),
                _ => Context::History,
            }),
            Mode::Branches(_) => Some(Context::Branches),
            Mode::Stashes(_) => Some(Context::Stash),
            Mode::Error { .. } => Some(Context::Error),
            Mode::Panel(section) => match section {
                workspace::Section::Worktree => Some(Context::Diff(DiffContext::Worktree)),
                workspace::Section::History => Some(Context::History),
                workspace::Section::Branches => Some(Context::Branches),
                workspace::Section::Overview => None,
            },
            Mode::Browsing => match self.page {
                Page::Configuration(Configuration::General) => Some(Context::Settings),
                Page::Configuration(Configuration::Repositories) => Some(Context::Repositories),
                Page::Configuration(Configuration::Schedule) => Some(Context::Schedule),
                _ => None,
            },
            _ => None,
        }
    }
}
