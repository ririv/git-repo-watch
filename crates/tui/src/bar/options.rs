//! Value completion owns selection and execution without adding catalog commands.
use super::*;
use command::{
    ArgumentAction,
    completion::Menu,
    options::{Candidates, Kind},
};

impl Screen {
    pub(crate) fn refresh_option_candidates(&mut self) {
        let candidates = match &self.bar {
            InputBar::Command {
                input,
                path,
                selection,
                ..
            } => match command::argument_input(&input.query()) {
                Some((ArgumentAction::Option(kind), _))
                    if selection.admits(kind.action(), path.as_deref())
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
            path,
            selection,
        };
        self.refresh_option_candidates();
    }
    pub(super) fn option_candidate_key(&mut self, key: KeyEvent, root: &Cancel) -> Result<bool> {
        let InputBar::Command {
            menu: Menu::Options(candidates),
            input,
            path,
            selection,
            ..
        } = &mut self.bar
        else {
            return Ok(false);
        };
        match key.code {
            KeyCode::Esc => {
                if let InputBar::Command { menu, .. } = &mut self.bar {
                    *menu = Menu::Commands;
                }
            }
            KeyCode::Up | KeyCode::BackTab => candidates.advance(true),
            KeyCode::Down => candidates.advance(false),
            KeyCode::Tab => {
                if let Some(editor) = candidates.choose() {
                    input.replace_canonical(editor);
                }
            }
            KeyCode::Enter => {
                let Some(option) = candidates.options.get(candidates.selected) else {
                    self.message = i18n::tr!(NoMatchingOptions).into();
                    return Ok(true);
                };
                let action = option.value.action();
                let path = path.clone();
                let selection = selection.clone();
                self.sync_bar();
                self.message.clear();
                self.activate(action, path, selection, root)?;
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
}
