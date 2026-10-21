//! Bounded, literal-path diffs with untracked and binary handling.
use super::{Area, Change, Files, validate};
use crate::local::{Head, open, utf8_path};
use anyhow::{Context, Result, ensure};
use git2::{DiffFormat, DiffOptions};
use std::fs;
pub fn diff(snapshot: &Files, change: &Change) -> Result<String> {
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
            return Ok(i18n::tf!(
                NewSymlink,
                change.location.name(),
                fs::read_link(path)?
            ));
        }
        if !meta.is_file() {
            return Ok(i18n::tr!(DirectoryOrSpecialFileTextDiffIs).into());
        }
        if meta.len() > 1024 * 1024 {
            return Ok(i18n::tf!(LargeUntrackedFileBytesViewWithAn, meta.len()));
        }
    }
    let mut options = DiffOptions::new();
    options
        .disable_pathspec_match(true)
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .show_untracked_content(true)
        .include_typechange(true);
    for path in change.location.paths() {
        options.pathspec(utf8_path(path)?);
    }
    let tree = match snapshot.head {
        Head::Initial { .. } => None,
        Head::Existing { oid, .. } => Some(repo.find_commit(oid)?.tree()?),
    };
    let diff = if change.area == Area::Staged {
        repo.diff_tree_to_index(tree.as_ref(), None, Some(&mut options))?
    } else {
        repo.diff_index_to_workdir(None, Some(&mut options))?
    };
    let mut out = Vec::new();
    let mut truncated = false;
    diff.print(DiffFormat::Patch, |_, _, line| {
        if truncated {
            return true;
        }
        if out.len() + line.content().len() + 1 > 128 * 1024 {
            truncated = true;
            return true;
        }
        if matches!(line.origin(), '+' | '-' | ' ') {
            out.push(line.origin() as u8)
        }
        out.extend_from_slice(line.content());
        true
    })?;
    let mut text = String::from_utf8(out).context(i18n::tr!(DiffIsNotUtf8ViewWith))?;
    if truncated {
        text.push_str(i18n::tr!(DiffExceeds128KibOnlyTheBeginning))
    }
    if text.is_empty() {
        text = i18n::tr!(NoTextDiffAvailablePossiblyBinaryOr).into()
    }
    Ok(text)
}
