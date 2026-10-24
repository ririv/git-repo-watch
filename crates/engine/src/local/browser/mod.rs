//! One-level worktree browsing with index membership and Git ignore semantics.
mod content;
mod search;
mod status;
use crate::cancel::Cancel;
use anyhow::{Context, Result, ensure};
pub use content::{Content, read};
pub use search::{Search, SearchExtent, find};
pub use status::{Delta, FileStatus};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    Directory,
    File(FileStatus),
    Symlink(FileStatus),
    Submodule(FileStatus),
    Special,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub path: PathBuf,
    pub node: Node,
}
impl Entry {
    pub fn directory(&self) -> bool {
        matches!(self.node, Node::Directory)
    }
    pub fn status(&self) -> Option<FileStatus> {
        match self.node {
            Node::File(status) | Node::Symlink(status) | Node::Submodule(status) => Some(status),
            Node::Directory | Node::Special => None,
        }
    }
}
#[derive(Debug)]
pub struct Directory {
    pub path: PathBuf,
    pub entries: Vec<Entry>,
}

fn checked_path(root: &Path, relative: &Path) -> Result<PathBuf> {
    ensure!(
        relative
            .components()
            .all(|part| matches!(part, Component::Normal(_))),
        i18n::tr!(BrowserInvalidPath)
    );
    let mut current = root.to_owned();
    let components = relative.components().collect::<Vec<_>>();
    for component in components.iter().take(components.len().saturating_sub(1)) {
        current.push(component);
        match fs::symlink_metadata(&current) {
            Ok(metadata) => ensure!(
                metadata.is_dir() && !metadata.is_symlink(),
                i18n::tr!(BrowserInvalidPath)
            ),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
            Err(error) => return Err(error.into()),
        }
    }
    Ok(root.join(relative))
}

pub fn directory(root: &Path, relative: &Path, cancel: &Cancel) -> Result<Directory> {
    cancel.check()?;
    let repo = super::open(root)?;
    let full = checked_path(root, relative)?;
    if let Ok(metadata) = fs::symlink_metadata(&full) {
        ensure!(
            metadata.is_dir() && !metadata.is_symlink(),
            i18n::tr!(BrowserInvalidPath)
        );
    }
    let mut tracked = BTreeMap::new();
    let mut submodules = BTreeSet::new();
    // 只投影当前层的索引路径；不会为了构造树而递归读取工作区。
    for entry in repo.index()?.iter() {
        cancel.check()?;
        let path = PathBuf::from(
            std::str::from_utf8(&entry.path).context(i18n::tr!(GitPathIsNotUtf8And))?,
        );
        if entry.mode == 0o160000 {
            submodules.insert(path.clone());
        }
        let Ok(suffix) = path.strip_prefix(relative) else {
            continue;
        };
        let mut components = suffix.components();
        let Some(first) = components.next() else {
            continue;
        };
        let child = relative.join(first);
        let is_directory = components.next().is_some();
        tracked
            .entry(child)
            .and_modify(|directory| *directory |= is_directory)
            .or_insert(is_directory);
    }
    let mut nodes = BTreeMap::new();
    match fs::read_dir(&full) {
        Ok(entries) => {
            for entry in entries {
                cancel.check()?;
                let entry = entry?;
                if entry.file_name() == ".git" {
                    continue;
                }
                let path = relative.join(entry.file_name());
                if !tracked.contains_key(&path) && repo.is_path_ignored(&path)? {
                    continue;
                }
                let kind = entry.file_type()?;
                let node = if submodules.contains(&path) {
                    Node::Submodule(FileStatus::read(&repo, &path)?)
                } else if kind.is_dir() {
                    Node::Directory
                } else if kind.is_symlink() {
                    Node::Symlink(FileStatus::read(&repo, &path)?)
                } else if kind.is_file() {
                    Node::File(FileStatus::read(&repo, &path)?)
                } else {
                    Node::Special
                };
                nodes.insert(path, node);
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound && !tracked.is_empty() => {}
        Err(error) => return Err(error.into()),
    }
    for (path, directory) in tracked {
        if let std::collections::btree_map::Entry::Vacant(slot) = nodes.entry(path) {
            let node = if directory {
                Node::Directory
            } else {
                Node::File(FileStatus::read(&repo, slot.key())?)
            };
            slot.insert(node);
        }
    }
    let mut entries = nodes
        .into_iter()
        .map(|(path, node)| Entry { path, node })
        .collect::<Vec<_>>();
    entries.sort_by(|a, b| (!a.directory(), &a.path).cmp(&(!b.directory(), &b.path)));
    cancel.check()?;
    Ok(Directory {
        path: relative.to_owned(),
        entries,
    })
}

#[cfg(test)]
mod tests;
