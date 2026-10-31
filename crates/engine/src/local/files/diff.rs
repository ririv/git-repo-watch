//! Bounded literal-path patches with line coordinates and independent source versions.
use super::{
    Area, Change, Files,
    patch::*,
    source::{FileInfo, Storage},
    validate,
};
use crate::{
    cancel::Cancel,
    local::{Head, open, utf8_path},
};
use anyhow::{Context, Result, ensure};
use git2::{DiffFile, DiffFormat, DiffLine, DiffLineType, DiffOptions};
use std::{
    fs::{self, OpenOptions},
    io::Read,
    num::NonZeroU32,
    os::unix::fs::OpenOptionsExt,
};

pub fn diff(snapshot: &Files, change: &Change) -> Result<String> {
    Ok(document(snapshot, change, &Cancel::default())?.text())
}

pub fn document(snapshot: &Files, change: &Change, cancel: &Cancel) -> Result<Document> {
    cancel.check()?;
    let repo = open(&snapshot.path)?;
    validate(&repo, snapshot)?;
    ensure!(
        snapshot.changes.contains(change),
        i18n::tr!(FileDoesNotBelongToThisSnapshot)
    );
    if change.area == Area::Untracked {
        let path = snapshot.path.join(change.location.name());
        let meta = fs::symlink_metadata(&path)?;
        if meta.file_type().is_symlink() {
            return Ok(Document::Notice(i18n::tf!(
                NewSymlink,
                change.location.name().display(),
                fs::read_link(path)?.display()
            )));
        }
        if !meta.is_file() {
            return Ok(Document::Notice(
                i18n::tr!(DirectoryOrSpecialFileTextDiffIs).into(),
            ));
        }
        if meta.len() > MAX_SOURCE_BYTES as u64 {
            let mut prefix = Vec::new();
            OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
                .open(&path)?
                .take(12)
                .read_to_end(&mut prefix)?;
            if super::media::Format::detect(&prefix).is_some() {
                ensure!(
                    meta.len() <= super::media::MAX_IMAGE_BYTES as u64,
                    i18n::tr!(ImageEncodedLimit)
                );
            } else {
                return Ok(Document::Notice(i18n::tf!(
                    LargeUntrackedFileBytesViewWithAn,
                    meta.len()
                )));
            }
        }
    }
    let mut options = DiffOptions::new();
    options
        .disable_pathspec_match(true)
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .show_untracked_content(true)
        .include_typechange(true);
    scope(&mut options, &change.location)?;
    let tree = match snapshot.head {
        Head::Initial { .. } => None,
        Head::Existing { oid, .. } => Some(repo.find_commit(oid)?.tree()?),
    };
    let mut diff = if change.kind == 'U' {
        repo.diff_tree_to_workdir(tree.as_ref(), Some(&mut options))?
    } else if change.area == Area::Staged {
        repo.diff_tree_to_index(tree.as_ref(), None, Some(&mut options))?
    } else {
        repo.diff_index_to_workdir(None, Some(&mut options))?
    };
    match_location(&mut diff, &change.location)?;
    let document = collect(
        &repo,
        &diff,
        if change.area == Area::Staged {
            Storage::Object
        } else {
            Storage::Worktree
        },
        &change.location,
        cancel,
    )?;
    validate(&repo, snapshot)?;
    cancel.check()?;
    Ok(document)
}

pub(crate) fn scope(options: &mut DiffOptions, location: &super::Location) -> Result<()> {
    for path in location.paths() {
        options.pathspec(utf8_path(path)?);
    }
    if let super::Location::Copy { before, .. } = location {
        options
            .include_unmodified(true)
            .pathspec(utf8_path(before)?);
    }
    Ok(())
}

pub(crate) fn match_location(diff: &mut git2::Diff<'_>, location: &super::Location) -> Result<()> {
    let mut options = git2::DiffFindOptions::new();
    options.for_untracked(true);
    match location {
        super::Location::Path(_) => return Ok(()),
        super::Location::Rename { .. } => {
            options.renames(true);
        }
        super::Location::Copy { .. } => {
            options
                .copies(true)
                .copies_from_unmodified(true)
                .remove_unmodified(true);
        }
    }
    diff.find_similar(Some(&mut options))?;
    Ok(())
}

