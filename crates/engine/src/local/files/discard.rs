//! Review and discard one worktree entry against a frozen index and file identity.
use super::{Area, Change, Files, Location, partial::capture};
use crate::{
    cancel::Cancel,
    local::{index_lock::IndexLock, open},
};
use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::{ErrorKind, Read},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Restore,
    RemoveUntracked,
}

impl Action {
    pub fn for_change(change: &Change) -> Option<Self> {
        match (&change.location, change.area, change.kind) {
            (Location::Path(_), Area::Unstaged, 'M' | 'D' | 'T') => Some(Self::Restore),
            (Location::Path(_), Area::Untracked, '?') => Some(Self::RemoveUntracked),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Identity {
    device: u64,
    inode: u64,
    mode: u32,
    size: u64,
    modified: (i64, i64),
    changed: (i64, i64),
}

impl From<&fs::Metadata> for Identity {
    fn from(meta: &fs::Metadata) -> Self {
        Self {
            device: meta.dev(),
            inode: meta.ino(),
            mode: meta.mode(),
            size: meta.size(),
            modified: (meta.mtime(), meta.mtime_nsec()),
            changed: (meta.ctime(), meta.ctime_nsec()),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Worktree {
    Missing,
    File(Identity, [u8; 32]),
    Symlink(Identity, PathBuf),
}

impl Worktree {
    fn read(path: &Path, cancel: &Cancel) -> Result<Self> {
        cancel.check()?;
        let meta = match fs::symlink_metadata(path) {
            Ok(meta) => meta,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Self::Missing),
            Err(error) => return Err(error.into()),
        };
        let identity = Identity::from(&meta);
        let result = if meta.is_symlink() {
            Self::Symlink(identity, fs::read_link(path)?)
        } else {
            ensure!(meta.is_file(), i18n::tr!(DiscardFileUnsupported));
            let mut file = OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
                .open(path)?;
            ensure!(
                Identity::from(&file.metadata()?) == identity,
                i18n::tr!(PartialChanged)
            );
            let mut hash = Sha256::new();
            let mut buffer = [0; 32 * 1024];
            loop {
                cancel.check()?;
                let count = file.read(&mut buffer)?;
                if count == 0 {
                    break;
                }
                hash.update(&buffer[..count]);
            }
            Self::File(identity, hash.finalize().into())
        };
        ensure!(
            Identity::from(&fs::symlink_metadata(path)?) == identity,
            i18n::tr!(PartialChanged)
        );
        Ok(result)
    }
}

#[derive(Debug)]
pub struct Plan {
    files: Files,
    relative: PathBuf,
    action: Action,
    expected: Worktree,
}

impl Plan {
    pub fn prepare(files: Files, change: &Change, cancel: &Cancel) -> Result<Self> {
        let action = Action::for_change(change).context(i18n::tr!(DiscardFileUnsupported))?;
        ensure!(files.changes.contains(change), i18n::tr!(PartialChanged));
        let repo = open(&files.path)?;
        super::validate(&repo, &files)?;
        let relative = change.location.name().to_owned();
        let full = capture::checked_worktree_path(&files.path, &relative)?;
        let index = repo.index()?;
        let entry = index.get_path(&relative, 0);
        match action {
            Action::Restore => {
                restore_entry(entry)?;
                capture::require_supported_filters(&repo, &relative)?;
            }
            Action::RemoveUntracked => ensure!(entry.is_none(), i18n::tr!(PartialChanged)),
        }
        let expected = Worktree::read(&full, cancel)?;
        ensure!(
            (change.kind == 'D') == matches!(expected, Worktree::Missing),
            i18n::tr!(PartialChanged)
        );
        super::validate(&open(&files.path)?, &files)?;
        Ok(Self {
            files,
            relative,
            action,
            expected,
        })
    }

    pub fn repository(&self) -> &Path {
        &self.files.path
    }

    pub fn file(&self) -> &Path {
        &self.relative
    }

    pub fn action(&self) -> Action {
        self.action
    }

    fn validate_worktree(&self, cancel: &Cancel) -> Result<PathBuf> {
        let full = capture::checked_worktree_path(self.repository(), self.file())?;
        ensure!(
            Worktree::read(&full, cancel)? == self.expected,
            i18n::tr!(PartialChanged)
        );
        cancel.check()?;
        Ok(full)
    }

    pub fn execute(&self, cancel: &Cancel) -> Result<Files> {
        let repo = open(self.repository())?;
        let index = repo.index()?;
        let _lock = IndexLock::new(index.path().context(i18n::tr!(PartialChanged))?)?;
        super::validate(&repo, &self.files)?;
        match self.action {
            Action::Restore => {
                capture::require_supported_filters(&repo, self.file())?;
                let mut checkout = git2::Index::new()?;
                checkout.add(&restore_entry(index.get_path(self.file(), 0))?)?;
                let pending = tempfile::tempdir_in(self.repository())?;
                capture::checkout_to(&repo, &mut checkout, pending.path())?;
                let source = pending.path().join(self.file());
                let meta = fs::symlink_metadata(&source)?;
                ensure!(
                    meta.is_file() || meta.is_symlink(),
                    i18n::tr!(DiscardFileUnsupported)
                );
                if meta.is_file() {
                    fs::File::open(&source)?.sync_all()?;
                }
                super::validate(&repo, &self.files)?;
                capture::require_supported_filters(&open(self.repository())?, self.file())?;
                let full = self.validate_worktree(cancel)?;
                if self.expected == Worktree::Missing {
                    fs::create_dir_all(full.parent().context(i18n::tr!(PartialChanged))?)?;
                    let full = capture::checked_worktree_path(self.repository(), self.file())?;
                    cancel.check()?;
                    // 新创建的同名文件必须保留；这两种安装方式都拒绝覆盖。
                    if meta.is_symlink() {
                        std::os::unix::fs::symlink(fs::read_link(source)?, full)?;
                    } else {
                        fs::hard_link(source, full)?;
                    }
                } else {
                    fs::rename(source, full)?;
                }
            }
            Action::RemoveUntracked => {
                super::validate(&repo, &self.files)?;
                fs::remove_file(self.validate_worktree(cancel)?)?;
            }
        }
        super::read(self.repository())
    }
}

fn restore_entry(entry: Option<git2::IndexEntry>) -> Result<git2::IndexEntry> {
    let entry = entry.context(i18n::tr!(DiscardFileUnsupported))?;
    // 意向添加和稀疏检出条目没有普通的工作区恢复语义。
    ensure!(
        matches!(entry.mode, 0o100644 | 0o100755 | 0o120000) && entry.flags_extended & 0x6000 == 0,
        i18n::tr!(DiscardFileUnsupported)
    );
    Ok(entry)
}
