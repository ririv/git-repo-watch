//! Index commands capture the resolved files, selected lines or conflict result once.
use super::worktree::Index;
use engine::local::files::{Change, Files, IndexAction, partial};

mod execute;
pub(super) struct IndexRequest {
    request: Operation,
}
enum Operation {
    Files {
        snapshot: Files,
        action: IndexAction,
        changes: Vec<Change>,
    },
    Partial(super::execution::partial::PreparedPartial),
    Conflict {
        snapshot: Files,
        change: Change,
        displayed: String,
    },
}
impl Index<'_> {
    pub(super) fn prepare(self, action: IndexAction) -> anyhow::Result<IndexRequest> {
        let request = match self {
            Self::Files {
                session, changes, ..
            } => Operation::Files {
                snapshot: session.snapshot.clone(),
                action,
                changes,
            },
            Self::Partial(target) => Operation::Partial(
                super::execution::partial::PreparedPartial::new(target.editing.snapshot.prepare(
                    match action {
                        IndexAction::Stage => partial::Action::Stage,
                        IndexAction::Unstage => partial::Action::Unstage,
                    },
                    target.editing.selected(),
                )?),
            ),
            Self::Conflict { session, view } => Operation::Conflict {
                snapshot: session.snapshot.clone(),
                change: session
                    .current()
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!(i18n::tr!(ChangesSelectionRequired)))?,
                displayed: view
                    .result
                    .source()
                    .ok_or_else(|| anyhow::anyhow!(i18n::tr!(ReplayResolveFirst)))?
                    .to_owned(),
            },
        };
        Ok(IndexRequest { request })
    }
}
