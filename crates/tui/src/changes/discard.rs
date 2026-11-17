//! Whole-file discard consumes the marked file-list rows or the current file and requires confirmation.
use crate::{Confirmation, Effect, Mode, Outcome, Screen, confirmation::Choice};
use engine::local::files::discard::{Action, Plan};

impl Screen {
    pub(crate) fn prepare_discard_files(
        &mut self,
        snapshot: engine::local::files::Files,
        changes: Vec<engine::local::files::Change>,
        _permit: &crate::command::target::ExecutionPermit,
    ) {
        self.start_repository(
            snapshot.path.clone(),
            i18n::tr!(DiscardFile),
            Effect::ReadOnly,
            move |service, cancel, _| {
                Ok(Outcome::DiscardFilePrepared(Box::new(
                    service.prepare_discard_file(snapshot, &changes, &cancel)?,
                )))
            },
        );
    }

    pub(crate) fn review_discard_file(&mut self, plan: Box<Plan>, origin: Mode) {
        use std::fmt::Write as _;
        let count = plan.entries().len();
        let mut review = format!(
            "{} · {}\n{}\n",
            i18n::tr!(DiscardFile),
            i18n::tf!(CommandTargetFiles, count = count),
            crate::view::safe(&plan.repository().display().to_string()),
        );
        for (action, description) in [
            (Action::Restore, i18n::tr!(DiscardFileRestoreReview)),
            (Action::RemoveUntracked, i18n::tr!(DiscardFileRemoveReview)),
        ] {
            let mut files = plan
                .entries()
                .filter(|(_, effect)| *effect == action)
                .peekable();
            if files.peek().is_some() {
                let _ = writeln!(review, "\n{description}");
                for (path, _) in files {
                    let _ = writeln!(
                        review,
                        "• {}",
                        crate::view::safe(&path.display().to_string())
                    );
                }
            }
        }
        self.mode = Mode::Confirm {
            operation: Confirmation::DiscardFile {
                plan,
                review,
                scroll: 0,
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
