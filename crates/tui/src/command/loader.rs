//! Parameter reads run outside input handling and publish only the active request.
use super::{
    completion::Menu,
    values::{Choice, Kind},
};
use crate::{Editor, Service};
use engine::cancel::Cancel;
use std::{path::PathBuf, sync::Arc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Request {
    Directory { editor: Editor, home: PathBuf },
    Values { kind: Kind, path: PathBuf },
    Editors { path: Option<PathBuf> },
}
#[derive(Debug)]
pub(crate) enum Output {
    Directory { editor: Editor, menu: Menu },
    Values { kind: Kind, entries: Vec<Choice> },
}
#[derive(Debug, Default)]
pub(crate) struct Loader {
    worker: crate::reader::Worker<Request, Output>,
    feedback: crate::loading::Feedback<()>,
}
impl Loader {
    pub fn request(&mut self, request: Request, service: &Arc<Service>) {
        self.feedback.begin(None, std::time::Instant::now());
        let source = request.clone();
        let service = Arc::clone(service);
        self.worker
            .request(request, move |cancel| read(source, &service, &cancel));
    }
    pub fn poll(&mut self) -> Option<(Request, Result<Output, String>)> {
        let result = self.worker.poll()?;
        self.feedback.finish();
        Some(result)
    }
    pub fn tick(&mut self) -> bool {
        self.feedback.tick(std::time::Instant::now())
    }
    pub fn visible(&self) -> bool {
        self.feedback.visible()
    }
    pub fn clear(&mut self) {
        self.worker.clear();
        self.feedback.finish();
    }
    #[cfg(test)]
    pub fn pending(&self) -> bool {
        self.worker.pending()
    }
    #[cfg(test)]
    pub fn request_with(
        &mut self,
        request: Request,
        work: impl FnOnce(Cancel) -> anyhow::Result<Output> + Send + 'static,
    ) {
        self.feedback.begin(None, std::time::Instant::now());
        self.worker.request(request, work);
    }
}
fn read(request: Request, service: &Service, cancel: &Cancel) -> anyhow::Result<Output> {
    match request {
        Request::Editors { path } => {
            let root = path.as_deref().unwrap_or(service.scheduler.user_home());
            let environment = engine::editor::Environment::capture(service.scheduler.user_home());
            let current = service.store.config()?.editor;
            let mut entries = engine::editor::detect(&service.git, root, &environment, cancel)?
                .into_iter()
                .map(|entry| Choice {
                    token: entry.preference.value(),
                    label: if entry.preference == current {
                        format!("✓ {}", entry.label)
                    } else {
                        entry.label
                    },
                    description: entry.detail,
                    unavailable: None,
                })
                .collect::<Vec<_>>();
            if !entries.iter().any(|entry| entry.token == current.value()) {
                entries.insert(
                    0,
                    Choice {
                        token: current.value(),
                        label: format!("✓ {}", current.label()),
                        description: i18n::tr!(ExternalEditorCurrent).into(),
                        unavailable: None,
                    },
                );
            }
            Ok(Output::Values {
                kind: Kind::Editor,
                entries,
            })
        }
        Request::Values {
            kind: Kind::Editor, ..
        } => unreachable!("editor request has an optional repository"),
        Request::Directory { mut editor, home } => {
            let menu = super::path::complete_cancellable(&mut editor, &home, cancel)?;
            Ok(Output::Directory { editor, menu })
        }
        Request::Values {
            kind: Kind::Branch,
            path,
        } => {
            let entries = service
                .branch_names(&path, cancel)?
                .into_iter()
                .map(|entry| Choice {
                    token: entry.name.clone(),
                    label: entry.name,
                    description: entry.subject,
                    unavailable: None,
                })
                .collect();
            Ok(Output::Values {
                kind: Kind::Branch,
                entries,
            })
        }
        Request::Values {
            kind: Kind::WorktreeBranch,
            path,
        } => {
            let candidates = service.worktree_add_candidates(&path, cancel)?;
            let mut entries = vec![Choice {
                token: "--new".into(),
                label: i18n::tr!(WorktreeNewBranchChoice).into(),
                description: "--new".into(),
                unavailable: candidates
                    .start
                    .is_none()
                    .then(|| i18n::tr!(WorktreeNewBranchNeedsCommit).into()),
            }];
            entries.extend(candidates.branches.into_iter().map(|entry| Choice {
                token: entry.name.clone(),
                label: entry.name,
                description: entry.subject,
                unavailable: entry.occupied_at.map(|path| {
                    format!("{} {}", i18n::tr!(WorktreeBranchOccupied), path.display())
                }),
            }));
            Ok(Output::Values {
                kind: Kind::WorktreeBranch,
                entries,
            })
        }
        Request::Values {
            kind: Kind::Stash,
            path,
        } => {
            let snapshot = service.stashes(&path, cancel)?;
            let entries = snapshot
                .entries
                .into_iter()
                .map(|entry| Choice {
                    token: entry.oid.to_string(),
                    label: format!("stash@{{{}}} {}", entry.index, &entry.oid.to_string()[..8]),
                    description: entry.message,
                    unavailable: None,
                })
                .collect();
            Ok(Output::Values {
                kind: Kind::Stash,
                entries,
            })
        }
    }
}
