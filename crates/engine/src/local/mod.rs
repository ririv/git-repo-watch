//! Local repository capabilities implemented exclusively through libgit2.
pub mod branches;
pub mod browser;
pub mod files;
pub mod guard;
pub mod history;
pub(crate) mod index_lock;
pub mod overview;
pub mod review;
pub mod stash;
use anyhow::{Context, Result, ensure};
use git2::{Repository, RepositoryOpenFlags, RepositoryState};
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Locate the containing worktree without modifying its files, index or references.
pub fn containing_repository(
    directory: &Path,
    cancel: &crate::cancel::Cancel,
) -> Result<Option<PathBuf>> {
    cancel.check()?;
    match Repository::discover(directory) {
        Ok(repo) => {
            let root = repo
                .workdir()
                .context(i18n::tr!(BareRepositoriesAreNotSupported))?;
            Ok(Some(fs::canonicalize(root)?))
        }
        Err(error) if error.code() == git2::ErrorCode::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

pub fn open(path: &Path) -> Result<Repository> {
    let repo = Repository::open_ext(
        path,
        RepositoryOpenFlags::NO_SEARCH,
        std::iter::empty::<&Path>(),
    )?;
    let root = repo
        .workdir()
        .context(i18n::tr!(BareRepositoriesAreNotSupported))?;
    ensure!(
        fs::canonicalize(root)? == fs::canonicalize(path)?,
        i18n::tf!(ConfiguredPathIsNotTheRepositoryRoot, root.display())
    );
    Ok(repo)
}
pub fn idle(repo: &Repository) -> Result<()> {
    ensure!(
        repo.state() == RepositoryState::Clean,
        i18n::tf!(RepositoryHasAnUnfinishedGitOperation, repo.state())
    );
    ensure!(
        !repo.index()?.has_conflicts(),
        i18n::tr!(RepositoryHasUnresolvedConflicts)
    );
    Ok(())
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Head {
    Initial { branch: String },
    Existing { branch: String, oid: git2::Oid },
}
impl Head {
    pub fn branch(&self) -> &str {
        match self {
            Self::Initial { branch } | Self::Existing { branch, .. } => branch,
        }
    }
}
pub fn head(repo: &Repository) -> Result<Head> {
    ensure!(
        !repo.head_detached()?,
        i18n::tr!(HeadIsDetachedSwitchToALocal)
    );
    match repo.head() {
        Ok(h) => Ok(Head::Existing {
            branch: h
                .shorthand()
                .context(i18n::tr!(BranchNameIsNotUtf8))?
                .to_owned(),
            oid: h.target().context(i18n::tr!(BranchHasNoCommit))?,
        }),
        Err(e) if e.code() == git2::ErrorCode::UnbornBranch => {
            let h = repo.find_reference("HEAD")?;
            let target = h
                .symbolic_target()?
                .context(i18n::tr!(HeadHasNoSymbolicBranch))?;
            Ok(Head::Initial {
                branch: target
                    .strip_prefix("refs/heads/")
                    .context(i18n::tr!(HeadIsNotAttachedToALocal))?
                    .to_owned(),
            })
        }
        Err(e) => Err(e.into()),
    }
}
pub fn common_dir(repo: &Repository) -> Result<PathBuf> {
    Ok(fs::canonicalize(repo.commondir())?)
}
pub fn utf8_path(path: &Path) -> Result<&str> {
    path.to_str().context(i18n::tr!(GitPathIsNotUtf8And))
}
