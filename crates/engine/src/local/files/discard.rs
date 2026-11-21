//! Review and discard selected worktree entries against a frozen index and file identity.
use super::{Area, Change, Files, Location, partial::capture};
use crate::platform::files as platform_files;
use crate::{
    cancel::Cancel,
    local::{index_lock::IndexLock, open},
};
use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    io::{ErrorKind, Read},
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

#[cfg(unix)]
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

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

impl Identity {
    fn at(path: &Path, metadata: &fs::Metadata) -> std::io::Result<Self> {
        #[cfg(unix)]
        {
            let _ = path;
            Ok(Self::from(metadata))
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            use windows_sys::Win32::Storage::FileSystem::{
                FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
            };
            let _ = metadata;
            let handle = fs::OpenOptions::new()
                .access_mode(0)
                .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT)
                .open(path)?;
            Self::of(&handle)
        }
    }

    fn of(file: &fs::File) -> std::io::Result<Self> {
        #[cfg(unix)]
        return Ok(Self::from(&file.metadata()?));
        #[cfg(windows)]
        {
            use std::os::windows::io::AsRawHandle;
            use windows_sys::Win32::Storage::FileSystem::{
                BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle,
            };
            let mut info = BY_HANDLE_FILE_INFORMATION::default();
            if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &mut info) } == 0 {
                return Err(std::io::Error::last_os_error());
            }
            let time = |value: windows_sys::Win32::Foundation::FILETIME| {
                (value.dwHighDateTime as i64, value.dwLowDateTime as i64)
            };
            Ok(Self {
                device: info.dwVolumeSerialNumber.into(),
                inode: (u64::from(info.nFileIndexHigh) << 32) | u64::from(info.nFileIndexLow),
                mode: info.dwFileAttributes,
                size: (u64::from(info.nFileSizeHigh) << 32) | u64::from(info.nFileSizeLow),
                modified: time(info.ftLastWriteTime),
                changed: time(info.ftCreationTime),
            })
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
        let identity = Identity::at(path, &meta)?;
        let result = if platform_files::is_file_link(&meta) {
            Self::Symlink(identity, fs::read_link(path)?)
        } else {
            ensure!(
                meta.is_file() && !platform_files::is_link(&meta),
                i18n::tr!(DiscardFileUnsupported)
            );
            let mut file = platform_files::regular_file(path)?;
            ensure!(Identity::of(&file)? == identity, i18n::tr!(PartialChanged));
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
            Identity::at(path, &fs::symlink_metadata(path)?)? == identity,
            i18n::tr!(PartialChanged)
        );
        Ok(result)
    }
}

#[derive(Debug)]
struct Entry {
    relative: PathBuf,
    action: Action,
    expected: Worktree,
}

impl Entry {
    fn capture(
        repo: &git2::Repository,
        index: &git2::Index,
        root: &Path,
        change: &Change,
        cancel: &Cancel,
    ) -> Result<Self> {
        let action = Action::for_change(change).context(i18n::tr!(DiscardFileUnsupported))?;
        let relative = change.location.name().to_owned();
        let full = capture::checked_worktree_path(root, &relative)?;
        let entry = index.get_path(&relative, 0);
        match action {
            Action::Restore => {
                restore_entry(entry)?;
                capture::require_supported_filters(repo, &relative)?;
            }
            Action::RemoveUntracked => ensure!(entry.is_none(), i18n::tr!(PartialChanged)),
        }
        let expected = Worktree::read(&full, cancel)?;
        ensure!(
            (change.kind == 'D') == matches!(expected, Worktree::Missing),
            i18n::tr!(PartialChanged)
        );
        Ok(Self {
            relative,
            action,
            expected,
        })
    }

    fn validate(&self, root: &Path, cancel: &Cancel) -> Result<PathBuf> {
        let full = capture::checked_worktree_path(root, &self.relative)?;
        ensure!(
            Worktree::read(&full, cancel)? == self.expected,
            i18n::tr!(PartialChanged)
        );
        cancel.check()?;
        Ok(full)
    }
}

enum Replacement {
    File(PathBuf),
    Symlink { source: PathBuf, target: PathBuf },
}

impl Replacement {
    fn read(source: PathBuf) -> Result<Self> {
        let meta = fs::symlink_metadata(&source)?;
        if platform_files::is_file_link(&meta) {
            let target = fs::read_link(&source)?;
            return Ok(Self::Symlink { source, target });
        }
        ensure!(
            meta.is_file() && !platform_files::is_link(&meta),
            i18n::tr!(DiscardFileUnsupported)
        );
        fs::File::open(&source)?.sync_all()?;
        Ok(Self::File(source))
    }

