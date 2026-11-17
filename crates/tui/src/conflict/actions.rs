//! Built-in resolution uses the repository worker and the existing command menus.
use super::resolution::{Action, Direction};
use crate::{
    Effect, Mode, Outcome, Screen,
    changes::{Focus, Preview},
    document::View,
};
impl Screen {
    pub(crate) fn navigate_conflict(&mut self, direction: Direction) {
        if let Mode::Changes(session) = &mut self.mode
            && let Preview::Ready {
                content, scroll, ..
            } = &mut session.preview
            && let View::Conflict(view) = content.as_mut()
        {
            view.resolution.advance(direction);
            view.refresh_selection();
            view.select(super::Focus::Result, scroll);
            view.seek_selected(scroll);
            session.focus = Focus::Preview;
            self.sync_bar();
        }
    }
    fn conflict_plan_current(
        &self,
        plan: &engine::local::files::conflict::resolve::Plan,
        path: Option<&std::path::Path>,
    ) -> bool {
        match &self.mode {
            Mode::Changes(session) => {
                path == Some(plan.path())
                    && session.files.snapshot.path == plan.path()
                    && session
                        .files
                        .current()
                        .is_some_and(|change| change.location.name() == plan.file())
                    && self
                        .conflict_view()
                        .and_then(|view| view.result.source())
                        .is_some_and(|source| plan.expects(source))
            }
            _ => false,
        }
    }
    pub(crate) fn reset_conflict_target(
        &mut self,
        path: std::path::PathBuf,
        resolution: super::resolution::Session,
        _permit: &crate::command::target::ExecutionPermit,
    ) {
        self.start_repository(
            path,
            i18n::tr!(ConflictResetCommand),
            Effect::ReadOnly,
            move |_, cancel, _| {
                let plan = resolution.reset(&cancel)?;
                let review = plan.reset_review()?;
                Ok(Outcome::ConflictResetPrepared {
                    plan: Box::new(plan),
                    review,
                })
            },
        );
    }
    pub(crate) fn review_conflict_reset(
        &mut self,
        plan: engine::local::files::conflict::resolve::Plan,
        review: String,
    ) {
        if !self.conflict_plan_current(&plan, Some(plan.path())) {
            self.message = crate::message::Message::error(i18n::tr!(ConflictResolveChanged).into());
            return;
        }
        let origin = Box::new(self.take_mode());
        self.mode = Mode::Confirm {
            operation: crate::Confirmation::ConflictReset {
                plan: Box::new(plan),
                review,
                origin,
            },
            choice: crate::confirmation::Choice::Return,
        };
    }
    pub(crate) fn execute_conflict_reset(
        &mut self,
        plan: engine::local::files::conflict::resolve::Plan,
    ) {
        if !self.conflict_plan_current(&plan, Some(plan.path())) {
            self.message = crate::message::Message::error(i18n::tr!(ConflictResolveChanged).into());
            return;
        }
        if let Mode::Changes(session) = &mut self.mode
            && let Preview::Ready { content, .. } = &mut session.preview
            && let View::Conflict(view) = content.as_mut()
        {
            view.resolution.accepting(&plan);
        }
        self.start_repository(
            plan.path().to_owned(),
            i18n::tr!(ConflictResetCommand),
            Effect::RepositoryWrite,
            move |service, cancel, _| Ok(Outcome::Files(service.resolve_conflict(&plan, &cancel)?)),
        );
    }
    pub(crate) fn accept_conflict_target(
        &mut self,
        action: Action,
        repository: std::path::PathBuf,
        resolution: super::resolution::Session,
        _permit: &crate::command::target::ExecutionPermit,
    ) {
        let plan = match action {
            Action::Accept(choice) => resolution.prepare(choice),
            Action::Undo => resolution
                .undo()
                .cloned()
                .ok_or_else(|| anyhow::anyhow!(i18n::tr!(ConflictUndoUnavailable))),
        };
        let plan = match plan {
            Ok(plan) => plan,
            Err(error) => {
                self.message = crate::message::Message::error(format!("{error:#}"));
                return;
            }
        };
        let current = self.conflict_plan_current(&plan, Some(&repository));
        if !current {
            self.message = crate::message::Message::error(i18n::tr!(ConflictResolveChanged).into());
            return;
        }
        if let Mode::Changes(session) = &mut self.mode
            && let Preview::Ready { content, .. } = &mut session.preview
            && let View::Conflict(view) = content.as_mut()
        {
            view.resolution = resolution;
            match action {
                Action::Accept(_) => view.resolution.accepting(&plan),
                Action::Undo => view.resolution.undoing(),
            }
        }
        let title = match action {
            Action::Accept(choice) => choice.label(),
            Action::Undo => i18n::tr!(ConflictUndoCommand),
        };
        self.start_repository(
            plan.path().to_owned(),
            title,
            Effect::RepositoryWrite,
            move |service, cancel, _| Ok(Outcome::Files(service.resolve_conflict(&plan, &cancel)?)),
        );
    }
}
