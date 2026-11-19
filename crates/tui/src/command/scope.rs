//! Apply the same target rules to command discovery, completion and direct input.
use super::{
    Action, Command,
    hierarchy::{Group, Location},
    named::{Candidate, Query, parameters},
};
use crate::{Screen, bar::InputBar, clipboard};
use std::path::Path;

#[derive(Default)]
pub(crate) struct CommandMenu {
    pub group: Option<Group>,
    pub entries: Vec<Candidate>,
}

impl Screen {
    pub(crate) fn command_menu(&self) -> Option<CommandMenu> {
        let InputBar::Command {
            input,
            path,
            selection,
            ..
        } = self.active_bar()
        else {
            return None;
        };
        let visible = |command: &Command| {
            self.command_availability(command.action, path.as_deref(), selection)
                .visible(input.discovery())
        };
        let query = input.query();
        if let Some(named) = Query::new(&query).filter(Query::parameter_only) {
            let entries = if visible(named.owner()) {
                named.candidates().map(Candidate::Parameter).collect()
            } else {
                Vec::new()
            };
            return Some(CommandMenu {
                group: None,
                entries,
            });
        }

        let mut commands = input
            .matches()
            .into_iter()
            .filter(|command| visible(command))
            .collect::<Vec<_>>();
        let group = Group::current(input, &commands);
        if let Some(group) = &group {
            group.retain_candidates(&mut commands);
        }
        super::hierarchy::arrange(&mut commands);

        // 展开后父命令行收起，仍由这个父命令提供参数子项。
        let mut entries = Vec::new();
        if let Some(group) = &group
            && group.location == Location::Children
            && visible(group.parent)
        {
            entries.extend(parameters(group.parent).map(Candidate::Parameter));
        }
        for command in commands {
            entries.push(Candidate::Command(command));
            entries.extend(parameters(command).map(Candidate::Parameter));
        }
        Some(CommandMenu { group, entries })
    }

    pub(crate) fn command_candidates(&self) -> Option<Vec<Candidate>> {
        self.command_menu().map(|menu| menu.entries)
    }

    #[cfg(test)]
    pub(crate) fn search_commands(&self) -> Option<Vec<&'static Command>> {
        self.command_candidates()
            .map(|entries| entries.into_iter().filter_map(Candidate::command).collect())
    }

    pub(crate) fn out_of_view_command(
        &self,
        query: &str,
        path: Option<&Path>,
        selection: &clipboard::Selection,
    ) -> Option<Action> {
        if selection.is_error() {
            return None;
        }
        super::COMMANDS
            .iter()
            .find(|command| {
                command.name.eq_ignore_ascii_case(query.trim())
                    && matches!(
                        self.command_availability(command.action, path, selection),
                        super::availability::Availability::Target(
                            super::target::Target::OutOfScope(_)
                        )
                    )
            })
            .map(|command| command.action)
    }
}
