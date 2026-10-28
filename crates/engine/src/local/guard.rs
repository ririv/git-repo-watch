//! Immutable review guards for HEAD, index and visible working-tree changes.
use super::{idle, open};
use crate::cancel::Cancel;
use anyhow::{Context, Result, ensure};
use git2::{ErrorCode, Oid, Repository, Status, StatusOptions};
use sha2::{Digest, Sha256};
use std::{fs, io::Read, path::Path};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Position {
    Attached { name: String, oid: Oid },
    Detached(Oid),
    Unborn(String),
}
impl Position {
    pub fn read(repo: &Repository) -> Result<Self> {
        match repo.head() {
            Ok(head) if repo.head_detached()? => Ok(Self::Detached(head.peel_to_commit()?.id())),
            Ok(head) => Ok(Self::Attached {
                name: head.name().context(i18n::tr!(BranchNameIsNotUtf8))?.into(),
                oid: head.peel_to_commit()?.id(),
            }),
            Err(error) if error.code() == ErrorCode::UnbornBranch => Ok(Self::Unborn(
                repo.find_reference("HEAD")?
                    .symbolic_target()?
                    .context(i18n::tr!(HeadHasNoSymbolicBranch))?
                    .into(),
            )),
            Err(error) => Err(error.into()),
        }
    }
    pub fn oid(&self) -> Result<Oid> {
        match self {
            Self::Attached { oid, .. } | Self::Detached(oid) => Ok(*oid),
            Self::Unborn(_) => anyhow::bail!(i18n::tr!(MutationRequiresCommit)),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Guard {
    pub position: Position,
    fingerprint: [u8; 32],
    operation: git2::RepositoryState,
}
impl Guard {
    pub fn capture(path: &Path, cancel: &Cancel) -> Result<Self> {
        cancel.check()?;
        let repo = open(path)?;
        idle(&repo)?;
        Self::capture_current(path, cancel)
    }
    pub(crate) fn capture_current(path: &Path, cancel: &Cancel) -> Result<Self> {
        cancel.check()?;
        let repo = open(path)?;
        let position = Position::read(&repo)?;
        let mut hash = Sha256::new();
        // 比较索引内容，不比较 stat 缓存，外部 git status 的缓存刷新不使预览失效。
        for entry in repo.index()?.iter() {
            cancel.check()?;
            hash.update((entry.path.len() as u64).to_le_bytes());
            hash.update(entry.path);
            hash.update(entry.id.as_bytes());
            hash.update(entry.mode.to_le_bytes());
            hash.update(entry.flags.to_le_bytes());
            hash.update(entry.flags_extended.to_le_bytes());
        }
        let mut options = StatusOptions::new();
        options
            .include_untracked(true)
            .recurse_untracked_dirs(true)
            .include_ignored(false);
        for entry in repo.statuses(Some(&mut options))?.iter() {
            cancel.check()?;
            hash.update(entry.status().bits().to_le_bytes());
            let name = entry.path().context(i18n::tr!(GitPathIsNotUtf8And))?;
            hash.update((name.len() as u64).to_le_bytes());
            hash.update(name.as_bytes());
            let full = path.join(name);
            match fs::symlink_metadata(&full) {
                Ok(metadata) if metadata.is_symlink() => {
                    hash.update(b"link");
                    hash.update(fs::read_link(&full)?.as_os_str().as_encoded_bytes());
                }
                Ok(metadata) if metadata.is_file() => {
                    use std::os::unix::fs::PermissionsExt;
                    hash.update(b"file");
                    hash.update(metadata.permissions().mode().to_le_bytes());
                    let mut file = fs::File::open(&full)?;
                    let mut buffer = [0; 65536];
                    loop {
                        cancel.check()?;
                        let size = file.read(&mut buffer)?;
                        if size == 0 {
                            break;
                        }
                        hash.update(&buffer[..size]);
                    }
                }
                // 子模块内容不由父仓库 stash 保存，不能把目录状态当成已保存。
                Ok(metadata) if metadata.is_dir() => {
                    anyhow::bail!(i18n::tr!(MutationDirtySubmodule))
                }
                Ok(_) => anyhow::bail!(i18n::tr!(MutationNonRegularFile)),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    hash.update(b"deleted")
                }
                Err(error) => return Err(error.into()),
            }
        }
        Ok(Self {
            position,
            fingerprint: hash.finalize().into(),
            operation: repo.state(),
        })
    }
    pub(crate) fn validate_current(&self, path: &Path, cancel: &Cancel) -> Result<()> {
        ensure!(
            Self::capture_current(path, cancel)? == *self,
            i18n::tr!(MutationChanged)
        );
        Ok(())
    }
    pub fn validate(&self, path: &Path, cancel: &Cancel) -> Result<()> {
        ensure!(
            Self::capture(path, cancel)? == *self,
            i18n::tr!(MutationChanged)
        );
        Ok(())
    }
}
pub fn clean_tracked(repo: &Repository) -> Result<()> {
    idle(repo)?;
    let mut options = StatusOptions::new();
    options.include_untracked(false).include_ignored(false);
    ensure!(
        repo.statuses(Some(&mut options))?
            .iter()
            .all(|entry| entry.status() == Status::CURRENT),
        i18n::tr!(MutationCleanRequired)
    );
    Ok(())
}
