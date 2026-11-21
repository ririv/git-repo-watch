//! Bind asynchronous candidates to the captured command and repository.
use super::*;
use command::{
    completion::Menu,
    loader::{Output, Request},
    values::{Candidates, Kind},
};

impl Screen {
    pub(super) fn start_path_completion(&mut self) {
        let InputBar::Command { input, menu, .. } = self.tasks.input_bar_mut(&mut self.bar) else {
            return;
        };
        let request = Request::Directory {
            editor: input.canonical_editor(),
            home: self.service.scheduler.user_home().into(),
        };
        self.command_reader.request(request.clone(), &self.service);
        *menu = Menu::Loading(request);
        self.message.clear();
    }
    pub(crate) fn refresh_value_candidates(&mut self) {
        let InputBar::Command {
            input, menu, path, ..
        } = self.tasks.input_bar_mut(&mut self.bar)
        else {
            return;
        };
        let editor = input.canonical_editor();
        let kind = command::argument_input(editor.text.strip_prefix('/').expect("command slash"))
            .filter(|(_, raw)| editor.cursor >= editor.text.len() - raw.len())
            .and_then(|(argument, raw)| Kind::for_argument(argument, raw));
        let Some(kind) = kind.filter(|kind| *kind == Kind::Editor || path.is_some()) else {
            if matches!(
                menu,
                Menu::Values(_) | Menu::Loading(Request::Values { .. } | Request::Editors { .. })
            ) {
                *menu = Menu::Commands;
            }
            return;
        };
        if let Menu::Values(values) = menu
            && values.kind == kind
            && values.refine(input).is_some()
        {
            return;
        }
        let request = if kind == Kind::Editor {
            Request::Editors { path: path.clone() }
        } else {
            Request::Values {
                kind,
                path: path.clone().expect("repository parameter"),
            }
        };
        if matches!(menu, Menu::Loading(current) if *current == request) {
            return;
        }
        self.command_reader.request(request.clone(), &self.service);
        *menu = Menu::Loading(request);
    }
    pub(crate) fn poll_command_completion(&mut self) -> bool {
        if !matches!(
            self.active_bar(),
            InputBar::Command {
                menu: Menu::Loading(_),
                ..
            }
        ) {
            self.command_reader.clear();
        }
        let changed = self.command_reader.tick();
        let Some((request, result)) = self.command_reader.poll() else {
            return changed;
        };
        let InputBar::Command {
            input,
            menu: menu @ Menu::Loading(_),
            ..
        } = self.tasks.input_bar_mut(&mut self.bar)
        else {
            return false;
        };
        if !matches!(menu, Menu::Loading(current) if *current == request) {
            return false;
        }
        match result {
            Ok(Output::Directory { editor, menu: next }) => {
                if let Request::Directory {
                    editor: original, ..
                } = request
                    && input.canonical_editor() == original
                {
                    input.replace_canonical(editor);
                    *menu = next;
                    self.message.clear();
                } else {
                    *menu = Menu::Commands;
                }
            }
            Ok(Output::Values { kind, entries }) => {
                *menu = match Candidates::new(kind, entries, input) {
                    Some(mut values) => {
                        if kind == Kind::Editor
                            && command::argument_input(&input.query())
                                .is_some_and(|(_, raw)| raw.trim().is_empty())
                        {
                            values.select_token(&self.config.editor.value());
                        }
                        Menu::Values(values)
                    }
                    None => Menu::Commands,
                };
            }
            Err(error) => {
                self.message = crate::message::Message::error(error.clone());
                *menu = Menu::Failed(error);
            }
        }
        true
    }
}