    fn install(self, full: &Path, missing: bool) -> Result<()> {
        if missing {
            // 即使外部新建了同名文件，安装时也不会覆盖它。
            match self {
                Self::File(source) => fs::hard_link(source, full)?,
                Self::Symlink { target, .. } => platform_files::symlink_file(&target, full)?,
            }
        } else {
            let source = match self {
                Self::File(source) | Self::Symlink { source, .. } => source,
            };
            platform_files::replace(&source, full)?;
        }
        Ok(())
    }
}

enum Prepared<'a> {
    Restore(&'a Entry, Replacement),
    Remove(&'a Entry),
}

impl<'a> Prepared<'a> {
    fn entry(&self) -> &'a Entry {
        match self {
            Self::Restore(entry, _) | Self::Remove(entry) => entry,
        }
    }

    fn apply(self, root: &Path, cancel: &Cancel) -> Result<()> {
        let entry = self.entry();
        match self {
            Self::Restore(_, replacement) => {
                capture::require_supported_filters(&open(root)?, &entry.relative)?;
                let missing = entry.expected == Worktree::Missing;
                let mut full = entry.validate(root, cancel)?;
                if missing {
                    fs::create_dir_all(full.parent().context(i18n::tr!(PartialChanged))?)?;
                    full = capture::checked_worktree_path(root, &entry.relative)?;
                    cancel.check()?;
                }
                replacement.install(&full, missing)
            }
            Self::Remove(_) => Ok(fs::remove_file(entry.validate(root, cancel)?)?),
        }
    }
}

#[derive(Debug)]
pub struct Plan {
    files: Files,
    entries: Vec<Entry>,
}

impl Plan {
    pub fn prepare(files: Files, changes: &[Change], cancel: &Cancel) -> Result<Self> {
        ensure!(!changes.is_empty(), i18n::tr!(DiscardFileSelect));
        let repo = open(&files.path)?;
        super::validate(&repo, &files)?;
        let index = repo.index()?;
        let mut paths = BTreeSet::new();
        let mut entries = Vec::with_capacity(changes.len());
        for change in changes {
            let path = change.location.name();
            ensure!(
                paths.insert(path) && files.changes.contains(change),
                i18n::tr!(PartialChanged)
            );
            entries.push(
                Entry::capture(&repo, &index, &files.path, change, cancel)
                    .with_context(|| path.display().to_string())?,
            );
        }
        super::validate(&open(&files.path)?, &files)?;
        Ok(Self { files, entries })
    }

    pub fn repository(&self) -> &Path {
        &self.files.path
    }

    pub fn entries(&self) -> impl ExactSizeIterator<Item = (&Path, Action)> {
        self.entries
            .iter()
            .map(|entry| (entry.relative.as_path(), entry.action))
    }

    pub fn execute(&self, cancel: &Cancel) -> Result<Files> {
        let root = self.repository();
        let repo = open(root)?;
        let index = repo.index()?;
        let _lock = IndexLock::new(index.path().context(i18n::tr!(PartialChanged))?)?;
        super::validate(&repo, &self.files)?;

        let mut checkout = git2::Index::new()?;
        let mut restores = Vec::new();
        let mut prepared = Vec::with_capacity(self.entries.len());
        let mut removals = Vec::new();
        for entry in &self.entries {
            match entry.action {
                Action::Restore => {
                    capture::require_supported_filters(&repo, &entry.relative)
                        .with_context(|| entry.relative.display().to_string())?;
                    checkout.add(&restore_entry(index.get_path(&entry.relative, 0))?)?;
                    restores.push(entry);
                }
                Action::RemoveUntracked => removals.push(Prepared::Remove(entry)),
            }
        }
        let pending = if restores.is_empty() {
            None
        } else {
            let directory = tempfile::tempdir_in(root)?;
            capture::checkout_to(&repo, &mut checkout, directory.path())?;
            for entry in restores {
                let source = directory.path().join(&entry.relative);
                let replacement = Replacement::read(source)
                    .with_context(|| entry.relative.display().to_string())?;
                prepared.push(Prepared::Restore(entry, replacement));
            }
            Some(directory)
        };

        let validation_repo = open(root)?;
        super::validate(&validation_repo, &self.files)?;
        for entry in &self.entries {
            entry
                .validate(root, cancel)
                .with_context(|| entry.relative.display().to_string())?;
            if entry.action == Action::Restore {
                capture::require_supported_filters(&validation_repo, &entry.relative)
                    .with_context(|| entry.relative.display().to_string())?;
            }
        }
        prepared.extend(removals);
        let total = prepared.len();
        for (count, entry) in prepared.into_iter().enumerate() {
            let path = entry.entry().relative.as_path();
            entry.apply(root, cancel).with_context(|| {
                i18n::tf!(
                    DiscardFileInterrupted,
                    count = count,
                    total = total,
                    path = path.display()
                )
            })?;
        }
        drop(pending);
        super::read(root)
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
