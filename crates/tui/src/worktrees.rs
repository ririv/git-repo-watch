//! Browse worktrees without moving the source repository's checkout.
use crate::{Action, Mode, Screen};
use crossterm::event::{KeyCode, KeyEvent};
use engine::{
    cancel::Cancel,
    local::worktrees::{Entry, Snapshot},
};
use ratatui::widgets::ListState;

#[derive(Debug, Clone)]
pub(crate) struct Session {
    pub snapshot: Snapshot,
    pub state: ListState,
    pub horizontal: crate::horizontal::Viewport,
}
impl Session {
    pub fn new(snapshot: Snapshot) -> Self {
        let selected = snapshot
            .entries
            .iter()
            .position(|entry| entry.current)
            .or_else(|| (!snapshot.entries.is_empty()).then_some(0));
        Self {
            snapshot,
            state: ListState::default().with_selected(selected),
            horizontal: Default::default(),
        }
    }
    pub fn selected(&self) -> Option<&Entry> {
        self.state
            .selected()
            .and_then(|index| self.snapshot.entries.get(index))
    }
    pub fn refresh(&mut self, snapshot: Snapshot) {
        let selected = self
            .selected()
            .and_then(|entry| {
                snapshot
                    .entries
                    .iter()
                    .position(|next| next.path == entry.path)
            })
            .or_else(|| {
                self.state
                    .selected()
                    .zip(snapshot.entries.len().checked_sub(1))
                    .map(|(index, last)| index.min(last))
            });
        self.snapshot = snapshot;
        self.state
            .select(selected.or_else(|| (!self.snapshot.entries.is_empty()).then_some(0)));
    }
    pub fn advance(&mut self, delta: isize) {
        let last = self.snapshot.entries.len().checked_sub(1);
        self.state.select(last.map(|last| {
            self.state
                .selected()
                .unwrap_or(0)
                .saturating_add_signed(delta)
                .min(last)
        }));
    }
}
impl Screen {
    pub(crate) fn worktree_key(
        &mut self,
        mut session: Session,
        key: KeyEvent,
        cancel: &Cancel,
    ) -> anyhow::Result<bool> {
        match key.code {
            KeyCode::Up => session.advance(-1),
            KeyCode::Down => session.advance(1),
            KeyCode::PageUp => session.advance(-10),
            KeyCode::PageDown => session.advance(10),
            KeyCode::Home => session
                .state
                .select((!session.snapshot.entries.is_empty()).then_some(0)),
            KeyCode::End => session
                .state
                .select(session.snapshot.entries.len().checked_sub(1)),
            _ => {}
        }
        self.mode = Mode::Worktrees(session);
        match key.code {
            KeyCode::Esc | KeyCode::Left => self.return_to_view(),
            KeyCode::Enter | KeyCode::Right => {
                return self.activate(
                    Action::WorktreeOpen,
                    self.command_repository(),
                    self.copy_selection(),
                    cancel,
                );
            }
            _ => {}
        }
        Ok(false)
    }
}
