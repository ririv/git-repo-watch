//! Conflict choices share the existing bottom input surface, not command completion.
use crate::{Mode, Screen};
use engine::local::files::conflict::resolve::Choice;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Action {
    Accept(Choice),
    Undo,
    Reset,
    Stage,
}
impl Action {
    pub const ALL: [Self; 6] = [
        Self::Accept(Choice::Left),
        Self::Accept(Choice::Right),
        Self::Accept(Choice::Both),
        Self::Undo,
        Self::Reset,
        Self::Stage,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Accept(Choice::Left) => i18n::tr!(ConflictChoiceLeft),
            Self::Accept(Choice::Right) => i18n::tr!(ConflictChoiceRight),
            Self::Accept(Choice::Both) => i18n::tr!(ConflictChoiceBoth),
            Self::Undo => i18n::tr!(ConflictChoiceUndo),
            Self::Reset => i18n::tr!(ConflictChoiceReset),
            Self::Stage => i18n::tr!(ConflictChoiceStage),
        }
    }
    pub fn command(self) -> crate::Action {
        match self {
            Self::Accept(choice) => crate::Action::ConflictAccept(choice),
            Self::Undo => crate::Action::ConflictUndo,
            Self::Reset => crate::Action::ConflictReset,
            Self::Stage => crate::Action::Stage,
        }
    }
    pub fn step(self, backwards: bool) -> Self {
        let index = Self::ALL
            .iter()
            .position(|action| *action == self)
            .expect("known conflict action");
        Self::ALL[(index + if backwards { Self::ALL.len() - 1 } else { 1 }) % Self::ALL.len()]
    }
}
impl Screen {
    pub(crate) fn focused_conflict_control(&self) -> Option<Action> {
        if !self.bar.is_plain()
            || self.focus_mode != crate::focus::State::Inactive
            || !self.has_conflict_page()
        {
            return None;
        }
        self.mode.file_session()?.recovery.conflict()
    }
    pub(crate) fn open_conflict_controls(&mut self) {
        if !self.has_conflict_page() {
            return;
        }
        let action = if self
            .conflict_view()
            .is_some_and(|view| view.resolution.resolved())
        {
            Action::Stage
        } else {
            Action::Accept(Choice::Left)
        };
        self.show_input_page(crate::input_pages::Page::Conflict);
        if let Mode::Changes(session) = &mut self.mode {
            session.files.recovery.choose_conflict(action);
        }
        self.message.clear();
    }
    pub(crate) fn conflict_control_reason(&self, action: Action) -> Option<String> {
        if action == Action::Stage
            && !self
                .conflict_view()
                .is_some_and(|view| view.resolution.resolved())
        {
            return Some(i18n::tr!(ReplayResolveFirst).into());
        }
        self.command_availability(
            action.command(),
            self.scope_path().as_deref(),
            &self.copy_selection(),
        )
        .reason()
        .map(str::to_owned)
    }
    pub(crate) fn perform_conflict_control(&mut self, action: Action) {
        let path = self.scope_path();
        let selection = self.copy_selection();
        if let Some(reason) = self.conflict_control_reason(action) {
            self.message = crate::message::Message::error(reason);
            return;
        }
        if let Mode::Changes(session) = &mut self.mode {
            session.files.recovery.choose_conflict(action);
        }
        if action == Action::Stage {
            self.close_replay_controls();
        }
        if let Err(error) = self.activate(
            action.command(),
            path,
            selection,
            &engine::cancel::Cancel::default(),
        ) {
            self.message = crate::message::Message::error(format!("{error:#}"));
        }
    }
    pub(crate) fn conflict_control_key(&mut self, key: crossterm::event::KeyEvent) -> bool {
        use crossterm::event::KeyCode;
        let Some(selected) = self.focused_conflict_control() else {
            return false;
        };
        match key.code {
            KeyCode::Left | KeyCode::Right if key.modifiers.is_empty() => {
                if let Mode::Changes(session) = &mut self.mode {
                    session
                        .files
                        .recovery
                        .choose_conflict(selected.step(key.code == KeyCode::Left));
                }
            }
            KeyCode::Enter => self.perform_conflict_control(selected),
            KeyCode::Up
            | KeyCode::Down
            | KeyCode::Home
            | KeyCode::End
            | KeyCode::PageUp
            | KeyCode::PageDown => {
                if let Mode::Changes(session) = &mut self.mode
                    && let crate::changes::Preview::Ready {
                        content, scroll, ..
                    } = &mut session.preview
                    && let crate::document::View::Conflict(view) = content.as_mut()
                {
                    view.navigate(key, scroll);
                }
            }
            _ => return false,
        }
        true
    }
}

impl super::View {
    pub(crate) fn navigate(&mut self, key: crossterm::event::KeyEvent, scroll: &mut usize) {
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Up | KeyCode::Down | KeyCode::Home | KeyCode::End
                if !self.resolution.resolved() =>
            {
                let delta = match key.code {
                    KeyCode::Up => -1,
                    KeyCode::Down => 1,
                    KeyCode::Home => isize::MIN,
                    KeyCode::End => isize::MAX,
                    _ => unreachable!(),
                };
                self.resolution.move_cursor(delta);
                self.refresh_selection();
                self.seek_selected(scroll);
            }
            KeyCode::End => *scroll = usize::MAX,
            _ => crate::input::scroll_key(scroll, key),
        }
    }
}
