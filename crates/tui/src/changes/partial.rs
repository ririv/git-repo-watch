//! Route selected text operations through repository-scoped workers and explicit discard review.
use super::{Focus, Preview, edit};
use crate::{Mode, Screen};
use engine::local::files::{Area, partial};

impl Screen {
    pub(crate) fn partial_mode(&self) -> Option<edit::Granularity> {
        match &self.mode {
            Mode::Changes(session) => match &session.preview {
                Preview::Ready {
                    editing: edit::State::Ready(editing),
                    ..
                } => Some(editing.mode()),
                _ => None,
            },
            _ => None,
        }
    }
    pub(crate) fn select_partial(&mut self, mode: edit::Granularity) {
        let Mode::Changes(session) = &mut self.mode else {
            self.message = crate::message::Message::error(i18n::tr!(PartialSelectRequired).into());
            return;
        };
        match &mut session.preview {
            Preview::Ready {
                editing: edit::State::Ready(editing),
                ..
            } => {
                editing.set_mode(mode);
                if editing.selected().is_empty() {
                    editing.advance(0, false);
                }
                session.focus = Focus::Preview;
            }
            Preview::Ready {
                editing: edit::State::Unavailable(reason),
                ..
            } => self.message = crate::message::Message::error(reason.clone()),
            _ => {
                self.message =
                    crate::message::Message::error(i18n::tr!(PartialSelectRequired).into())
            }
        }
    }
    pub(crate) fn edit_partial(&mut self, action: partial::Action) {
        let action = match action {
            partial::Action::Stage => crate::Action::Stage,
            partial::Action::Unstage => crate::Action::Unstage,
            partial::Action::Discard => crate::Action::Discard,
        };
        let path = self.scope_path();
        let selection = self.copy_selection();
        if let Err(error) =
            self.activate(action, path, selection, &engine::cancel::Cancel::default())
        {
            self.message = crate::message::Message::error(format!("{error:#}"));
        }
    }
    pub(crate) fn toggle_partial(&mut self) {
        let action = match &self.mode {
            Mode::Changes(session) => match session.files.current().map(|change| change.area) {
                Some(Area::Staged) => partial::Action::Unstage,
                Some(Area::Unstaged | Area::Untracked) => partial::Action::Stage,
                None => {
                    self.message =
                        crate::message::Message::error(i18n::tr!(PartialSelectRequired).into());
                    return;
                }
            },
            _ => return,
        };
        self.edit_partial(action);
    }
}
