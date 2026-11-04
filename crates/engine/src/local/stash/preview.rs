//! Inspect immutable stash layers without reading or modifying the current worktree.
use super::Entry;
use crate::{
    cancel::Cancel,
    local::{history, open},
};
use anyhow::{Result, ensure};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    All,
    Staged,
    Unstaged,
}
impl Scope {
    pub fn label(self) -> &'static str {
        match self {
            Self::All => i18n::tr!(StashPreviewAll),
            Self::Staged => i18n::tr!(StashPreviewStaged),
            Self::Unstaged => i18n::tr!(StashPreviewUnstaged),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Layer {
    Working,
    Index,
    Unstaged,
    Untracked,
}
impl Layer {
    pub fn labels(self) -> (&'static str, &'static str) {
        match self {
            Self::Working => (i18n::tr!(StashBase), i18n::tr!(StashSavedWorktree)),
            Self::Index => (i18n::tr!(StashBase), i18n::tr!(StashSavedIndex)),
            Self::Unstaged => (i18n::tr!(StashSavedIndex), i18n::tr!(StashSavedWorktree)),
            Self::Untracked => (i18n::tr!(StashAbsent), i18n::tr!(StashSavedUntracked)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct File {
    pub change: history::Change,
    pub layer: Layer,
    comparison: Arc<history::Detail>,
}
#[derive(Debug, Clone)]
pub struct Detail {
    pub path: PathBuf,
    pub entry: Entry,
    pub scope: Scope,
    pub base: git2::Oid,
    pub files: Vec<File>,
}

pub fn read(path: &Path, entry: &Entry, scope: Scope, cancel: &Cancel) -> Result<Detail> {
    cancel.check()?;
    let repo = open(path)?;
    let stash = repo.find_commit(entry.oid)?;
    ensure!(
        (2..=3).contains(&stash.parent_count()),
        i18n::tr!(StashInvalidStructure)
    );
    let base = stash.parent_id(0)?;
    let index = stash.parent(1)?;
    ensure!(
        index.parent_count() == 1 && index.parent_id(0)? == base,
        i18n::tr!(StashInvalidStructure)
    );
    let (oid, parent, layer) = match scope {
        Scope::All => (stash.id(), history::Parent::First, Layer::Working),
        Scope::Staged => (index.id(), history::Parent::First, Layer::Index),
        Scope::Unstaged => (
            stash.id(),
            history::Parent::Commit(index.id()),
            Layer::Unstaged,
        ),
    };
    let mut files = Vec::new();
    let mut append = |oid, parent, layer| -> Result<()> {
        let comparison = Arc::new(history::detail(path, oid, parent, cancel)?);
        files.extend(comparison.changes.iter().cloned().map(|change| File {
            change,
            layer,
            comparison: comparison.clone(),
        }));
        Ok(())
    };
    append(oid, parent, layer)?;
    if stash.parent_count() == 3 {
        let untracked = stash.parent(2)?;
        ensure!(
            untracked.parent_count() == 0,
            i18n::tr!(StashInvalidStructure)
        );
        if scope != Scope::Staged {
            append(untracked.id(), history::Parent::First, Layer::Untracked)?;
        }
    }
    files.sort_by(|a, b| a.change.location.name().cmp(b.change.location.name()));
    Ok(Detail {
        path: path.into(),
        entry: entry.clone(),
        scope,
        base,
        files,
    })
}

pub fn document(
    detail: &Detail,
    index: usize,
    cancel: &Cancel,
) -> Result<crate::local::files::patch::Document> {
    let file = detail
        .files
        .get(index)
        .ok_or_else(|| anyhow::anyhow!(i18n::tr!(FileDoesNotBelongToThisSnapshot)))?;
    history::document(&file.comparison, &file.change, cancel)
}
