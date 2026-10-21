//! File snapshots; index mutation and diff rendering are separate capabilities.
mod diff;
mod index;
use super::{Head, head, idle, open, utf8_path};
use anyhow::{Context, Result, ensure};
pub use diff::diff;
use git2::{Delta, Oid, Repository, Status, StatusOptions};
pub use index::edit;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Area {
    Staged,
    Unstaged,
    Untracked,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Location {
    Path(PathBuf),
    Rename { before: PathBuf, after: PathBuf },
    Copy { before: PathBuf, after: PathBuf },
}
impl Location {
    pub fn name(&self) -> &Path {
        match self {
            Self::Path(p) => p,
            Self::Rename { after, .. } | Self::Copy { after, .. } => after,
        }
    }
    pub fn paths(&self) -> Vec<&Path> {
        match self {
            Self::Path(p) => vec![p],
            Self::Rename { before, after } => vec![before, after],
            Self::Copy { after, .. } => vec![after],
        }
    }
    pub fn display(&self) -> String {
        match self {
            Self::Path(p) => format!("{:?}", p),
            Self::Rename { before, after } | Self::Copy { before, after } => {
                format!("{before:?} → {after:?}")
            }
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    pub area: Area,
    pub kind: char,
    pub location: Location,
}
#[derive(Debug, Clone)]
pub struct Files {
    pub path: PathBuf,
    pub(crate) head: Head,
    pub(crate) tree: Oid,
    pub changes: Vec<Change>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexAction {
    Stage,
    Unstage,
}
fn location(delta: git2::DiffDelta<'_>) -> Result<Location> {
    let after = delta
        .new_file()
        .path()
        .or_else(|| delta.old_file().path())
        .context(gw_i18n::tr!(ChangeHasNoPath))?
        .to_owned();
    utf8_path(&after)?;
    Ok(match delta.status() {
        Delta::Renamed => Location::Rename {
            before: delta
                .old_file()
                .path()
                .context(gw_i18n::tr!(RenameHasNoOriginalPath))?
                .to_owned(),
            after,
        },
        Delta::Copied => Location::Copy {
            before: delta
                .old_file()
                .path()
                .context(gw_i18n::tr!(CopyHasNoOriginalPath))?
                .to_owned(),
            after,
        },
        _ => Location::Path(after),
    })
}
fn changes(repo: &Repository) -> Result<Vec<Change>> {
    let mut options = StatusOptions::new();
    options
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(false)
        .renames_head_to_index(true)
        .renames_index_to_workdir(true)
        .exclude_submodules(false);
    let statuses = repo.statuses(Some(&mut options))?;
    let mut result = Vec::new();
    for entry in statuses.iter() {
        let s = entry.status();
        ensure!(
            !s.contains(Status::CONFLICTED),
            gw_i18n::tr!(RepositoryHasUnresolvedConflicts)
        );
        if s.intersects(
            Status::INDEX_NEW
                | Status::INDEX_MODIFIED
                | Status::INDEX_DELETED
                | Status::INDEX_RENAMED
                | Status::INDEX_TYPECHANGE,
        ) {
            let delta = entry
                .head_to_index()
                .context(gw_i18n::tr!(StagedDiffIsMissing))?;
            let kind = kind(delta.status());
            result.push(Change {
                area: Area::Staged,
                kind,
                location: location(delta)?,
            });
        }
        if s.intersects(
            Status::WT_NEW
                | Status::WT_MODIFIED
                | Status::WT_DELETED
                | Status::WT_RENAMED
                | Status::WT_TYPECHANGE,
        ) {
            let delta = entry
                .index_to_workdir()
                .context(gw_i18n::tr!(WorktreeDiffIsMissing))?;
            let area = if s.contains(Status::WT_NEW) {
                Area::Untracked
            } else {
                Area::Unstaged
            };
            let kind = if area == Area::Untracked {
                '?'
            } else {
                kind(delta.status())
            };
            result.push(Change {
                area,
                kind,
                location: location(delta)?,
            });
        }
    }
    for path in submodule_changes(repo)? {
        if !result
            .iter()
            .any(|c| c.area != Area::Staged && c.location.name() == path)
        {
            result.push(Change {
                area: Area::Unstaged,
                kind: 'M',
                location: Location::Path(path),
            });
        }
    }
    Ok(result)
}
fn kind(delta: Delta) -> char {
    match delta {
        Delta::Added => 'A',
        Delta::Deleted => 'D',
        Delta::Renamed => 'R',
        Delta::Copied => 'C',
        Delta::Typechange => 'T',
        Delta::Untracked => '?',
        _ => 'M',
    }
}
impl Files {
    pub fn branch(&self) -> &str {
        self.head.branch()
    }
}
pub fn read(path: &Path) -> Result<Files> {
    let repo = open(path)?;
    idle(&repo)?;
    let head = head(&repo)?;
    let tree = repo.index()?.write_tree()?;
    let result = Files {
        path: path.to_owned(),
        head,
        tree,
        changes: changes(&repo)?,
    };
    validate(&repo, &result)?;
    Ok(result)
}
pub(crate) fn validate(repo: &Repository, f: &Files) -> Result<()> {
    idle(repo)?;
    ensure!(
        head(repo)? == f.head,
        gw_i18n::tr!(BranchOrHeadChangedRefreshTheFile)
    );
    ensure!(
        repo.index()?.write_tree()? == f.tree,
        gw_i18n::tr!(IndexChangedRefreshTheFileList)
    );
    Ok(())
}
pub fn dirty(repo: &Repository) -> Result<bool> {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(false)
        .exclude_submodules(false);
    Ok(!repo.statuses(Some(&mut opts))?.is_empty() || !submodule_changes(repo)?.is_empty())
}
fn submodule_changes(repo: &Repository) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for sub in repo.submodules()? {
        let name = sub.name().context(gw_i18n::tr!(SubmoduleNameIsNotUtf8))?;
        let status = repo.submodule_status(name, git2::SubmoduleIgnore::None)?;
        if status.intersects(
            git2::SubmoduleStatus::WD_MODIFIED
                | git2::SubmoduleStatus::WD_INDEX_MODIFIED
                | git2::SubmoduleStatus::WD_WD_MODIFIED
                | git2::SubmoduleStatus::WD_UNTRACKED
                | git2::SubmoduleStatus::WD_DELETED,
        ) {
            paths.push(sub.path().to_owned());
        }
    }
    Ok(paths)
}
