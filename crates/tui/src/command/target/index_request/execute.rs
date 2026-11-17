//! Execute an index request captured by the command target resolver.
use super::{IndexRequest, Operation};
use crate::{Effect, Outcome, Screen};
use engine::local::files::IndexAction;

impl IndexRequest {
    pub(in crate::command::target) fn execute(self, screen: &mut Screen) {
        match self.request {
            Operation::Files {
                snapshot,
                action,
                changes,
            } => {
                screen.start_repository(
                    snapshot.path.clone(),
                    match action {
                        IndexAction::Stage => i18n::tr!(StageSelectedFiles),
                        IndexAction::Unstage => i18n::tr!(UnstageSelectedFiles),
                    },
                    Effect::IndexWrite,
                    move |service, cancel, _| {
                        Ok(Outcome::Files(
                            service.edit_index(&snapshot, action, &changes, &cancel)?,
                        ))
                    },
                );
            }
            Operation::Partial(plan) => plan.execute(screen),
            Operation::Conflict {
                snapshot,
                change,
                displayed,
            } => {
                screen.start_repository(
                    snapshot.path.clone(),
                    i18n::tr!(StageSelectedFiles),
                    Effect::IndexWrite,
                    move |service, cancel, _| {
                        let source = engine::local::files::patch::Source::Text {
                            path: change.location.name().to_owned(),
                            text: displayed,
                        };
                        let capture = engine::local::files::conflict::resolve::Snapshot::capture(
                            &snapshot, &change, &source, &cancel,
                        )?;
                        anyhow::ensure!(capture.blocks().is_empty(), i18n::tr!(ReplayResolveFirst));
                        Ok(Outcome::Files(service.edit_index(
                            &snapshot,
                            IndexAction::Stage,
                            &[change],
                            &cancel,
                        )?))
                    },
                );
            }
        }
    }
}
