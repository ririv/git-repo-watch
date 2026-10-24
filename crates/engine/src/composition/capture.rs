//! Capture staged and working-tree layers without writing the user's index.
use super::*;
use crate::{
    cancel::Cancel,
    local::{self, files},
};
use anyhow::{Context, Result, ensure};
use git2::{Delta, DiffOptions, Oid, Patch, Repository};
use std::{fs, path::Path};
const MAX_CONTEXT: usize = 128 * 1024;

pub(super) fn trees(path: &Path, cancel: &Cancel) -> Result<(Head, Oid, Oid)> {
    cancel.check()?;
    let files = files::read(path)?;
    ensure!(files.changes.len() <= 512, i18n::tr!(ComposeTooLarge));
    let repo = local::open(path)?;
    let mut index = repo.index()?;
    let original = index.write_tree()?;
    for change in files
        .changes
        .iter()
        .filter(|change| change.area != files::Area::Staged)
    {
        cancel.check()?;
        let path = change.location.name();
        let full = files.path.join(path);
        match fs::symlink_metadata(&full) {
            Ok(metadata) if metadata.is_dir() => {
                let sub = Repository::open(&full)?;
                let oid = sub.head()?.peel_to_commit()?.id();
                let mut entry = index
                    .get_path(path, 0)
                    .context(i18n::tr!(ComposeSubmodule))?;
                entry.id = oid;
                index.add(&entry)?;
            }
            Ok(metadata) if metadata.is_file() || metadata.is_symlink() => {
                ensure!(
                    metadata.len() <= 16 * 1024 * 1024,
                    "{}: {}",
                    i18n::tr!(ComposeFileTooLarge),
                    path.display()
                );
                files::index::require_supported_filters(&repo, path)?;
                index.add_path(path)?;
            }
            Ok(_) => anyhow::bail!(i18n::tr!(MutationNonRegularFile)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                index.remove_path(path)?;
            }
            Err(error) => return Err(error.into()),
        }
    }
    let working = index.write_tree()?;
    Ok((files.head, original, working))
}
pub fn capture(path: &Path, cancel: &Cancel) -> Result<Snapshot> {
    let (head, index, worktree) = trees(path, cancel)?;
    let repo = local::open(path)?;
    let base = projection::head_tree(&repo, &head)?;
    let mut units = Vec::new();
    collect(&repo, base, index, Layer::Staged, &mut units, cancel)?;
    let staged = units.clone();
    collect(&repo, index, worktree, Layer::Worktree, &mut units, cancel)?;
    for unit in units
        .iter_mut()
        .filter(|unit| unit.layer == Layer::Worktree)
    {
        unit.dependencies = staged
            .iter()
            .filter(|previous| intersects(previous, unit))
            .map(|unit| unit.id.clone())
            .collect();
    }
    ensure!(!units.is_empty(), i18n::tr!(ComposeNoChanges));
    ensure!(
        units.len() <= 512
            && units.iter().map(|unit| unit.preview.len()).sum::<usize>() <= MAX_CONTEXT,
        i18n::tr!(ComposeTooLarge)
    );
    let snapshot = Snapshot {
        path: path.to_owned(),
        head,
        index,
        worktree,
        units,
    };
    validate(&snapshot, cancel)?;
    Ok(snapshot)
}
fn intersects(a: &Unit, b: &Unit) -> bool {
    let paths = |unit: &Unit| match &unit.change {
        Change::Hunk { .. } => vec![unit.path.clone()],
        Change::Whole { before, after, .. } => vec![before.clone(), after.clone()],
    };
    paths(a).iter().any(|path| paths(b).contains(path))
}
pub(super) fn validate(snapshot: &Snapshot, cancel: &Cancel) -> Result<()> {
    ensure!(
        trees(&snapshot.path, cancel)?
            == (snapshot.head.clone(), snapshot.index, snapshot.worktree),
        i18n::tr!(ComposeChanged)
    );
    Ok(())
}
fn collect(
    repo: &Repository,
    before: Oid,
    after: Oid,
    layer: Layer,
    units: &mut Vec<Unit>,
    cancel: &Cancel,
) -> Result<()> {
    let before_tree = repo.find_tree(before)?;
    let after_tree = repo.find_tree(after)?;
    let mut options = DiffOptions::new();
    options.context_lines(3).include_typechange(true);
    let mut diff =
        repo.diff_tree_to_tree(Some(&before_tree), Some(&after_tree), Some(&mut options))?;
    diff.find_similar(Some(git2::DiffFindOptions::new().renames(true)))?;
    for index in 0..diff.deltas().len() {
        cancel.check()?;
        let delta = diff.get_delta(index).context(i18n::tr!(ComposeInvalid))?;
        let path = delta
            .new_file()
            .path()
            .or_else(|| delta.old_file().path())
            .context(i18n::tr!(ChangeHasNoPath))?
            .to_owned();
        let mut patch = Patch::from_diff(&diff, index)?;
        let splittable = delta.status() == Delta::Modified
            && delta.old_file().mode() == delta.new_file().mode()
            && matches!(
                delta.new_file().mode(),
                git2::FileMode::Blob | git2::FileMode::BlobExecutable
            )
            && patch.as_ref().is_some_and(|patch| patch.num_hunks() > 0);
        if splittable {
            let patch = patch.as_mut().expect("text patch with hunks");
            for hunk in 0..patch.num_hunks() {
                let range = {
                    let hunk = patch.hunk(hunk)?.0;
                    (hunk.old_start(), hunk.new_start())
                };
                let mut text = Vec::new();
                let printed = patch.print(&mut |_, current, line| {
                    if current.is_none_or(|current| {
                        current.old_start() == range.0 && current.new_start() == range.1
                    }) {
                        if matches!(line.origin(), '+' | '-' | ' ') {
                            text.push(line.origin() as u8);
                        }
                        text.extend_from_slice(line.content());
                    }
                    text.len() <= MAX_CONTEXT
                });
                ensure!(text.len() <= MAX_CONTEXT, i18n::tr!(ComposeTooLarge));
                printed?;
                let text = String::from_utf8(text)?
                    .split_inclusive('\n')
                    .filter(|line| !line.starts_with("index "))
                    .collect::<String>();
                push(
                    units,
                    layer,
                    path.clone(),
                    text.clone(),
                    Change::Hunk { patch: text },
                )?;
            }
        } else {
            let preview = match patch.as_mut() {
                Some(patch) => {
                    let mut bytes = Vec::new();
                    let result = patch.print(&mut |_, _, line| {
                        if matches!(line.origin(), '+' | '-' | ' ') {
                            bytes.push(line.origin() as u8);
                        }
                        bytes.extend_from_slice(line.content());
                        bytes.len() <= MAX_CONTEXT
                    });
                    ensure!(bytes.len() <= MAX_CONTEXT, i18n::tr!(ComposeTooLarge));
                    result?;
                    String::from_utf8(bytes)?
                }
                None => format!("{} → {}\n", delta.old_file().id(), delta.new_file().id()),
            };
            ensure!(preview.len() <= MAX_CONTEXT, i18n::tr!(ComposeTooLarge));
            push(
                units,
                layer,
                path.clone(),
                preview,
                Change::Whole {
                    before: delta.old_file().path().unwrap_or(&path).to_owned(),
                    after: path,
                    source: after,
                },
            )?;
        }
    }
    Ok(())
}
fn push(
    units: &mut Vec<Unit>,
    layer: Layer,
    path: PathBuf,
    preview: String,
    change: Change,
) -> Result<()> {
    ensure!(
        units.len() < 512
            && units.iter().map(|unit| unit.preview.len()).sum::<usize>() + preview.len()
                <= MAX_CONTEXT,
        i18n::tr!(ComposeTooLarge)
    );
    let id = format!(
        "{}{}",
        if layer == Layer::Staged { 'S' } else { 'W' },
        units.len() + 1
    );
    units.push(Unit {
        id,
        layer,
        path,
        preview,
        change,
        dependencies: Vec::new(),
    });
    Ok(())
}
