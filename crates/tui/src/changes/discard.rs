//! Whole-file discard consumes the marked file-list rows or the current file and requires confirmation.
use crate::{Confirmation, Effect, Mode, Outcome, Screen, clipboard, confirmation::Choice};
use engine::local::files::discard::{Action, Plan};
use std::path::Path;

impl Screen {
    pub(crate) fn discard_file_target(
        &self,
        path: Option<&Path>,
        selection: &clipboard::Selection,
    ) -> Result<&crate::FileSession, &'static str> {
        let changes = selection.worktree_files();
        let files = self
            .worktree_file_list()
            .ok_or(i18n::tr!(DiscardFileSelect))?;
        if changes.is_empty() {
            return Err(i18n::tr!(DiscardFileSelect));
        }
        if Some(files.snapshot.path.as_path()) != path || files.discard_selection() != changes {
            return Err(i18n::tr!(PartialChanged));
        }
        if changes
            .iter()
            .any(|change| change.area == crate::Area::Staged)
        {
            return Err(i18n::tr!(CommandNeedsUnstaged));
        }
        if changes
            .iter()
            .any(|change| Action::for_change(change).is_none())
        {
            return Err(i18n::tr!(DiscardFileUnsupported));
        }
        Ok(files)
    }

    pub(crate) fn prepare_discard_file(
        &mut self,
        path: Option<&Path>,
        selection: &clipboard::Selection,
    ) {
        let snapshot = match self.discard_file_target(path, selection) {
            Ok(files) => files.snapshot.clone(),
            Err(reason) => {
                self.message = crate::message::Message::error(reason.into());
                return;
            }
        };
        let changes = selection.worktree_files().to_vec();
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
