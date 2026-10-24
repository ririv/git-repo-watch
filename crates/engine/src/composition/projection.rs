//! Compile a plan into exact commit and preserved-index trees before any write.
use super::*;
use crate::local;
use anyhow::{Context, Result, ensure};
use git2::{Index, Repository};
use std::collections::HashSet;

pub(super) fn head_tree(repo: &Repository, head: &Head) -> Result<Oid> {
    match head {
        Head::Existing { oid, .. } => Ok(repo.find_commit(*oid)?.tree_id()),
        Head::Initial { .. } => Ok(Index::new()?.write_tree_to(repo)?),
    }
}
pub(super) fn apply(repo: &Repository, base: Oid, unit: &Unit) -> Result<Oid> {
    let tree = repo.find_tree(base)?;
    match &unit.change {
        Change::Hunk { patch } => {
            let diff = git2::Diff::from_buffer(patch.as_bytes())?;
            let mut index = repo.apply_to_tree(&tree, &diff, None)?;
            Ok(index.write_tree_to(repo)?)
        }
        Change::Whole {
            before,
            after,
            source,
        } => {
            let mut index = Index::new()?;
            index.read_tree(&tree)?;
            if index.get_path(before, 0).is_some() {
                index.remove_path(before)?;
            }
            let mut source_index = Index::new()?;
            source_index.read_tree(&repo.find_tree(*source)?)?;
            if let Some(entry) = source_index.get_path(after, 0) {
                index.add(&entry)?;
            }
            Ok(index.write_tree_to(repo)?)
        }
    }
}
pub(super) fn step(
    repo: &Repository,
    snapshot: &Snapshot,
    step: &Step,
    base: Oid,
    index: Oid,
    applied: &mut HashSet<String>,
    cancel: &crate::cancel::Cancel,
) -> Result<(Oid, Oid)> {
    let mut next = base;
    let mut preserved = index;
    // 使用快照顺序应用同一提交中的块，避免模型给出的 ID 顺序改变补丁偏移。
    for unit in snapshot
        .units
        .iter()
        .filter(|unit| step.units.contains(&unit.id))
    {
        cancel.check()?;
        ensure!(
            unit.dependencies.iter().all(|id| applied.contains(id)),
            "{}: {}",
            i18n::tr!(ComposeDependency),
            unit.id
        );
        next = apply(repo, next, unit)
            .with_context(|| format!("{}: {}", i18n::tr!(ComposeDependency), unit.id))?;
        if unit.layer == Layer::Worktree {
            preserved = apply(repo, preserved, unit)?;
        }
        applied.insert(unit.id.clone());
    }
    ensure!(next != base, i18n::tr!(ComposeEmptyStep));
    Ok((next, preserved))
}
pub(super) fn validate(prepared: &Prepared, cancel: &crate::cancel::Cancel) -> Result<()> {
    let repo = local::open(&prepared.snapshot.path)?;
    let mut base = head_tree(&repo, &prepared.snapshot.head)?;
    let mut index = prepared.snapshot.index;
    let mut applied = HashSet::new();
    for commit in &prepared.plan.commits {
        (base, index) = step(
            &repo,
            &prepared.snapshot,
            commit,
            base,
            index,
            &mut applied,
            cancel,
        )?;
    }
    ensure!(
        base == prepared.snapshot.worktree && index == base,
        i18n::tr!(ComposeCoverage)
    );
    Ok(())
}