pub(crate) fn collect(
    repo: &git2::Repository,
    diff: &git2::Diff<'_>,
    after_storage: Storage,
    location: &super::Location,
    cancel: &Cancel,
) -> Result<Document> {
    struct Pending {
        before: FileInfo,
        after: FileInfo,
        lines: Vec<Line>,
    }
    let mut files: Vec<Pending> = Vec::new();
    let mut bytes = 0;
    enum Collection {
        Complete,
        Truncated,
        Failed(anyhow::Error),
    }
    let mut collection = Collection::Complete;
    let printed = diff.print(DiffFormat::Patch, |delta, _, line| {
        if cancel.is_cancelled() {
            return false;
        }
        let selected = match location {
            super::Location::Path(path) => {
                delta.old_file().path() == Some(path.as_path())
                    || delta.new_file().path() == Some(path.as_path())
            }
            super::Location::Rename { before, after } | super::Location::Copy { before, after } => {
                delta.old_file().path() == Some(before.as_path())
                    && delta.new_file().path() == Some(after.as_path())
            }
        };
        if !selected {
            return true;
        }
        if bytes + line.content().len() + 1 > 128 * 1024 {
            collection = Collection::Truncated;
            return false;
        }
        bytes += line.content().len() + 1;
        let converted = (|| -> Result<()> {
            if line.origin_value() == DiffLineType::FileHeader {
                files.push(Pending {
                    before: info(delta.old_file())?,
                    after: info(delta.new_file())?,
                    lines: Vec::new(),
                });
            }
            let file = files.last_mut().context(i18n::tr!(InvalidDiffStructure))?;
            file.lines.push(convert(line, delta)?);
            Ok(())
        })();
        match converted {
            Ok(()) => true,
            Err(error) => {
                collection = Collection::Failed(error);
                false
            }
        }
    });
    cancel.check()?;
    let extent = match collection {
        Collection::Complete => {
            printed?;
            Extent::Complete
        }
        Collection::Truncated => Extent::Truncated,
        Collection::Failed(error) => return Err(error),
    };
    if files.is_empty() && extent == Extent::Complete {
        return Ok(Document::Notice(
            i18n::tr!(NoTextDiffAvailablePossiblyBinaryOr).into(),
        ));
    }
    let mut result = Vec::new();
    for file in files {
        cancel.check()?;
        result.push(FilePatch {
            before: file.before.read(repo, Storage::Object)?,
            after: file.after.read(repo, after_storage)?,
            lines: file.lines,
        });
    }
    cancel.check()?;
    Ok(Document::Patch {
        files: result,
        extent,
    })
}

fn info(file: DiffFile<'_>) -> Result<FileInfo> {
    Ok(FileInfo {
        path: file.path().context(i18n::tr!(ChangeHasNoPath))?.to_owned(),
        oid: file.id(),
        mode: file.mode(),
        binary: file.is_binary(),
        exists: file.exists(),
    })
}
fn binary_path(file: DiffFile<'_>) -> Result<BinaryPath> {
    // 与 libgit2 的二进制提示一致：零对象 ID 的一侧显示为 /dev/null。
    if file.id().is_zero() {
        return Ok(BinaryPath::Absent);
    }
    let path = file.path().context(i18n::tr!(ChangeHasNoPath))?;
    utf8_path(path)?;
    Ok(BinaryPath::File(path.to_owned()))
}

fn convert(line: DiffLine<'_>, delta: git2::DiffDelta<'_>) -> Result<Line> {
    let text =
        String::from_utf8(line.content().to_vec()).context(i18n::tr!(DiffIsNotUtf8ViewWith))?;
    let number = |value: Option<u32>| {
        value
            .and_then(NonZeroU32::new)
            .context(i18n::tr!(InvalidDiffStructure))
    };
    Ok(match line.origin_value() {
        DiffLineType::Context => Line::Context {
            old: number(line.old_lineno())?,
            new: number(line.new_lineno())?,
            text,
        },
        DiffLineType::Addition => Line::Added {
            new: number(line.new_lineno())?,
            text,
        },
        DiffLineType::Deletion => Line::Removed {
            old: number(line.old_lineno())?,
            text,
        },
        kind => Line::Metadata {
            kind: match kind {
                DiffLineType::FileHeader => Metadata::File,
                DiffLineType::HunkHeader => Metadata::Hunk,
                DiffLineType::Binary => Metadata::Binary {
                    before: binary_path(delta.old_file())?,
                    after: binary_path(delta.new_file())?,
                },
                _ => Metadata::Notice,
            },
            text,
        },
    })
}
