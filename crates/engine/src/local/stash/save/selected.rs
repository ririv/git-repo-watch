//! Build a path-scoped Git stash and clean only reviewed paths under the real index lock.
use super::*;
use crate::local::{files::index::require_supported_filters, index_lock::IndexLock};
use anyhow::Context;
use std::fs;

fn replace(index: &mut git2::Index, source: &git2::Index, files: &[PathBuf]) -> Result<()> {
    for path in files {
        if let Some(entry) = source.get_path(path, 0) {
            index.add(&entry)?;
        } else if index.get_path(path, 0).is_some() {
            index.remove_path(path)?;
        }
    }
    Ok(())
}
fn add_worktree(
    index: &mut git2::Index,
    repo: &git2::Repository,
    root: &Path,
    path: &Path,
) -> Result<()> {
    match fs::symlink_metadata(root.join(path)) {
        Ok(metadata) if metadata.is_file() || metadata.is_symlink() => {
            require_supported_filters(repo, path)?;
            index.add_path(path)?;
        }
        Ok(_) => anyhow::bail!(i18n::tr!(MutationNonRegularFile)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            if index.get_path(path, 0).is_some() {
                index.remove_path(path)?;
            }
        }
        Err(error) => return Err(error.into()),
    }
    Ok(())
}
pub(super) fn execute(plan: &SavePlan, git: &Git, cancel: &Cancel) -> Result<git2::Oid> {
    let repo = open(&plan.path)?;
    let mut index = repo.index()?;
    let path = index
        .path()
        .context(i18n::tr!(StashInvalidPath))?
        .to_owned();
    let mut lock = IndexLock::new(&path)?;
    plan.guard.validate(&plan.path, cancel)?;
    let head = repo.find_commit(plan.guard.position.oid()?)?;
    let head_tree = head.tree()?;
    let mut base = git2::Index::new()?;
    base.read_tree(&head_tree)?;
    let original_tree = index.write_tree()?;
    let mut original = git2::Index::new()?;
    original.read_tree(&repo.find_tree(original_tree)?)?;
    let directory = tempfile::tempdir_in(repo.path())?;
    let cleanup_path = directory.path().join("cleanup-index");
    let mut cleanup = git2::Index::open(&cleanup_path)?;
    cleanup.read_tree(&head_tree)?;
    cleanup.write()?;
    let pending_path = directory.path().join("remaining-index");
    match fs::copy(&path, &pending_path) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    let mut pending = git2::Index::open(&pending_path)?;
    replace(&mut pending, &base, &plan.files)?;
    pending.write()?;
    lock.prepare(&fs::read(&pending_path)?)?;

    // 每个树均从 HEAD 投影，未勾选的暂存项不会进入 stash 的任一层。
    index.read_tree(&head_tree)?;
    replace(&mut index, &original, &plan.files)?;
    let staged = index.write_tree()?;
    let (tracked, untracked): (Vec<_>, Vec<_>) = plan
        .files
        .iter()
        .partition(|path| base.get_path(path, 0).is_some() || original.get_path(path, 0).is_some());
    for path in &tracked {
        cancel.check()?;
        add_worktree(&mut index, &repo, &plan.path, path)?;
    }
    let working = index.write_tree()?;
    index.clear()?;
    for path in &untracked {
        cancel.check()?;
        add_worktree(&mut index, &repo, &plan.path, path)?;
    }
    let untracked_tree = index.write_tree()?;
    let branch = match &plan.guard.position {
        Position::Attached { name, .. } => name.trim_start_matches("refs/heads/"),
        _ => "(no branch)",
    };
    let summary = format!(
        "{branch}: {} {}",
        head.id().to_string().chars().take(7).collect::<String>(),
        head.summary()?.unwrap_or("")
    );
    let signature = repo.signature()?;
    let staged_commit = repo.commit(
        None,
        &signature,
        &signature,
        &format!("index on {summary}"),
        &repo.find_tree(staged)?,
        &[&head],
    )?;
    let mut parents = vec![head, repo.find_commit(staged_commit)?];
    if !untracked.is_empty() {
        let oid = repo.commit(
            None,
            &signature,
            &signature,
            &format!("untracked files on {summary}"),
            &repo.find_tree(untracked_tree)?,
            &[],
        )?;
        parents.push(repo.find_commit(oid)?);
    }
    let message = match plan.message.text() {
        Some(text) => format!("On {branch}: {text}"),
        None => format!("WIP on {summary}"),
    };
    let oid = repo.commit(
        None,
        &signature,
        &signature,
        &message,
        &repo.find_tree(working)?,
        &parents.iter().collect::<Vec<_>>(),
    )?;
    plan.guard.validate(&plan.path, cancel)?;
    repo.reference_ensure_log("refs/stash")?;
    let mut transaction = repo.transaction()?;
    transaction.lock_ref("refs/stash")?;
    transaction.set_target("refs/stash", oid, Some(&signature), &message)?;
    cancel.check()?;
    transaction.commit()?;
    // 先持久保存 Git 对象，再恢复选定路径；失败时保留 stash 与真实状态，交由用户查看。
    git.clean_stashed_files(plan, &cleanup_path, &base, cancel)
        .with_context(|| i18n::tf!(StashSaveIncomplete, oid = oid))?;
    ensure!(
        Position::read(&repo)? == plan.guard.position,
        i18n::tr!(MutationChanged)
    );
    lock.install()
        .with_context(|| i18n::tf!(StashSaveIncomplete, oid = oid))?;
    Ok(oid)
}
