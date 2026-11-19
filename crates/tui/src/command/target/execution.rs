//! Capture owned command arguments before releasing the borrowed operation context.
use super::{IndexRequest, Resolved, resolved::Value};
use crate::{Action, clipboard};
use engine::local::files::{Change, Files, IndexAction, partial as file_partial};
use std::path::PathBuf;

mod local;
pub(super) mod partial;
mod run;
pub(crate) struct Execution {
    action: Action,
    selection: clipboard::Selection,
    request: Request,
    return_to: crate::bar::ReturnTo,
}
/// An unforgeable capability available only during prepared command dispatch.
pub(crate) struct Permit {
    _private: (),
}
enum Request {
    Task(crate::task_center::Operation),
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
    Discard(self::partial::PreparedPartial),
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
impl Execution {
    pub(crate) fn return_to(mut self, target: crate::bar::ReturnTo) -> Self {
        self.return_to = target;
        self
    }
}
impl Resolved<'_> {
    pub(in crate::command) fn capture(
        self,
        action: Action,
        path: Option<PathBuf>,
        selection: &clipboard::Selection,
    ) -> anyhow::Result<Execution> {
        let request = match (self.value, action) {
            (Value::Task(operation), _) => Request::Task(operation),
            (Value::Repository(repository), _) => Request::Local {
                path: Some(repository.to_owned()),
            },
            (Value::Stash(target), Action::StashPreview(scope)) => Request::StashPreview {
                scope,
                session: target.session.clone(),
                entry: target
                    .session
                    .selected()
                    .ok_or_else(|| anyhow::anyhow!(i18n::tr!(StashIdRequired)))?
                    .clone(),
                previous: target.inspect.clone(),
            },
            (Value::Conflict(target), Action::ConflictAccept(choice)) => Request::ConflictAccept {
                action: crate::conflict::resolution::Action::Accept(choice),
                repository: target.repository.to_owned(),
                resolution: target.resolution.clone(),
            },
            (Value::Conflict(target), Action::ConflictUndo) => Request::ConflictAccept {
                action: crate::conflict::resolution::Action::Undo,
                repository: target.repository.to_owned(),
                resolution: target.resolution.clone(),
            },
            (Value::Conflict(target), Action::ConflictReset) => Request::ConflictReset {
                repository: target.repository.to_owned(),
                resolution: target.resolution.clone(),
            },
            (Value::Destination(destination), Action::OpenEditor) => Request::Editor {
                root: destination.root.to_owned(),
                target: destination.path,
            },
            (Value::Destination(destination), Action::Terminal) => Request::Terminal {
                root: destination.root.to_owned(),
                directory: destination.path,
            },
            (Value::File(file), Action::CopyPath) => Request::Copy(clipboard::Payload {
                target: clipboard::Target::FilePaths,
                text: file
                    .relative_path()
                    .to_str()
                    .expect("validated UTF-8 path")
                    .to_owned(),
            }),
            (Value::Clipboard(scope), Action::Copy) => {
                Request::Copy(selection.payload(scope, path.as_deref())?)
            }
            (Value::Commits(commits), Action::Replay(kind)) => Request::Replay {
                repository: commits.repository.to_owned(),
                commits: match kind {
                    engine::operations::replay::Kind::CherryPick => commits.marked,
                    engine::operations::replay::Kind::Revert => vec![commits.selected],
                },
            },
            (Value::Refresh(target), Action::Refresh) => Request::Refresh(target),
            (Value::Index(target), Action::Stage) => {
                Request::Index(target.prepare(IndexAction::Stage)?)
            }
            (Value::Index(target), Action::Unstage) => {
                Request::Index(target.prepare(IndexAction::Unstage)?)
            }
            (Value::Partial(target), Action::Discard) => {
                Request::Discard(self::partial::PreparedPartial::new(
                    target
                        .editing
                        .snapshot
                        .prepare(file_partial::Action::Discard, target.editing.selected())?,
                ))
            }
            (Value::WorktreeFiles(target), Action::DiscardFile) => Request::DiscardFiles {
                snapshot: target.session.snapshot.as_ref().clone(),
                changes: target.changes.to_vec(),
            },
            (Value::History(graph), Action::LogMore) => Request::MoreHistory(graph.clone()),
            (Value::MergeCommit(commit), Action::LogParent) => {
                Request::HistoryParent(commit.clone())
            }
            (Value::Browser(session), Action::BrowserDiff) => Request::BrowserDiff {
                root: session.root.clone(),
                entry: session.selected().expect("validated changed file").clone(),
            },
            (Value::Previewable(target), Action::BrowserMode(_) | Action::BrowserLayout(_)) => {
                Request::Presentation(target.clone())
            }
            (Value::Composition(super::resolved::Composition::Plan(session)), _) => {
                Request::Local {
                    path: Some(session.prepared.snapshot.path.clone()),
                }
            }
            (Value::Composition(super::resolved::Composition::Waiting(waiting)), _) => {
                Request::Local {
                    path: Some(waiting.path.clone()),
                }
            }
            (
                Value::Global
                | Value::Search(_)
                | Value::Branches(_)
                | Value::History(_)
                | Value::Partial(_)
                | Value::Previewable(_)
                | Value::Conflict(_)
                | Value::Stash(_),
                _,
            ) => Request::Local { path },
            _ => unreachable!("command requirement and resolved capability agree"),
        };
        Ok(Execution {
            action,
            selection: selection.clone(),
            request,
            return_to: crate::bar::ReturnTo::Command,
        })
    }
}
