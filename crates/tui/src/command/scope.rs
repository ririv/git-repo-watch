//! Apply the same target rules to command discovery, completion and direct input.
use super::{Action, Command};
use crate::{Screen, bar::InputBar, clipboard};
use std::path::Path;

impl Screen {
    pub(crate) fn search_commands(&self) -> Option<Vec<&'static Command>> {
        let InputBar::Command {
            input,
            path,
            selection,
            ..
        } = &self.bar
        else {
            return None;
        };
        let mut candidates = input
            .matches()
            .into_iter()
            .filter(|command| {
                self.command_availability(command.action, path.as_deref(), selection)
                    .visible(input.discovery())
            })
            .collect::<Vec<_>>();
        if let Some(group) = super::hierarchy::Group::current(input, &candidates) {
            group.retain_candidates(&mut candidates);
        }
        super::hierarchy::arrange(&mut candidates);
        Some(candidates)
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
