//! Explicit file searches walk ignored-aware directories in a cancellable worker.
use super::{Entry, FileStatus, Node};
use crate::cancel::Cancel;
use anyhow::{Context, Result};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};
const MAX_MATCHES: usize = 1000;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchExtent {
    Complete,
    Limited,
}
#[derive(Debug)]
pub struct Search {
    pub entries: Vec<Entry>,
    pub extent: SearchExtent,
}
pub fn find(root: &Path, query: &str, cancel: &Cancel) -> Result<Search> {
    cancel.check()?;
    let repo = super::super::open(root)?;
    let mut tracked = BTreeMap::new();
    let mut ancestors = BTreeSet::new();
    for entry in repo.index()?.iter() {
        cancel.check()?;
        let path = PathBuf::from(
            std::str::from_utf8(&entry.path).context(i18n::tr!(GitPathIsNotUtf8And))?,
        );
        for parent in path.ancestors().skip(1) {
            ancestors.insert(parent.to_owned());
        }
        tracked.insert(path, entry.mode == 0o160000);
    }
    let query = query.to_lowercase();
    let mut pending = vec![PathBuf::new()];
    let mut seen = BTreeSet::new();
    let mut entries = Vec::new();
    while let Some(directory) = pending.pop() {
        cancel.check()?;
        let full = super::checked_path(root, &directory)?;
        let mut children = fs::read_dir(full)?.collect::<std::io::Result<Vec<_>>>()?;
        children.sort_by_key(|entry| entry.file_name());
        for child in children {
            cancel.check()?;
            if child.file_name() == ".git" {
                continue;
            }
            let path = directory.join(child.file_name());
            let kind = child.file_type()?;
            let submodule = tracked.get(&path) == Some(&true);
            if kind.is_dir() && !submodule {
                if ancestors.contains(&path) || !repo.is_path_ignored(&path)? {
                    pending.push(path);
                }
                continue;
            }
            seen.insert(path.clone());
            if !path.to_string_lossy().to_lowercase().contains(&query) {
                continue;
            }
            if !tracked.contains_key(&path) && repo.is_path_ignored(&path)? {
                continue;
            }
            let node = if submodule {
                Node::Submodule(FileStatus::read(&repo, &path)?)
            } else if kind.is_file() {
                Node::File(FileStatus::read(&repo, &path)?)
            } else if kind.is_symlink() {
                Node::Symlink(FileStatus::read(&repo, &path)?)
            } else {
                Node::Special
            };
            entries.push(Entry { path, node });
            if entries.len() > MAX_MATCHES {
                return Ok(finish(entries, SearchExtent::Limited));
            }
        }
    }
    for path in tracked.keys() {
        cancel.check()?;
        if !seen.contains(path) && path.to_string_lossy().to_lowercase().contains(&query) {
            entries.push(Entry {
                path: path.clone(),
                node: Node::File(FileStatus::read(&repo, path)?),
            });
            if entries.len() > MAX_MATCHES {
                return Ok(finish(entries, SearchExtent::Limited));
            }
        }
    }
    Ok(finish(entries, SearchExtent::Complete))
}
fn finish(mut entries: Vec<Entry>, extent: SearchExtent) -> Search {
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    entries.truncate(MAX_MATCHES);
    Search { entries, extent }
}
