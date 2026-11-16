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
    pub(crate) fn stage_conflict(&mut self) {
        let Mode::Changes(session) = &self.mode else {
            return;
        };
        let Some(view) = self.conflict_view() else {
            return;
        };
        if !view.resolution.resolved() {
            self.message = i18n::tr!(ReplayResolveFirst).into();
            return;
        }
        let Some(change) = session.files.current().cloned() else {
            return;
        };
        let files = session.files.snapshot.clone();
        let Some(displayed) = view.result.source().map(str::to_owned) else {
            return;
        };
        self.start_repository(
            files.path.clone(),
            i18n::tr!(StageSelectedFiles),
            Effect::IndexWrite,
            move |service, cancel, _| {
                let source = engine::local::files::patch::Source::Text {
                    path: change.location.name().to_owned(),
                    text: displayed,
                };
                let capture = engine::local::files::conflict::resolve::Snapshot::capture(
                    &files, &change, &source, &cancel,
                )?;
                anyhow::ensure!(capture.blocks().is_empty(), i18n::tr!(ReplayResolveFirst));
                Ok(Outcome::Files(service.edit_index(
                    &files,
                    engine::local::files::IndexAction::Stage,
                    &[change],
                    &cancel,
                )?))
            },
        );
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
    pub(crate) fn reset_conflict(
        &mut self,
        path: Option<&std::path::Path>,
        selection: &crate::clipboard::Selection,
    ) {
        let Some(resolution) = selection.conflict_resolution().cloned() else {
            self.message = i18n::tr!(ConflictFileRequired).into();
            return;
        };
        let Some(path) = path.map(std::path::Path::to_owned) else {
            self.message = i18n::tr!(ConflictFileRequired).into();
            return;
        };
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
            self.message = i18n::tr!(ConflictResolveChanged).into();
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
            self.message = i18n::tr!(ConflictResolveChanged).into();
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
    pub(crate) fn accept_conflict(
        &mut self,
        action: Action,
        path: Option<&std::path::Path>,
        selection: &crate::clipboard::Selection,
    ) {
        let captured = selection.conflict_resolution();
        let plan = match captured {
            Some(resolution) => match action {
                Action::Accept(choice) => resolution.prepare(choice),
                Action::Undo => resolution
                    .undo()
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!(i18n::tr!(ConflictUndoUnavailable))),
            },
            None => Err(anyhow::anyhow!(i18n::tr!(ConflictFileRequired))),
        };
        let plan = match plan {
            Ok(plan) => plan,
            Err(error) => {
                self.message = format!("{error:#}");
                return;
            }
        };
        let current = self.conflict_plan_current(&plan, path);
        if !current {
            self.message = i18n::tr!(ConflictResolveChanged).into();
            return;
        }
        if let Mode::Changes(session) = &mut self.mode
            && let Preview::Ready { content, .. } = &mut session.preview
            && let View::Conflict(view) = content.as_mut()
        {
            view.resolution = captured
                .expect("validated conflict command capture")
                .clone();
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
