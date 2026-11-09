//! Capture raw worktree bytes and canonical index bytes without writing the index.
use super::*;
use crate::local::files::{
    Area, Location,
    patch::{Document, Extent, Line, MAX_SOURCE_BYTES, Metadata, Source},
};
use crate::{
    cancel::Cancel,
    local::{Head, open},
};
use anyhow::{Context, ensure};
use std::{
    fs::OpenOptions,
    io::Read,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::Path,
};

pub(super) fn indexed(
    repo: &git2::Repository,
    index: &git2::Index,
    path: &Path,
) -> Result<Version> {
    match index.get_path(path, 0) {
        None => Ok(Version::Absent),
        Some(entry) => object(repo, entry.id, entry.mode),
    }
}
fn object(repo: &git2::Repository, oid: git2::Oid, mode: u32) -> Result<Version> {
    ensure!(
        matches!(mode, 0o100644 | 0o100755),
        i18n::tr!(PartialUnsupported)
    );
    let blob = repo.find_blob(oid)?;
    ensure!(
        blob.size() <= MAX_SOURCE_BYTES && !blob.is_binary(),
        i18n::tr!(PartialUnsupported)
    );
    Ok(Version::Text {
        data: std::str::from_utf8(blob.content())?.into(),
        mode,
    })
}
pub(super) fn working(path: &Path) -> Result<Version> {
    let mut file = match OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
    {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Version::Absent),
        Err(error) => return Err(error.into()),
    };
    let meta = file.metadata()?;
    ensure!(
        meta.is_file() && meta.len() <= MAX_SOURCE_BYTES as u64,
        i18n::tr!(PartialUnsupported)
    );
    let mut data = Vec::new();
    file.by_ref()
        .take((MAX_SOURCE_BYTES + 1) as u64)
        .read_to_end(&mut data)?;
    ensure!(
        data.len() <= MAX_SOURCE_BYTES && !data.contains(&0),
        i18n::tr!(PartialUnsupported)
    );
    Ok(Version::Text {
        data: String::from_utf8(data)?.into(),
        mode: 0o100000 | (meta.permissions().mode() & 0o777),
    })
}
pub(crate) fn checked_worktree_path(root: &Path, path: &Path) -> Result<std::path::PathBuf> {
    let mut full = root.to_owned();
    let components = path.components().collect::<Vec<_>>();
    for (index, component) in components.iter().enumerate() {
        let std::path::Component::Normal(name) = component else {
            anyhow::bail!(i18n::tr!(PartialUnsupported));
        };
        full.push(name);
        if index + 1 < components.len() {
            match std::fs::symlink_metadata(&full) {
                Ok(metadata) => ensure!(
                    metadata.is_dir() && !metadata.is_symlink(),
                    i18n::tr!(PartialUnsupported)
                ),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
    }
    Ok(full)
}
fn matches_source(version: &Version, source: &Source) -> bool {
    match (version, source) {
        (Version::Absent, Source::Absent) => true,
        (Version::Text { data, .. }, Source::Text { text, .. }) => data.as_ref() == text,
        _ => false,
    }
}
pub(super) fn staged_worktree(
    repo: &git2::Repository,
    change: &Change,
    actual: &Version,
) -> Result<Version> {
    // repo.index() 共享仓库句柄中的索引缓存；使用独立句柄生成内存版本。
    let isolated = open(
        repo.workdir()
            .context(i18n::tr!(BareRepositoriesAreNotSupported))?,
    )?;
    let repo = &isolated;
    let mut index = repo.index()?;
    let path = change.location.name();
    match actual {
        Version::Absent => {
            if index.get_path(path, 0).is_some() {
                index.remove_path(path)?;
            }
        }
        Version::Text { .. } => {
            index.add_path(path)?;
        }
    }
    indexed(repo, &index, path)
}
fn checkout_version(repo: &git2::Repository, path: &Path, version: &Version) -> Result<Version> {
    if matches!(version, Version::Absent) {
        return Ok(Version::Absent);
    }
    let mut index = git2::Index::new()?;
    index.add(
        &repo
            .index()?
            .get_path(path, 0)
            .context(i18n::tr!(PartialChanged))?,
    )?;
    let directory = tempfile::tempdir()?;
    checkout_to(repo, &mut index, directory.path())?;
    working(&directory.path().join(path))
}

pub(crate) fn checkout_to(
    repo: &git2::Repository,
    index: &mut git2::Index,
    directory: &Path,
) -> Result<()> {
    let mut options = git2::build::CheckoutBuilder::new();
    options.target_dir(directory).force().update_index(false);
    repo.checkout_index(Some(index), Some(&mut options))?;
    Ok(())
}
impl Snapshot {
    pub fn capture(
        files: &Files,
        change: &Change,
        document: &Document,
        cancel: &Cancel,
    ) -> Result<Self> {
        cancel.check()?;
        ensure!(
            matches!(change.location, Location::Path(_)) && change.kind != 'U',
            i18n::tr!(PartialUnsupported)
        );
        let Document::Patch {
            files: patches,
            extent: Extent::Complete,
        } = document
        else {
            anyhow::bail!(i18n::tr!(PartialUnsupported));
        };
        ensure!(patches.len() == 1, i18n::tr!(PartialUnsupported));
        let repo = open(&files.path)?;
        super::super::validate(&repo, files)?;
        let path = change.location.name();
        require_supported_filters(&repo, path)?;
        let index = indexed(&repo, &repo.index()?, path)?;
        let layer = if change.area == Area::Staged {
            let before = match files.head {
                Head::Initial { .. } => Version::Absent,
                Head::Existing { oid, .. } => {
                    let tree = repo.find_commit(oid)?.tree()?;
                    match tree.get_path(path) {
                        Ok(entry) => object(&repo, entry.id(), entry.filemode() as u32)?,
                        Err(error) if error.code() == git2::ErrorCode::NotFound => Version::Absent,
                        Err(error) => return Err(error.into()),
                    }
                }
            };
            ensure!(
                matches_source(&before, &patches[0].before)
                    && matches_source(&index, &patches[0].after),
                i18n::tr!(PartialChanged)
            );
            Layer::Staged {
                before,
                after: index,
            }
        } else {
            let after = working(&checked_worktree_path(&files.path, path)?)?;
            ensure!(
                matches_source(&index, &patches[0].before)
                    && matches_source(&after, &patches[0].after),
                i18n::tr!(PartialChanged)
            );
            let indexed = staged_worktree(&repo, change, &after)?;
            let checkout = checkout_version(&repo, path, &index)?;
            Layer::Working {
                before: index,
                indexed,
                checkout,
                after,
            }
        };
        let mut hunks: Vec<Vec<LineId>> = Vec::new();
        let mut blocks = Vec::new();
        let mut block = Vec::new();
        for line in &patches[0].lines {
            let id = match line {
                Line::Added { new, .. } => Some(LineId::New(*new)),
                Line::Removed { old, .. } => Some(LineId::Old(*old)),
                Line::Context { .. }
                | Line::Metadata {
                    kind: Metadata::Hunk,
                    ..
                } => {
                    if !block.is_empty() {
                        blocks.push(std::mem::take(&mut block));
                    }
                    if matches!(
                        line,
                        Line::Metadata {
                            kind: Metadata::Hunk,
                            ..
                        }
                    ) {
                        hunks.push(Vec::new());
                    }
                    None
                }
                _ => None,
            };
            if let Some(id) = id {
                hunks
                    .last_mut()
                    .context(i18n::tr!(InvalidDiffStructure))?
                    .push(id);
                block.push(id);
            }
        }
        if !block.is_empty() {
            blocks.push(block);
        }
        hunks.retain(|hunk| !hunk.is_empty());
        ensure!(!hunks.is_empty(), i18n::tr!(PartialUnsupported));
        super::super::validate(&repo, files)?;
        cancel.check()?;
        let snapshot = Self {
            files: files.clone(),
            change: change.clone(),
            hunks,
            blocks,
            layer,
        };
        snapshot.check_document(&patches[0])?;
        Ok(snapshot)
    }
    pub(super) fn validate(&self, repo: &git2::Repository, cancel: &Cancel) -> Result<()> {
        cancel.check()?;
        super::super::validate(repo, &self.files)?;
        require_supported_filters(repo, self.change.location.name())?;
        if let Layer::Working {
            before,
            indexed,
            checkout,
            after,
        } = &self.layer
        {
            let actual = working(&checked_worktree_path(
                &self.files.path,
                self.change.location.name(),
            )?)?;
            ensure!(
                &actual == after && &staged_worktree(repo, &self.change, &actual)? == indexed,
                i18n::tr!(PartialChanged)
            );
            ensure!(
                &checkout_version(repo, self.change.location.name(), before)? == checkout,
                i18n::tr!(PartialChanged)
            );
        }
        Ok(())
    }
}

pub(crate) fn require_supported_filters(repo: &git2::Repository, path: &Path) -> Result<()> {
    super::super::index::require_supported_filters(repo, path)?;
    if let git2::AttrValue::String(name) = git2::AttrValue::from_bytes(repo.get_attr_bytes(
        path,
        "filter",
        git2::AttrCheckFlags::FILE_THEN_INDEX,
    )?) {
        match repo.config()?.get_string(&format!("filter.{name}.smudge")) {
            Ok(_) => anyhow::bail!(i18n::tf!(
                FilePathUsesExternalFilterNameSuch,
                name = name,
                path = path.display()
            )),
            Err(error) if error.code() == git2::ErrorCode::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}
