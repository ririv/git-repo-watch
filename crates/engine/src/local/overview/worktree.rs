//! Local status counts, including unresolved conflicts and dirty submodules.
use crate::cancel::Cancel;
use anyhow::{Context, Result};
use git2::{Repository, Status, StatusOptions};

#[derive(Debug, Default)]
pub struct Changes {
    pub staged: usize,
    pub unstaged: usize,
    pub untracked: usize,
    pub conflicts: usize,
}

pub(super) fn read(repo: &Repository, cancel: &Cancel) -> Result<Changes> {
    let mut options = StatusOptions::new();
    options
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(false)
        .renames_head_to_index(true)
        .renames_index_to_workdir(true)
        .exclude_submodules(false);
    let statuses = repo.statuses(Some(&mut options))?;
    let mut changes = Changes::default();
    let mut worktree_paths = std::collections::BTreeSet::new();
    for entry in statuses.iter() {
        cancel.check()?;
        let status = entry.status();
        let path = entry
            .index_to_workdir()
            .or_else(|| entry.head_to_index())
            .and_then(|delta| delta.new_file().path().or_else(|| delta.old_file().path()))
            .context(i18n::tr!(ChangeHasNoPath))?
            .to_owned();
        if status.contains(Status::CONFLICTED) {
            changes.conflicts += 1;
            worktree_paths.insert(path);
            continue;
        }
        if status.intersects(
            Status::INDEX_NEW
                | Status::INDEX_MODIFIED
                | Status::INDEX_DELETED
                | Status::INDEX_RENAMED
                | Status::INDEX_TYPECHANGE,
        ) {
            changes.staged += 1;
        }
        if status.intersects(
            Status::WT_MODIFIED | Status::WT_DELETED | Status::WT_RENAMED | Status::WT_TYPECHANGE,
        ) {
            changes.unstaged += 1;
            worktree_paths.insert(path.clone());
        }
        if status.contains(Status::WT_NEW) {
            changes.untracked += 1;
            worktree_paths.insert(path);
        }
    }
    for path in crate::local::files::submodule_changes(repo)? {
        cancel.check()?;
        if worktree_paths.insert(path.clone()) {
            changes.unstaged += 1;
        }
    }
    Ok(changes)
}
