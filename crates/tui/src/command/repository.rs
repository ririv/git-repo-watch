//! Command drafts share one frozen, asynchronously resolved repository identity.
use crate::{Service, reader::Worker};
use engine::local::head::Head;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
};

#[derive(Debug, Clone, Default)]
pub(crate) enum Captured {
    #[default]
    None,
    Repository {
        path: PathBuf,
        head: Arc<OnceLock<Result<Head, String>>>,
    },
}
impl PartialEq for Captured {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::None, Self::None) => true,
            (Self::Repository { head: a, .. }, Self::Repository { head: b, .. }) => {
                Arc::ptr_eq(a, b)
            }
            _ => false,
        }
    }
}
impl Eq for Captured {}
impl Captured {
    pub fn new(path: Option<PathBuf>) -> Self {
        match path {
            Some(path) => Self::Repository {
                path,
                head: Arc::new(OnceLock::new()),
            },
            None => Self::None,
        }
    }
    pub fn matches(&self, root: &Path) -> bool {
        matches!(self, Self::Repository { path, .. } if path == root)
    }
    pub fn label(&self, root: &Path) -> Option<String> {
        let Self::Repository { path, head } = self else {
            return None;
        };
        if path != root {
            return None;
        }
        let name = path
            .file_name()
            .unwrap_or(path.as_os_str())
            .to_string_lossy();
        let branch = match head.get() {
            None => i18n::tr!(CommandTargetReadingBranch).into(),
            Some(Err(_)) => i18n::tr!(CommandTargetBranchFailed).into(),
            Some(Ok(Head::Branch(name) | Head::Unborn(name))) => name.clone(),
            Some(Ok(Head::Detached(oid))) => format!("HEAD@{}", &oid[..8]),
        };
        Some(format!("{name} · {branch}"))
    }
    #[cfg(test)]
    pub fn resolved(&self) -> bool {
        matches!(self, Self::Repository { head, .. } if head.get().is_some())
    }
}
#[derive(Default)]
pub(crate) struct Reader {
    worker: Worker<Captured, Head>,
}
impl Reader {
    pub fn poll(&mut self, captured: &Captured, service: &Arc<Service>) -> bool {
        let mut changed = false;
        if let Some((Captured::Repository { head, .. }, result)) = self.worker.poll() {
            let _ = head.set(result);
            changed = true;
        }
        if let Captured::Repository { path, head } = captured {
            if head.get().is_none() && self.worker.requested() != Some(captured) {
                let path = path.clone();
                let service = Arc::clone(service);
                self.worker.request(captured.clone(), move |cancel| {
                    service.command_head(&path, &cancel)
                });
            }
        } else {
            self.worker.clear();
        }
        changed
    }
    pub fn clear(&mut self) {
        self.worker.clear();
    }
}
