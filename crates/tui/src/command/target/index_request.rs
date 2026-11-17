//! Index commands capture the resolved files, selected lines or conflict result once.
use super::worktree::Index;
use engine::local::files::{Change, Files, IndexAction, partial};

pub(crate) enum IndexRequest {
    Files {
        snapshot: Files,
        action: IndexAction,
        changes: Vec<Change>,
    },
    Partial(partial::Plan),
    Conflict {
        snapshot: Files,
        change: Change,
        displayed: String,
    },
}
impl Index<'_> {
    pub fn prepare(self, action: IndexAction) -> anyhow::Result<IndexRequest> {
        Ok(match self {
            Self::Files {
                session, changes, ..
            } => IndexRequest::Files {
                snapshot: session.snapshot.clone(),
                action,
                changes,
            },
            Self::Partial(target) => IndexRequest::Partial(target.editing.snapshot.prepare(
                match action {
                    IndexAction::Stage => partial::Action::Stage,
                    IndexAction::Unstage => partial::Action::Unstage,
                },
                target.editing.selected(),
            )?),
            Self::Conflict { session, view } => IndexRequest::Conflict {
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
        })
    }
}
