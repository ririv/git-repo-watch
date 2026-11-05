//! Value completion owns selection and execution without adding catalog commands.
use super::*;
use command::{
    completion::Menu,
    options::{Candidates, Kind},
};

impl Screen {
    pub(crate) fn refresh_option_candidates(&mut self) {
        let candidates = match &self.bar {
            InputBar::Command {
                input, selection, ..
            } => match command::argument_input(&input.query()).and_then(|(argument, value)| {
                argument
                    .options()
                    .filter(|kind| *kind != Kind::DiffLayout || !value.trim().is_empty())
            }) {
                Some(kind)
                    if kind.action().available(self.config.ai.enabled)
                        && selection.in_scope(kind.action())
                        && self.search_support().admits(kind.action()) =>
                {
                    Candidates::new(input, kind, self)
                }
                _ => None,
            },
            _ => None,
        };
        if let InputBar::Command { menu, .. } = &mut self.bar {
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
        self.bar = InputBar::Command {
            input: command::Input::from_canonical(
                self.command_context(),
                Editor::new(format!("/{} ", kind.command().name)),
            ),
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
            } = &self.bar
            && let Some(option) = candidates.options.get(candidates.selected)
            && let Some(reason) = self
                .command_availability(option.value.action(), path.as_deref(), selection)
                .reason()
        {
            self.message = reason.to_owned();
            return true;
        }
        if key.code == KeyCode::Enter
            && matches!(&self.bar, InputBar::Command { input, menu: Menu::Commands, .. }
                if command::argument_input(&input.query()).is_some_and(|(argument, value)|
                    argument.options().is_some() && value.trim().is_empty()))
        {
            self.refresh_option_candidates();
            if matches!(
                self.bar,
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
        } = &mut self.bar
        else {
            return false;
        };
        match key.code {
            KeyCode::Esc => {
                if let InputBar::Command { menu, .. } = &mut self.bar {
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
                    self.message = i18n::tr!(NoMatchingOptions).into();
                    return true;
                };
                input.replace_canonical(editor);
                if let InputBar::Command { menu, .. } = &mut self.bar {
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
