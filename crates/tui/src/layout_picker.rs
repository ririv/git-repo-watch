//! A persisted layout choice preserves its command draft and underlying view.
use crate::{
    Screen,
    bar::{CommandDraft, InputBar},
};
use crossterm::event::{KeyCode, KeyEvent};

impl Screen {
    pub(crate) fn empty_layout_picker_key(&mut self, key: KeyEvent) -> bool {
        if !matches!(key.code, KeyCode::Enter | KeyCode::Tab) {
            return false;
        }
        let InputBar::Command {
            input,
            path,
            selection,
            selected,
            ..
        } = &self.bar
        else {
            return false;
        };
        if !crate::command::argument_input(&input.query()).is_some_and(|(argument, value)| {
            argument.options() == Some(crate::command::options::Kind::DiffLayout)
                && value.trim().is_empty()
        }) {
            return false;
        }
        let draft = CommandDraft {
            input: input.clone(),
            path: path.clone(),
            repository: self.capture_repository(path.as_deref()),
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
            KeyCode::Left | KeyCode::BackTab => *selected = selected.advance(true),
            KeyCode::Right | KeyCode::Tab => *selected = selected.advance(false),
            KeyCode::Enter => {
                let preference = *selected;
                self.set_diff_layout(preference);
                self.sync_bar();
                self.message = i18n::tf!(DiffLayoutSelected, preference.label());
            }
            KeyCode::Esc => {
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
            }
            _ => {}
        }
        true
    }
}
