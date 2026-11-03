//! Bind asynchronous candidates to the captured command and repository.
use super::*;
use command::{
    completion::Menu,
    loader::{Output, Request},
    values::{Candidates, Kind},
};

impl Screen {
    pub(super) fn start_path_completion(&mut self) {
        let InputBar::Command { input, menu, .. } = &mut self.bar else {
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
    pub(super) fn refresh_value_candidates(&mut self) {
        let InputBar::Command {
            input, menu, path, ..
        } = &mut self.bar
        else {
            return;
        };
        let editor = input.canonical_editor();
        let kind = command::argument_input(editor.text.strip_prefix('/').expect("command slash"))
            .filter(|(_, raw)| editor.cursor >= editor.text.len() - raw.len())
            .and_then(|(argument, _)| Kind::for_argument(argument));
        let Some((kind, path)) = kind.zip(path.as_ref()) else {
            if matches!(
                menu,
                Menu::Values(_) | Menu::Loading(Request::Values { .. })
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
        let request = Request::Values {
            kind,
            path: path.clone(),
        };
        if matches!(menu, Menu::Loading(current) if *current == request) {
            return;
        }
        self.command_reader.request(request.clone(), &self.service);
        *menu = Menu::Loading(request);
    }
    pub(crate) fn poll_command_completion(&mut self) -> bool {
        if !matches!(
            self.bar,
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
        } = &mut self.bar
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
                    Some(values) => Menu::Values(values),
                    None => Menu::Commands,
                };
            }
            Err(error) => {
                self.message = error.clone();
                *menu = Menu::Failed(error);
            }
        }
        true
    }
}
