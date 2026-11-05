//! Menu actions use frozen marked commits and the existing reviewed mutation pipeline.
use super::{
    View,
    menu::{Menu, Operation, PointerInput},
};
use crate::{Action, Mode, Screen, bar::ReturnTo};
use crossterm::event::{KeyCode, KeyEvent};
use engine::operations::replay::Kind;

impl Screen {
    pub(crate) fn history_actions(&self) -> Option<&Menu> {
        match &self.mode {
            Mode::History(view) => view.session().selection.menu(),
            _ => None,
        }
    }
    fn history_actions_mut(&mut self) -> Option<&mut Menu> {
        match &mut self.mode {
            Mode::History(view) => match view.as_mut() {
                View::Graph(session) => session.selection.menu_mut(),
                _ => None,
            },
            _ => None,
        }
    }
    pub(crate) fn open_history_actions(&mut self) {
        let Some(path) = self.scope_path() else {
            return;
        };
        let allowed =
            crate::replay::admits(self.replay_state(&path), Action::Replay(Kind::CherryPick));
        let Mode::History(view) = &mut self.mode else {
            return;
        };
        let View::Graph(session) = view.as_mut() else {
            return;
        };
        let commits = session.picks();
        if !session.selection.open_actions(commits, allowed) {
            self.message = i18n::tr!(HistorySelectionRequired).into();
        } else {
            self.message.clear();
            self.clicks.reset();
        }
    }
    pub(crate) fn close_history_actions(&mut self) -> bool {
        let Mode::History(view) = &mut self.mode else {
            return false;
        };
        let View::Graph(session) = view.as_mut() else {
            return false;
        };
        if session.selection.menu().is_none() {
            return false;
        }
        session.selection.close_actions();
        self.sync_bar();
        true
    }
    pub(crate) fn history_actions_key(&mut self, key: KeyEvent) -> bool {
        if self.history_actions().is_some()
            && matches!(
                key.code,
                KeyCode::Char(_) | KeyCode::Backspace | KeyCode::Delete
            )
        {
            let command = self.command_trigger_key(key).code == KeyCode::Char('/');
            self.close_history_actions();
            if command {
                self.open_command_bar();
                return true;
            }
            return false;
        }
        let Some(menu) = self.history_actions_mut() else {
            return false;
        };
        let input = match key.code {
            KeyCode::Esc | KeyCode::BackTab => PointerInput::Close,
            KeyCode::Up => {
                menu.advance(-1);
                PointerInput::Stay
            }
            KeyCode::Down => {
                menu.advance(1);
                PointerInput::Stay
            }
            KeyCode::Home => {
                menu.seek(0);
                PointerInput::Stay
            }
            KeyCode::End => {
                menu.seek(usize::MAX);
                PointerInput::Stay
            }
            KeyCode::Enter => match menu.state.selected() {
                Some(index) => PointerInput::Choose(index),
                None => PointerInput::Stay,
            },
            _ => PointerInput::Stay,
        };
        self.view_revision += 1;
        self.history_menu_pointer(input);
        true
    }
    pub(crate) fn seek_history_action(&mut self, index: usize) -> bool {
        let Some(menu) = self.history_actions_mut() else {
            return false;
        };
        menu.seek(index);
        *menu.state.offset_mut() = index.min(menu.operations().len() - 1);
        true
    }
    fn preview_history_action(&mut self, kind: Kind, commits: Vec<engine::local::history::Oid>) {
        let mut draft = self.mutation_draft(Action::Replay(kind), self.scope_path());
        draft.return_to = ReturnTo::View;
        self.prepare_replay(kind, commits, draft);
    }
    pub(crate) fn history_menu_pointer(&mut self, input: PointerInput) -> bool {
        let index = match input {
            PointerInput::Close => return self.close_history_actions(),
            PointerInput::Stay => return false,
            PointerInput::Choose(index) => index,
        };
        let Some(menu) = self.history_actions() else {
            return false;
        };
        let Some(operation) = menu.choose(index) else {
            return false;
        };
        let commits = menu.commits();
        self.close_history_actions();
        self.message.clear();
        match operation {
            Operation::CherryPick => self.preview_history_action(Kind::CherryPick, commits),
            Operation::Revert(oid) => self.preview_history_action(Kind::Revert, vec![oid]),
            Operation::Copy => {
                let selection = self.copy_selection();
                self.copy_to_clipboard(crate::clipboard::Scope::Log, selection, self.scope_path());
            }
            Operation::Clear => self.mark_history(super::SelectionAction::Clear),
            Operation::Done => self.mark_history(super::SelectionAction::Done),
        }
        true
    }
}
