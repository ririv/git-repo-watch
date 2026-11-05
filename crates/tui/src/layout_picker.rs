//! A persisted layout choice preserves its command draft and underlying view.
use crate::{
    Screen,
    bar::{CommandDraft, InputBar},
};
use crossterm::event::{KeyCode, KeyEvent};

impl Screen {
    pub(crate) fn empty_layout_picker_key(&mut self, key: KeyEvent) -> bool {
        if !matches!(key.code, KeyCode::Enter | KeyCode::BackTab) {
            return false;
        }
        let InputBar::Command {
            input,
            path,
            selection,
            selected,
            repository,
            ..
        } = &self.bar
        else {
            return false;
        };
        if input.prefix_selected() {
            return false;
        }
        let argument =
            crate::command::argument_input(&input.query()).is_some_and(|(argument, value)| {
                argument.options() == Some(crate::command::options::Kind::DiffLayout)
                    && (value.trim().is_empty() || key.code == KeyCode::BackTab)
            });
        let selected_layout = key.code == KeyCode::BackTab
            && self
                .search_commands()
                .and_then(|commands| commands.get(*selected).copied())
                .is_some_and(|command| command.action == crate::Action::DiffLayoutMenu);
        if !argument && !selected_layout {
            return false;
        }
        let draft = CommandDraft {
            input: input.clone(),
            path: path.clone(),
            repository: repository.clone(),
            selection: selection.clone(),
            return_to: crate::bar::ReturnTo::Command,
        };
        self.open_layout_picker(draft, *selected);
        true
    }
    pub(super) fn open_layout_picker(&mut self, draft: CommandDraft, command_cursor: usize) {
        self.bar = InputBar::Layout {
            selected: self.diff_preference,
            command_cursor,
            draft,
        };
        self.message.clear();
    }
    pub(super) fn layout_picker_key(&mut self, key: KeyEvent) -> bool {
        let InputBar::Layout { selected, .. } = &mut self.bar else {
            return false;
        };
        match key.code {
            KeyCode::Left => *selected = selected.advance(true),
            KeyCode::Right => *selected = selected.advance(false),
            KeyCode::Enter => {
                let preference = *selected;
                self.set_diff_layout(preference);
                self.sync_bar();
                self.message = i18n::tf!(DiffLayoutSelected, preference.label());
            }
            KeyCode::Esc | KeyCode::BackTab => {
                self.restore_layout_input();
            }
            KeyCode::Char(_) | KeyCode::Backspace | KeyCode::Delete => {
                self.restore_layout_input();
                return false;
            }
            _ => {}
        }
        true
    }
    pub(crate) fn restore_layout_input(&mut self) {
        let InputBar::Layout {
            draft,
            command_cursor,
            ..
        } = std::mem::take(&mut self.bar)
        else {
            unreachable!()
        };
        self.bar = InputBar::Command {
            input: draft.input,
            path: draft.path,
            selection: draft.selection,
            repository: draft.repository,
            selected: command_cursor,
            menu: Default::default(),
        };
        self.refresh_option_candidates();
    }
}
