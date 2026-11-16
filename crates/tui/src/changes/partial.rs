//! Route selected text operations through repository-scoped workers and explicit discard review.
use super::{Focus, Preview, edit};
use crate::{Effect, Mode, Outcome, Screen};
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
        let Mode::Changes(session) = &self.mode else {
            self.message = crate::message::Message::error(i18n::tr!(PartialSelectRequired).into());
            return;
        };
        if session.focus != Focus::Preview {
            self.message = crate::message::Message::error(i18n::tr!(PartialSelectRequired).into());
            return;
        }
        let plan = match &session.preview {
            Preview::Ready {
                editing: edit::State::Ready(editing),
                ..
            } => editing.snapshot.prepare(action, editing.selected()),
            Preview::Ready {
                editing: edit::State::Unavailable(reason),
                ..
            } => Err(anyhow::anyhow!(reason.clone())),
            _ => Err(anyhow::anyhow!(i18n::tr!(PartialSelectRequired))),
        };
        let plan = match plan {
            Ok(plan) => plan,
            Err(error) => {
                self.message = crate::message::Message::error(format!("{error:#}"));
                return;
            }
        };
        if action == partial::Action::Discard {
            let review = match plan.preview() {
                Ok(patch) => format!(
                    "{}\n\n{}\n\n{patch}",
                    i18n::tr!(PartialDiscardConfirm),
                    plan.snapshot().change().location.display()
                ),
                Err(error) => {
                    self.message = crate::message::Message::error(format!("{error:#}"));
                    return;
                }
            };
            let origin = Box::new(self.take_mode());
            self.mode = Mode::Confirm {
                operation: crate::Confirmation::Partial {
                    plan: Box::new(plan),
                    review,
                    origin,
                },
                choice: crate::confirmation::Choice::Return,
            };
        } else {
            self.execute_partial(plan);
        }
    }
    pub(crate) fn execute_partial(&mut self, plan: partial::Plan) {
        let (title, effect) = match plan.action() {
            partial::Action::Stage => (i18n::tr!(PartialStage), Effect::IndexWrite),
            partial::Action::Unstage => (i18n::tr!(PartialUnstage), Effect::IndexWrite),
            partial::Action::Discard => (i18n::tr!(PartialDiscard), Effect::RepositoryWrite),
        };
        self.start_repository(
            plan.snapshot().files().path.clone(),
            title,
            effect,
            move |service, cancel, _| Ok(Outcome::Files(service.edit_partial(&plan, &cancel)?)),
        );
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
