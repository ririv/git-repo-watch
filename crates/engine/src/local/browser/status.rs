//! File status retains independent index and worktree changes.
use anyhow::Result;
use git2::{Repository, Status};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Delta {
    Clean,
    Added,
    Modified,
    Deleted,
    Renamed,
    TypeChanged,
}
impl Delta {
    pub fn symbol(self) -> char {
        match self {
            Self::Clean => ' ',
            Self::Added => 'A',
            Self::Modified => 'M',
            Self::Deleted => 'D',
            Self::Renamed => 'R',
            Self::TypeChanged => 'T',
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileStatus {
    Tracked { index: Delta, worktree: Delta },
    Untracked,
    Conflict,
}
impl FileStatus {
    pub(super) fn read(repo: &Repository, path: &Path) -> Result<Self> {
        let status = repo.status_file(path)?;
        if status.contains(Status::CONFLICTED) {
            return Ok(Self::Conflict);
        }
        if status.contains(Status::WT_NEW) {
            return Ok(Self::Untracked);
        }
        let delta = |flags: [(Status, Delta); 5]| {
            flags
                .into_iter()
                .find_map(|(flag, delta)| status.contains(flag).then_some(delta))
                .unwrap_or(Delta::Clean)
        };
        Ok(Self::Tracked {
            index: delta([
                (Status::INDEX_NEW, Delta::Added),
                (Status::INDEX_DELETED, Delta::Deleted),
                (Status::INDEX_RENAMED, Delta::Renamed),
                (Status::INDEX_TYPECHANGE, Delta::TypeChanged),
                (Status::INDEX_MODIFIED, Delta::Modified),
            ]),
            worktree: delta([
                (Status::WT_DELETED, Delta::Deleted),
                (Status::WT_RENAMED, Delta::Renamed),
                (Status::WT_TYPECHANGE, Delta::TypeChanged),
                (Status::WT_MODIFIED, Delta::Modified),
                (Status::WT_UNREADABLE, Delta::Modified),
            ]),
        })
    }
    pub fn changed(self) -> bool {
        !matches!(
            self,
            Self::Tracked {
                index: Delta::Clean,
                worktree: Delta::Clean
            }
        )
    }
    pub fn label(self) -> String {
        match self {
            Self::Tracked { index, worktree } => format!("{}{}", index.symbol(), worktree.symbol()),
            Self::Untracked => "??".into(),
            Self::Conflict => "UU".into(),
        }
    }
}
