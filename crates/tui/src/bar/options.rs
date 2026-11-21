//! Value completion owns selection and execution without adding catalog commands.
use super::*;
use command::{
    completion::Menu,
    options::{Candidates, Kind},
};

impl Screen {
    pub(crate) fn refresh_option_candidates(&mut self) {
        let candidates = match self.active_bar() {
            InputBar::Command {
                input,
                path,
                selection,
                ..
            } => match command::argument_input(&input.query())
                .and_then(|(argument, _)| argument.options())
            {
                Some(kind)
                    if self
                        .command_availability(kind.action(), path.as_deref(), selection)
                        .visible(command::Discovery::Context) =>
                {
                    Candidates::new(input, kind, self, path.as_deref(), selection)
                }
                _ => None,
            },
            _ => None,
        };
        if let InputBar::Command { menu, .. } = self.active_bar_mut() {
            if let Some(candidates) = candidates {
                *menu = Menu::Options(candidates);
            } else if matches!(menu, Menu::Options(_)) {
                *menu = Menu::Commands;
            }
        }
        self.refresh_value_candidates();
    }
    pub(crate) fn open_option_menu(
        &mut self,
        kind: Kind,
        path: Option<PathBuf>,
        selection: crate::clipboard::Selection,
    ) {
        self.open_command_entry(kind.command(), path, selection);
    }
    pub(crate) fn open_command_entry(
        &mut self,
        command: &command::Command,
        path: Option<PathBuf>,
        selection: crate::clipboard::Selection,
    ) {
        let editor = Editor::new(format!("/{} ", command.name));
        let input = if command.common() {
            command::Input::global(editor)
        } else {
            command::Input::from_canonical(self.command_context(), editor)
        };
        *self.active_bar_mut() = InputBar::Command {
            input,
            selected: 0,
            menu: Menu::Commands,
            repository: self.capture_repository(path.as_deref()),
            path,
            selection,
        };
        self.refresh_option_candidates();
    }
    pub(super) fn option_candidate_key(&mut self, key: KeyEvent) -> bool {
        if key.code == KeyCode::Enter
            && let InputBar::Command {
                menu: Menu::Options(candidates),
                path,
                selection,
                ..
            } = self.active_bar()
            && let Some(option) = candidates.options.get(candidates.selected)
            && let Some(reason) = self
                .command_availability(option.value.action(), path.as_deref(), selection)
                .reason()
        {
            self.message = crate::message::Message::error(reason.to_owned());
            return true;
        }
        if key.code == KeyCode::Enter
            && matches!(self.active_bar(), InputBar::Command { input, menu: Menu::Commands, .. }
                if command::argument_input(&input.query()).is_some_and(|(argument, value)|
                    argument.options().is_some() && value.trim().is_empty()))
        {
            self.refresh_option_candidates();
            if matches!(
                self.active_bar(),
                InputBar::Command {
                    menu: Menu::Options(_),
                    ..
                }
            ) {
                self.message.clear();
                return true;
            }
        }
        let InputBar::Command {
            menu: Menu::Options(candidates),
            input,
            ..
        } = self.active_bar_mut()
        else {
            return false;
        };
        match key.code {
            KeyCode::Esc => {
                let query = input.query();
                if let Some(named) = command::named::Query::new(&query)
                    && named.value().is_some_and(|(_, raw)| raw.trim().is_empty())
                {
                    input.replace_canonical(Editor::new(format!("/{} ", named.owner().name)));
                }
                if let InputBar::Command { menu, .. } = self.active_bar_mut() {
                    *menu = Menu::Commands;
                }
            }
            KeyCode::Up => candidates.advance(true),
            KeyCode::Down => candidates.advance(false),
            KeyCode::Tab => {
                if let Some(editor) = candidates.choose() {
                    input.replace_canonical(editor);
                }
            }
            KeyCode::Enter => {
                let Some(editor) = candidates.choose() else {
                    self.message =
                        crate::message::Message::error(i18n::tr!(NoMatchingOptions).into());
                    return true;
                };
                input.replace_canonical(editor);
                if let InputBar::Command { menu, .. } = self.active_bar_mut() {
                    *menu = Menu::Commands;
                }
                // 候选只补齐值；同一次 Enter 继续走原有解析与确认流程。
                return false;
            }
            _ => return false,
        }
        true
    }
}
