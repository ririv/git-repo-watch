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
    Repository,
    Repositories,
    Settings,
    Startup,
    Ai,
    Schedule,
    Files,
    Composition,
}
impl Context {
    pub fn parent(self) -> Option<Self> {
        match self {
            Self::Diff(_)
            | Self::History
            | Self::Branches
            | Self::Stash
            | Self::Files
            | Self::Composition => Some(Self::Repository),
            Self::Ai | Self::Startup => Some(Self::Settings),
            Self::Repository | Self::Repositories | Self::Settings | Self::Schedule => None,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Diff(_) => "repo diff",
            Self::History => "repo log",
            Self::Branches => "repo branch",
            Self::Stash => "repo stash",
            Self::Repository => "repo",
            Self::Repositories => "repos",
            Self::Settings => "settings",
            Self::Startup => "settings startup",
            Self::Ai => "settings ai",
            Self::Schedule => "schedule",
            Self::Files => "repo files",
            Self::Composition => "repo compose",
        }
    }
    pub fn relative(self, command: &Command) -> Option<&'static str> {
        command.name.strip_prefix(self.name())?.strip_prefix(' ')
    }
    pub fn admits(self, command: &Command) -> bool {
        self.relative(command).is_some()
            && !(self == Self::Repositories
                && matches!(
                    command.action,
                    Action::FilterMenu
                        | Action::Filter(_)
                        | Action::SortMenu
                        | Action::Sort(_)
                        | Action::RepositorySettings
                ))
            && !(self == Self::Diff(DiffContext::History)
                && matches!(
                    command.action,
                    Action::Stage
                        | Action::Unstage
                        | Action::PartialSelectMenu
                        | Action::PartialSelect(_)
                        | Action::Discard
                        | Action::FileLayoutMenu
                        | Action::FileLayout(_)
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
        let grouped = query.is_empty()
            || super::COMMANDS.iter().any(|command| {
                self.admits(command)
                    && self
                        .relative(command)
                        .is_some_and(|name| name.eq_ignore_ascii_case(&query))
            });
        candidates.sort_by_key(|(rank, command)| {
            (
                *rank,
                if grouped {
                    self.relative(command)
                        .expect("admitted command")
                        .split_whitespace()
                        .count()
                } else {
                    0
                },
            )
        });
        candidates.into_iter().map(|(_, command)| command).collect()
    }
}
impl Screen {
    pub(crate) fn command_context(&self) -> Option<Context> {
        match &self.mode {
            Mode::Management(_) => Some(Context::Repositories),
            Mode::Workflow(session) => Some(match session.trigger {
                engine::workflow::Trigger::Startup => Context::Startup,
                engine::workflow::Trigger::Scheduled => Context::Schedule,
            }),
            Mode::Composition(_) | Mode::CompositionWaiting(_) => Some(Context::Composition),
            Mode::Browser(_) => Some(Context::Files),
            Mode::Changes(_) | Mode::Files(_) => Some(Context::Diff(DiffContext::Worktree)),
            Mode::History(view) => Some(match view.as_ref() {
                history::View::Diff { .. } => Context::Diff(DiffContext::History),
                _ => Context::History,
            }),
            Mode::Branches(_) => Some(Context::Branches),
            Mode::Stashes(view) => Some(
                if matches!(view.as_ref(), crate::stashes::View::Diff { .. }) {
                    Context::Diff(DiffContext::History)
                } else {
                    Context::Stash
                },
            ),
            Mode::Error { .. } => None,
            Mode::Panel(section) => match section {
                workspace::Section::Worktree => Some(Context::Diff(DiffContext::Worktree)),
                workspace::Section::History => Some(Context::History),
                workspace::Section::Branches => Some(Context::Branches),
                workspace::Section::Overview => Some(Context::Repository),
            },
            Mode::Browsing => match self.page {
                Page::Configuration(Configuration::General) => Some(
                    Configuration::General
                        .items(self.snapshot.registration, &self.config.ai)
                        .get(self.setting_selected)
                        .map_or(Context::Settings, |item| {
                            let context = item.context();
                            if context == Context::Ai && !self.config.ai.enabled {
                                Context::Settings
                            } else {
                                context
                            }
                        }),
                ),
                Page::Repositories => None,
                Page::Detail => Some(Context::Repository),
                Page::Configuration(Configuration::Repositories) => Some(Context::Repositories),
                Page::Configuration(Configuration::Schedule) => Some(Context::Schedule),
                _ => None,
            },
            _ => None,
        }
    }
}
