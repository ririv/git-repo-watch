//! A selected worktree object supplies separate editor and terminal destinations.
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub(crate) enum Target {
    File(PathBuf),
    Directory(PathBuf),
}
impl Target {
    pub fn path(&self, root: &Path) -> PathBuf {
        let (Self::File(path) | Self::Directory(path)) = self;
        root.join(path)
    }
    pub fn directory(&self, root: &Path) -> PathBuf {
        match self {
            Self::Directory(path) => root.join(path),
            Self::File(path) => root.join(path.parent().expect("selected file has a parent")),
        }
    }
    pub fn browser(entry: engine::local::browser::Entry) -> Self {
        match entry.node {
            engine::local::browser::Node::Directory
            | engine::local::browser::Node::Submodule(_) => Self::Directory(entry.path),
            _ => Self::File(entry.path),
        }
    }
}
