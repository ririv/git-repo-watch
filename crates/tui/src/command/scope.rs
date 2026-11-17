//! Apply the same target rules to command discovery, completion and direct input.
use super::{Action, Command, target::Status};
use crate::{Screen, bar::InputBar, clipboard};
use std::path::Path;

impl Screen {
    pub(crate) fn command_visible(
        &self,
        action: Action,
        path: Option<&Path>,
        selection: &clipboard::Selection,
    ) -> bool {
        action.available(&self.config)
            && (!selection.is_error()
                || matches!(action, Action::Copy | Action::Quit | Action::Tasks))
            && !matches!(
                self.target_status(action, path, selection),
                Status::OutOfScope(_)
            )
    }

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
            .filter(|command| self.command_visible(command.action, path.as_deref(), selection))
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
                        self.target_status(command.action, path, selection),
                        Status::OutOfScope(_)
                    )
            })
            .map(|command| command.action)
    }
}
