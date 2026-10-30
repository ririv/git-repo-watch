//! One confirmed commit using an alternate index and an atomic real-index update.
use super::*;
use crate::{
    cancel::Cancel,
    local,
    operations::commit::{CommitResult, Review},
    transport::Git,
};
use anyhow::{Context, Result, ensure};
use std::{fs, path::Path};

#[derive(Debug, Clone)]
pub enum Progress {
    More {
        prepared: Prepared,
        result: CommitResult,
    },
    Complete {
        completed: Vec<CommitResult>,
    },
}
use crate::local::index_lock::IndexLock;
fn preserve_index(repo: &git2::Repository, path: &Path, desired: git2::Oid) -> Result<()> {
    let mut index = git2::Index::open(path)?;
    let before = index.write_tree_to(repo)?;
    let mut source = git2::Index::new()?;
    source.read_tree(&repo.find_tree(desired)?)?;
    let diff = repo.diff_tree_to_tree(
        Some(&repo.find_tree(before)?),
        Some(&repo.find_tree(desired)?),
        None,
    )?;
    // 仅改动参与本次提交的条目，保留其它条目的暂存意图、扩展标记和缓存。
    for delta in diff.deltas() {
        let name = delta
            .new_file()
            .path()
            .or_else(|| delta.old_file().path())
            .context(i18n::tr!(ChangeHasNoPath))?;
        if let Some(mut entry) = source.get_path(name, 0) {
            if let Some(previous) = index.get_path(name, 0) {
                entry.flags = previous.flags;
                entry.flags_extended = previous.flags_extended;
            }
            index.add(&entry)?;
        } else if index.get_path(name, 0).is_some() {
            index.remove_path(name)?;
        }
    }
    index.write()?;
    Ok(())
}
pub fn execute(git: &Git, mut prepared: Prepared, cancel: &Cancel) -> Result<Progress> {
    cancel.check()?;
    prepared.validate_with_cancel(cancel)?;
    let snapshot = &prepared.snapshot;
    let repo = local::open(&snapshot.path)?;
    let index_path = repo
        .index()?
        .path()
        .context(i18n::tr!(ComposeInvalid))?
        .to_owned();
    let mut lock = IndexLock::new(&index_path)?;
    capture::validate(snapshot, cancel)?;
    let base = projection::head_tree(&repo, &snapshot.head)?;
    let step = &prepared.plan.commits[0];
    let mut applied = std::collections::HashSet::new();
    let (next, preserved) = projection::step(
        &repo,
        snapshot,
        step,
        base,
        snapshot.index,
        &mut applied,
        cancel,
    )?;
    let directory = tempfile::tempdir_in(repo.path())?;
    let alternate = directory.path().join("commit-index");
    let pending = directory.path().join("preserved-index");
    let mut index = git2::Index::open(&alternate)?;
    index.read_tree(&repo.find_tree(next)?)?;
    index.write()?;
    match fs::copy(&index_path, &pending) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    preserve_index(&repo, &pending, preserved)?;
    lock.prepare(&fs::read(&pending)?)?;
    capture::validate(snapshot, cancel)?;
    let outcome = git.commit_index(&snapshot.path, &alternate, &step.message, cancel);
    let after = local::head(&local::open(&snapshot.path)?)?;
    if after == snapshot.head {
        outcome?;
        anyhow::bail!(i18n::tr!(ComposeCommitMissing));
    }
    let Head::Existing { oid, ref branch } = after else {
        anyhow::bail!(i18n::tr!(ComposeChanged));
    };
    let actual = repo.find_commit(oid)?;
    let parent_matches = match snapshot.head {
        Head::Initial { .. } => actual.parent_count() == 0,
        Head::Existing { oid, .. } => actual.parent_count() == 1 && actual.parent_id(0)? == oid,
    };
    ensure!(
        parent_matches && branch == snapshot.head.branch(),
        "{}: {oid}",
        i18n::tr!(ComposeCommitStopped)
    );
    // 钩子可能额外暂存内容；只在可无冲突保留原暂存意图时同步真实索引，随后停止编排。
    if actual.tree_id() != next {
        let mut merged = repo.merge_trees(
            &repo.find_tree(next)?,
            &actual.tree()?,
            &repo.find_tree(preserved)?,
            None,
        )?;
        ensure!(
            !merged.has_conflicts(),
            "{}: {oid}",
            i18n::tr!(ComposeHookConflict)
        );
        let tree = merged.write_tree_to(&repo)?;
        preserve_index(&repo, &pending, tree)?;
        lock.prepare(&fs::read(&pending)?)?;
        lock.install()?;
        anyhow::bail!("{}: {oid}", i18n::tr!(ComposeHookChanged));
    }
    lock.install()?;
    outcome.with_context(|| format!("{}: {oid}", i18n::tr!(ComposeCommitStopped)))?;
    let result = CommitResult {
        oid: oid.to_string(),
        review: Review::Exact,
    };
    prepared.completed.push(result.clone());
    prepared.plan.commits.remove(0);
    let (head, index, worktree) = capture::trees(&snapshot.path, &Cancel::default())?;
    ensure!(
        head == after && index == preserved && worktree == snapshot.worktree,
        "{}: {oid}",
        i18n::tr!(ComposeCommitStopped)
    );
    if prepared.plan.commits.is_empty() {
        return Ok(Progress::Complete {
            completed: prepared.completed,
        });
    }
    prepared.snapshot.head = head;
    prepared.snapshot.index = index;
    prepared
        .snapshot
        .units
        .retain(|unit| !applied.contains(&unit.id));
    for unit in &mut prepared.snapshot.units {
        unit.dependencies.retain(|id| !applied.contains(id));
    }
    prepared.validate_structure()?;
    Ok(Progress::More { prepared, result })
}
