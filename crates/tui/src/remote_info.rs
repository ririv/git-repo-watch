//! One replaceable local-config reader supplies both repository detail presentations.
use crate::{Page, Screen, reader::Worker};
use engine::local::remotes::Endpoint;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Debug, Default)]
pub(crate) enum Content {
    #[default]
    Closed,
    Loading(PathBuf),
    Ready {
        path: PathBuf,
        result: Result<Vec<Endpoint>, String>,
    },
}
impl Content {
    fn path(&self) -> Option<&Path> {
        match self {
            Self::Closed => None,
            Self::Loading(path) | Self::Ready { path, .. } => Some(path),
        }
    }
}
#[derive(Debug, Default)]
pub(crate) struct RemoteInfo {
    pub content: Content,
    worker: Worker<PathBuf, Vec<Endpoint>>,
}
impl RemoteInfo {
    pub fn invalidate(&mut self, path: &Path) {
        if self.content.path() == Some(path) {
            self.worker.clear();
            self.content = Content::Closed;
        }
    }
    pub fn clear(&mut self) {
        self.worker.clear();
        self.content = Content::Closed;
    }
    pub fn content(&self, path: &Path) -> Option<&Content> {
        (self.content.path() == Some(path)).then_some(&self.content)
    }
}
impl Screen {
    pub(crate) fn poll_remote_info(&mut self) -> bool {
        if self.window_focus == crate::live::WindowFocus::Unfocused {
            return false;
        }
        let path = match self.page {
            Page::Repositories => self.selected_path(),
            Page::Detail => self.workspace.path().map(Path::to_owned),
            _ => None,
        };
        let mut changed = false;
        if self.remote_info.content.path() != path.as_deref() {
            self.remote_info.clear();
            if let Some(path) = &path {
                let root = path.clone();
                let service = Arc::clone(&self.service);
                self.remote_info.content = Content::Loading(path.clone());
                self.remote_info
                    .worker
                    .request(path.clone(), move |cancel| service.remotes(&root, &cancel));
            }
            changed = true;
        }
        if let Some((read, result)) = self.remote_info.worker.poll()
            && path.as_ref() == Some(&read)
        {
            self.remote_info.content = Content::Ready { path: read, result };
            changed = true;
        }
        changed
    }
}
