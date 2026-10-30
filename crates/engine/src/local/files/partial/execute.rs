//! Revalidate under Git's index lock before installing a selected text projection.
use super::*;
use crate::{
    cancel::Cancel,
    local::{index_lock::IndexLock, open},
};
use anyhow::Context;
use std::{fs, io::Write, os::unix::fs::PermissionsExt};

pub fn execute(plan: &Plan, cancel: &Cancel) -> Result<Files> {
    let snapshot = &plan.snapshot;
    let repo = open(&snapshot.files.path)?;
    let index_path = repo
        .index()?
        .path()
        .context(i18n::tr!(PartialChanged))?
        .to_owned();
    let mut lock = IndexLock::new(&index_path)?;
    snapshot.validate(&repo, cancel)?;
    let path = snapshot.change.location.name();
    if plan.action == Action::Discard {
        let full = super::capture::checked_worktree_path(&snapshot.files.path, path)?;
        match &plan.next {
            Version::Absent => {
                snapshot.validate(&repo, cancel)?;
                fs::remove_file(&full)?;
            }
            Version::Text { data, mode } => {
                fs::create_dir_all(full.parent().context(i18n::tr!(PartialChanged))?)?;
                let mut pending = tempfile::NamedTempFile::new_in(
                    full.parent().context(i18n::tr!(PartialChanged))?,
                )?;
                pending
                    .as_file()
                    .set_permissions(fs::Permissions::from_mode(mode & 0o777))?;
                pending.write_all(data.as_bytes())?;
                pending.as_file().sync_all()?;
                snapshot.validate(&repo, cancel)?;
                pending.persist(&full).map_err(|error| error.error)?;
            }
        }
    } else {
        let directory = tempfile::tempdir_in(repo.path())?;
        let pending = directory.path().join("partial-index");
        match fs::copy(&index_path, &pending) {
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        let mut index = git2::Index::open(&pending)?;
        match &plan.next {
            Version::Absent => {
                index.remove_path(path)?;
            }
            Version::Text { data, mode } => {
                let mut entry = index.get_path(path, 0).unwrap_or_else(|| git2::IndexEntry {
                    ctime: git2::IndexTime::new(0, 0),
                    mtime: git2::IndexTime::new(0, 0),
                    dev: 0,
                    ino: 0,
                    mode: *mode,
                    uid: 0,
                    gid: 0,
                    file_size: 0,
                    id: git2::Oid::ZERO_SHA1,
                    flags: 0,
                    flags_extended: 0,
                    path: path.as_os_str().as_encoded_bytes().to_vec(),
                });
                entry.id = repo.blob(data.as_bytes())?;
                entry.mode = *mode;
                entry.file_size = data.len() as u32;
                entry.mtime = git2::IndexTime::new(0, 0);
                entry.flags_extended &= !0x2000;
                index.add(&entry)?;
            }
        }
        index.write()?;
        lock.prepare(&fs::read(&pending)?)?;
        snapshot.validate(&repo, cancel)?;
        lock.install()?;
    }
    super::super::read(&snapshot.files.path)
}
