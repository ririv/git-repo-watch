//! Stash snapshots and reviewed operations use object identities, never stale indices.
use super::{
    guard::{Guard, Position},
    idle, open,
};
use crate::cancel::Cancel;
use anyhow::{Result, ensure};
use git2::Oid;
pub mod preview;
mod save;
pub use save::{Message, SaveMode, SavePlan, SelectedFiles};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Entry {
    pub index: usize,
    pub oid: Oid,
    pub message: String,
    pub time: i64,
}
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub path: PathBuf,
    pub entries: Vec<Entry>,
}
pub fn read(path: &Path, cancel: &Cancel) -> Result<Snapshot> {
    let mut repo = open(path)?;
    cancel.check()?;
    let mut entries = Vec::new();
    repo.stash_foreach(|index, message, oid| {
        entries.push((index, *oid, message.to_owned()));
        true
    })?;
    let mut result = Vec::new();
    for (index, oid, message) in entries {
        cancel.check()?;
        result.push(Entry {
            index,
            oid,
            message,
            time: repo.find_commit(oid)?.time().seconds(),
        });
    }
    Ok(Snapshot {
        path: path.into(),
        entries: result,
    })
}
#[derive(Debug, Clone, Copy)]
pub enum Restore {
    Apply,
    Pop,
}
#[derive(Debug, Clone)]
pub enum Request {
    Save { mode: SaveMode, message: Message },
    Apply(Oid),
    Pop(Oid),
    Drop(Oid),
}
#[derive(Debug, Clone)]
pub enum Plan {
    Save(SavePlan),
    Restore {
        path: PathBuf,
        entry: Entry,
        order: Vec<Oid>,
        guard: Guard,
        action: Restore,
    },
    Drop {
        path: PathBuf,
        entry: Entry,
        order: Vec<Oid>,
    },
}
impl Plan {
    pub fn path(&self) -> &Path {
        match self {
            Self::Save(plan) => &plan.path,
            Self::Restore { path, .. } | Self::Drop { path, .. } => path,
        }
    }
}
pub fn prepare(path: &Path, request: Request, cancel: &Cancel) -> Result<Plan> {
    let repo = open(path)?;
    idle(&repo)?;
    cancel.check()?;
    match request {
        Request::Save { mode, message } => {
            Ok(Plan::Save(save::prepare(path, mode, message, cancel)?))
        }
        request => {
            let oid = match request {
                Request::Apply(oid) | Request::Pop(oid) | Request::Drop(oid) => oid,
                _ => unreachable!(),
            };
            let snapshot = read(path, cancel)?;
            let entries = snapshot
                .entries
                .iter()
                .filter(|entry| entry.oid == oid)
                .collect::<Vec<_>>();
            ensure!(entries.len() == 1, i18n::tr!(MutationStashChanged));
            let entry = (*entries[0]).clone();
            let order = snapshot.entries.iter().map(|entry| entry.oid).collect();
            if matches!(request, Request::Drop(_)) {
                return Ok(Plan::Drop {
                    path: path.into(),
                    entry,
                    order,
                });
            }
            super::guard::clean_tracked(&repo)?;
            Ok(Plan::Restore {
                path: path.into(),
                entry,
                order,
                guard: Guard::capture(path, cancel)?,
                action: match request {
                    Request::Apply(_) => Restore::Apply,
                    Request::Pop(_) => Restore::Pop,
                    _ => unreachable!(),
                },
            })
        }
    }
}
pub fn execute(git: &crate::transport::Git, plan: &Plan, cancel: &Cancel) -> Result<Oid> {
    cancel.check()?;
    let mut repo = open(plan.path())?;
    idle(&repo)?;
    match plan {
        Plan::Save(plan) => plan.execute(git, cancel),
        Plan::Restore {
            path,
            entry,
            order,
            guard,
            action,
        } => {
            guard.validate(path, cancel)?;
            super::guard::clean_tracked(&repo)?;
            let index = selected(path, entry.oid, order, cancel)?;
            let mut options = git2::StashApplyOptions::new();
            options.progress_cb(|_| !cancel.is_cancelled());
            // libgit2 恢复冲突时仍可能返回成功；确认无冲突后才删除原 stash。
            repo.stash_apply(index, Some(&mut options))?;
            let mut restored = repo.index()?;
            restored.read(true)?;
            ensure!(!restored.has_conflicts(), i18n::tr!(MutationStashConflicts));
            if matches!(action, Restore::Pop) {
                let index = selected(path, entry.oid, order, cancel)?;
                repo.stash_drop(index)?;
            }
            Ok(entry.oid)
        }
        Plan::Drop { path, entry, order } => {
            let index = selected(path, entry.oid, order, cancel)?;
            repo.stash_drop(index)?;
            Ok(entry.oid)
        }
    }
}
fn selected(path: &Path, oid: Oid, expected: &[Oid], cancel: &Cancel) -> Result<usize> {
    let current = read(path, cancel)?;
    let order = current
        .entries
        .iter()
        .map(|entry| entry.oid)
        .collect::<Vec<_>>();
    ensure!(order == expected, i18n::tr!(MutationStashChanged));
    order
        .iter()
        .position(|current| *current == oid)
        .ok_or_else(|| anyhow::anyhow!(i18n::tr!(MutationStashChanged)))
}
