//! Whole-file discard consumes the current file-list row and requires confirmation.
use crate::{Confirmation, Effect, Mode, Outcome, Screen, clipboard, confirmation::Choice};
use engine::local::files::discard::{Action, Plan};
use std::path::Path;

impl Screen {
    pub(crate) fn prepare_discard_file(
        &mut self,
        path: Option<&Path>,
        selection: &clipboard::Selection,
    ) {
        let (Some(files), Some(change)) = (self.worktree_file_list(), selection.worktree_file())
        else {
            self.message = i18n::tr!(DiscardFileSelect).into();
            return;
        };
        if Some(files.snapshot.path.as_path()) != path || files.current() != Some(change) {
            self.message = i18n::tr!(PartialChanged).into();
            return;
        }
        let snapshot = files.snapshot.clone();
        let change = change.clone();
        self.start_repository(
            snapshot.path.clone(),
            i18n::tr!(DiscardFile),
            Effect::ReadOnly,
            move |service, cancel, _| {
                Ok(Outcome::DiscardFilePrepared(Box::new(
                    service.prepare_discard_file(snapshot, &change, &cancel)?,
                )))
            },
        );
    }

    pub(crate) fn review_discard_file(&mut self, plan: Box<Plan>, origin: Mode) {
        let effect = match plan.action() {
            Action::Restore => i18n::tr!(DiscardFileRestoreReview),
            Action::RemoveUntracked => i18n::tr!(DiscardFileRemoveReview),
        };
        let review = format!(
            "{}\n\n{}\n\n{}",
            i18n::tr!(DiscardFile),
            plan.repository().join(plan.file()).display(),
            effect
        );
        self.mode = Mode::Confirm {
            operation: Confirmation::DiscardFile {
                plan,
                review,
                origin: Box::new(origin),
            },
            choice: Choice::Return,
        };
    }

    pub(crate) fn execute_discard_file(&mut self, plan: Box<Plan>) {
        self.start_repository(
            plan.repository().to_owned(),
            i18n::tr!(DiscardFile),
            Effect::RepositoryWrite,
            move |service, cancel, _| Ok(Outcome::Files(service.discard_file(&plan, &cancel)?)),
        );
    }
}
