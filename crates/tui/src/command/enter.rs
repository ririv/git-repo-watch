//! Describe the next Enter transition using the same command and parameter candidates.
use super::{Action, ArgumentAction, completion::Menu, named::Candidate};
use crate::{Screen, bar::InputBar};

enum Intent {
    Parameters,
    Group,
    Argument,
    Required,
    Review,
    Open,
    Run,
    Apply,
    FillCommand,
    FillPath,
    FillValue,
    Waiting,
}
impl Intent {
    fn text(self) -> String {
        match self {
            Self::Parameters => i18n::tr!(CommandEnterParameters).into(),
            Self::Group => i18n::tr!(CommandEnterGroup).into(),
            Self::Argument => i18n::tr!(CommandEnterArgument).into(),
            Self::Required => i18n::tr!(CommandEnterRequired).into(),
            Self::Review => i18n::tr!(CommandEnterReview).into(),
            Self::Open => i18n::tr!(CommandEnterOpen).into(),
            Self::Run => i18n::tr!(CommandEnterRun).into(),
            Self::Apply => i18n::tr!(CommandEnterApply).into(),
            Self::FillCommand => i18n::tr!(CommandEnterFillCommand).into(),
            Self::FillPath => i18n::tr!(CommandEnterFillPath).into(),
            Self::FillValue => i18n::tr!(CommandEnterFillValue).into(),
            Self::Waiting => i18n::tr!(CommandEnterWaiting).into(),
        }
    }
}
impl super::Command {
    fn enter_intent(&self) -> Intent {
        if let Some(argument) = self.action.argument()
            && self.action != Action::AiPrompt
        {
            return if argument.options().is_some() {
                Intent::Parameters
            } else {
                Intent::Argument
            };
        }
        if super::named::has_parameters(self) {
            return Intent::Group;
        }
        match self.action {
            Action::OpenRepository
            | Action::Repositories
            | Action::RepositorySettings
            | Action::Settings
            | Action::AiSettings
            | Action::AiPrompt
            | Action::Schedule
            | Action::Workflow
            | Action::Help
            | Action::Tasks
            | Action::Branches
            | Action::Stashes
            | Action::Log
            | Action::Diff
            | Action::DiffGroup(_)
            | Action::BrowseFiles
            | Action::BrowserDiff
            | Action::Commit
            | Action::Compose(crate::composition::Action::Open)
            | Action::Terminal
            | Action::OpenEditor => Intent::Open,
            _ => Intent::Run,
        }
    }
}
impl Screen {
    pub(crate) fn command_enter_hint(&self) -> Option<String> {
        let InputBar::Command {
            input,
            selected,
            menu,
            path,
            selection,
            ..
        } = &self.bar
        else {
            return None;
        };
        if input.prefix_selected() {
            return None;
        }
        let keys = if matches!(menu, Menu::Commands)
            && matches!(input, super::Input::Global(editor) if editor.text == "/")
        {
            i18n::tr!(CommandCandidateHistoryKeys)
        } else if matches!(menu, Menu::Commands)
            && matches!(input, super::Input::Scoped { editor, .. } if editor.text.is_empty())
        {
            i18n::tr!(CommandScopedCandidateKeys)
        } else {
            i18n::tr!(CommandCandidateKeys)
        };
        let intent = match menu {
            // 候选已展示名称和说明；底部只提示按键行为，切换候选时不复述内容。
            Menu::Options(values) => {
                let option = values.options.get(values.selected)?;
                if let Some(reason) = self
                    .command_availability(option.value.action(), path.as_deref(), selection)
                    .reason()
                {
                    return Some(format!("{reason} · {keys}"));
                }
                Intent::Apply
            }
            Menu::Directories(_) => Intent::FillPath,
            Menu::History(_) => Intent::FillCommand,
            Menu::Values(_) => Intent::FillValue,
            Menu::Loading(_) => Intent::Waiting,
            Menu::Failed(_) => return None,
            Menu::Commands => {
                let query = input.query();
                if let Some((argument, raw)) = super::argument_input(&query) {
                    if let Some(reason) = self
                        .command_availability(argument.action(), path.as_deref(), selection)
                        .reason()
                    {
                        return Some(format!("{reason} · {keys}"));
                    }
                    match argument {
                        ArgumentAction::Option(_) if raw.trim().is_empty() => Intent::Parameters,
                        ArgumentAction::Option(kind) => {
                            kind.parse(raw).ok()?;
                            Intent::Apply
                        }
                        ArgumentAction::AiPrompt if raw.trim().is_empty() => Intent::Open,
                        ArgumentAction::Editor | ArgumentAction::AiAgent => Intent::FillValue,
                        _ if raw.trim().is_empty() => Intent::Required,
                        _ => Intent::Review,
                    }
                } else {
                    let menu = self.command_menu()?;
                    let candidate = *menu.entries.get(*selected)?;
                    let command = candidate.owner();
                    if let Some(reason) = self
                        .command_availability(command.action, path.as_deref(), selection)
                        .reason()
                    {
                        return Some(format!("{reason} · {keys}"));
                    }
                    match candidate {
                        Candidate::Parameter(_) => Intent::Parameters,
                        Candidate::Command(command) => {
                            if menu
                                .group
                                .as_ref()
                                .is_some_and(|group| group.at_parent(command))
                            {
                                let keys = if super::named::has_parameters(command) {
                                    i18n::tr!(CommandGroupKeys)
                                } else {
                                    i18n::tr!(CommandChildrenKeys)
                                };
                                return Some(format!("{} · {keys}", command.enter_intent().text()));
                            }
                            command.enter_intent()
                        }
                    }
                }
            }
        };
        Some(format!("{} · {keys}", intent.text()))
    }
}
