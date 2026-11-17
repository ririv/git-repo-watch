//! Capture owned command arguments before releasing the borrowed operation context.
use super::{IndexRequest, Resolved};
use crate::{Action, clipboard};
use engine::local::files::{Change, Files, IndexAction, partial};
use std::path::PathBuf;

pub(crate) enum Execution {
    Local {
        path: Option<PathBuf>,
    },
    Editor {
        root: PathBuf,
        target: PathBuf,
    },
    Terminal {
        root: PathBuf,
        directory: PathBuf,
    },
    Copy(clipboard::Payload),
    Replay {
        repository: PathBuf,
        commits: Vec<engine::local::history::Oid>,
    },
    Refresh(crate::view_refresh::Target),
    Index(IndexRequest),
    Discard(partial::Plan),
    DiscardFiles {
        snapshot: Files,
        changes: Vec<Change>,
    },
    MoreHistory(crate::history::Session),
    HistoryParent(crate::history::Commit),
    BrowserDiff {
        root: PathBuf,
        entry: engine::local::browser::Entry,
    },
    Presentation(crate::browser::opening::OpenTarget),
    StashPreview {
        scope: engine::local::stash::preview::Scope,
        session: crate::stashes::Session,
        entry: engine::local::stash::Entry,
        previous: crate::inspection::Session,
    },
    ConflictAccept {
        action: crate::conflict::resolution::Action,
        repository: PathBuf,
        resolution: crate::conflict::resolution::Session,
    },
    ConflictReset {
        repository: PathBuf,
        resolution: crate::conflict::resolution::Session,
    },
}
impl Resolved<'_> {
    pub fn capture(
        self,
        action: Action,
        path: Option<PathBuf>,
        selection: &clipboard::Selection,
    ) -> anyhow::Result<Execution> {
        Ok(match (self, action) {
            (Self::Repository(repository), _) => Execution::Local {
                path: Some(repository.to_owned()),
            },
            (Self::Stash(target), Action::StashPreview(scope)) => Execution::StashPreview {
                scope,
                session: target.session.clone(),
                entry: target
                    .session
                    .selected()
                    .ok_or_else(|| anyhow::anyhow!(i18n::tr!(StashIdRequired)))?
                    .clone(),
                previous: target.inspect.clone(),
            },
            (Self::Conflict(target), Action::ConflictAccept(choice)) => Execution::ConflictAccept {
                action: crate::conflict::resolution::Action::Accept(choice),
                repository: target.repository.to_owned(),
                resolution: target.resolution.clone(),
            },
            (Self::Conflict(target), Action::ConflictUndo) => Execution::ConflictAccept {
                action: crate::conflict::resolution::Action::Undo,
                repository: target.repository.to_owned(),
                resolution: target.resolution.clone(),
            },
            (Self::Conflict(target), Action::ConflictReset) => Execution::ConflictReset {
                repository: target.repository.to_owned(),
                resolution: target.resolution.clone(),
            },
            (Self::Destination(destination), Action::OpenEditor) => Execution::Editor {
                root: destination.root.to_owned(),
                target: destination.path,
            },
            (Self::Destination(destination), Action::Terminal) => Execution::Terminal {
                root: destination.root.to_owned(),
                directory: destination.path,
            },
            (Self::File(file), Action::CopyPath) => Execution::Copy(clipboard::Payload {
                target: clipboard::Target::FilePaths,
                text: file
                    .relative_path()
                    .to_str()
                    .expect("validated UTF-8 path")
                    .to_owned(),
            }),
            (Self::Clipboard(scope), Action::Copy) => {
                Execution::Copy(selection.payload(scope, path.as_deref())?)
            }
            (Self::Commits(commits), Action::Replay(kind)) => Execution::Replay {
                repository: commits.repository.to_owned(),
                commits: match kind {
                    engine::operations::replay::Kind::CherryPick => commits.marked,
                    engine::operations::replay::Kind::Revert => vec![commits.selected],
                },
            },
            (Self::Refresh(target), Action::Refresh) => Execution::Refresh(target),
            (Self::Index(target), Action::Stage) => {
                Execution::Index(target.prepare(IndexAction::Stage)?)
            }
            (Self::Index(target), Action::Unstage) => {
                Execution::Index(target.prepare(IndexAction::Unstage)?)
            }
            (Self::Partial(target), Action::Discard) => Execution::Discard(
                target
                    .editing
                    .snapshot
                    .prepare(partial::Action::Discard, target.editing.selected())?,
            ),
            (Self::WorktreeFiles(target), Action::DiscardFile) => Execution::DiscardFiles {
                snapshot: target.session.snapshot.clone(),
                changes: target.changes.to_vec(),
            },
            (Self::History(graph), Action::LogMore) => Execution::MoreHistory(graph.clone()),
            (Self::MergeCommit(commit), Action::LogParent) => {
                Execution::HistoryParent(commit.clone())
            }
            (Self::Browser(session), Action::BrowserDiff) => Execution::BrowserDiff {
                root: session.root.clone(),
                entry: session.selected().expect("validated changed file").clone(),
            },
            (Self::Previewable(target), Action::BrowserMode(_) | Action::BrowserLayout(_)) => {
                Execution::Presentation(target.clone())
            }
            (Self::Composition(super::resolved::Composition::Plan(session)), _) => {
                Execution::Local {
                    path: Some(session.prepared.snapshot.path.clone()),
                }
            }
            (Self::Composition(super::resolved::Composition::Waiting(waiting)), _) => {
                Execution::Local {
                    path: Some(waiting.path.clone()),
                }
            }
            (
                Self::Global
                | Self::Search(_)
                | Self::Branches(_)
                | Self::History(_)
                | Self::Partial(_)
                | Self::Previewable(_)
                | Self::Conflict(_)
                | Self::Stash(_),
                _,
            ) => Execution::Local { path },
            _ => unreachable!("command requirement and resolved capability agree"),
        })
    }
}
