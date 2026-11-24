//! Inspect linked checkouts and prepare reviewed worktree operations.
use super::{common_dir, open};
use crate::{cancel::Cancel, transport::Git};
use anyhow::{Context, Result, ensure};
use git2::{BranchType, ErrorCode, Oid, Repository, StatusOptions, WorktreeLockStatus};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Revision {
    Branch { name: String, oid: Oid },
    Detached(Oid),
    Unborn(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Clean,
    Changed,
    IgnoredFiles,
    Operation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    Main,
    Linked,
    Locked(Option<String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    Available {
        git_dir: PathBuf,
        revision: Revision,
        status: Status,
    },
    Unavailable(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub path: PathBuf,
    pub kind: Kind,
    pub state: State,
    pub current: bool,
}

impl Entry {
    pub fn removal_problem(&self) -> Option<&str> {
        if self.current || self.kind == Kind::Main {
            return Some(i18n::tr!(WorktreeRemoveProtected));
        }
        if matches!(self.kind, Kind::Locked(_)) {
            return Some(i18n::tr!(WorktreeLocked));
        }
        match &self.state {
            State::Unavailable(error) => Some(error),
            State::Available {
                status: Status::Clean,
                ..
            } => None,
            State::Available { .. } => Some(i18n::tr!(WorktreeRemoveDirty)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub path: PathBuf,
    pub entries: Vec<Entry>,
}

fn revision(repo: &Repository) -> Result<Revision> {
    match super::branches::checkout::read(repo)? {
        super::branches::checkout::Checkout::Branch { name, oid, .. } => {
            Ok(Revision::Branch { name, oid })
        }
        super::branches::checkout::Checkout::Detached(oid) => Ok(Revision::Detached(oid)),
        super::branches::checkout::Checkout::Unborn(name) => Ok(Revision::Unborn(name)),
    }
}

fn status(repo: &Repository) -> Result<Status> {
    if repo.state() != git2::RepositoryState::Clean {
        return Ok(Status::Operation);
    }
    let mut options = StatusOptions::new();
    // Git 自身的 remove 会删除被忽略文件，预览和执行均先检查。
    options
        .include_untracked(true)
        .include_ignored(true)
        .exclude_submodules(false);
    let statuses = repo.statuses(Some(&mut options))?;
    if statuses.is_empty() {
        Ok(Status::Clean)
    } else if statuses
        .iter()
        .all(|entry| entry.status() == git2::Status::IGNORED)
    {
        Ok(Status::IgnoredFiles)
    } else {
        Ok(Status::Changed)
    }
}

fn entry(path: &Path, kind: Kind, current: &Path, common: &Path, cancel: &Cancel) -> Result<Entry> {
    cancel.check()?;
    let inspected = (|| -> Result<_> {
        let path = fs::canonicalize(path)?;
        let repo = open(&path)?;
        ensure!(common_dir(&repo)? == common, i18n::tr!(MutationChanged));
        Ok((
            path,
            State::Available {
                git_dir: fs::canonicalize(repo.path())?,
                revision: revision(&repo)?,
                status: status(&repo)?,
            },
        ))
    })();
    let (path, state) = match inspected {
        Ok(entry) => entry,
        Err(error) => (path.to_owned(), State::Unavailable(format!("{error:#}"))),
    };
    cancel.check()?;
    Ok(Entry {
        current: path == current,
        path,
        kind,
        state,
    })
}

pub fn read(path: &Path, cancel: &Cancel) -> Result<Snapshot> {
    cancel.check()?;
    let repo = open(path)?;
    let common = common_dir(&repo)?;
    let current = fs::canonicalize(path)?;
    let main = Repository::open(repo.commondir())?;
    let mut entries = Vec::new();
    if let Some(main) = main.workdir() {
        entries.push(entry(main, Kind::Main, &current, &common, cancel)?);
    }
    for name in repo.worktrees()?.iter() {
        cancel.check()?;
        let worktree = repo.find_worktree(name?.context(i18n::tr!(GitPathIsNotUtf8And))?)?;
        let kind = match worktree.is_locked()? {
            WorktreeLockStatus::Unlocked => Kind::Linked,
            WorktreeLockStatus::Locked(reason) => Kind::Locked(reason),
        };
        entries.push(entry(worktree.path(), kind, &current, &common, cancel)?);
    }
    entries.sort_by(|left, right| {
        (!left.current, left.kind != Kind::Main, &left.path).cmp(&(
            !right.current,
            right.kind != Kind::Main,
            &right.path,
        ))
    });
    Ok(Snapshot {
        path: path.to_owned(),
        entries,
    })
}

pub struct BranchCandidate {
    pub name: String,
    pub subject: String,
    pub occupied_at: Option<PathBuf>,
}

pub struct AddCandidates {
    pub branches: Vec<BranchCandidate>,
    pub start: Option<Oid>,
}

fn occupied_branches(repo: &Repository, cancel: &Cancel) -> Result<BTreeMap<String, PathBuf>> {
    let mut occupied = BTreeMap::new();
    let mut inspect = |checkout: &Repository, path: &Path| -> Result<()> {
        cancel.check()?;
        if let Some(name) = checkout
            .find_reference("HEAD")?
            .symbolic_target()?
            .and_then(|reference| reference.strip_prefix("refs/heads/"))
        {
            occupied
                .entry(name.to_owned())
                .or_insert(fs::canonicalize(path)?);
        }
        Ok(())
    };
    let main = Repository::open(repo.commondir())?;
    if let Some(path) = main.workdir() {
        inspect(&main, path)?;
    }
    for name in repo.worktrees()?.iter() {
        let worktree = repo.find_worktree(name?.context(i18n::tr!(GitPathIsNotUtf8And))?)?;
        inspect(&Repository::open_from_worktree(&worktree)?, worktree.path())?;
    }
    Ok(occupied)
}

fn new_branch_start(repo: &Repository) -> Result<Option<Oid>> {
    match repo.head() {
        Ok(head) => Ok(Some(head.peel_to_commit()?.id())),
        Err(error) if error.code() == ErrorCode::UnbornBranch => Ok(None),
        Err(error) => Err(error.into()),
    }
}

pub fn add_candidates(path: &Path, cancel: &Cancel) -> Result<AddCandidates> {
    cancel.check()?;
    let repo = open(path)?;
    let start = new_branch_start(&repo)?;
    let mut occupied = occupied_branches(&repo, cancel)?;
    let branches = super::branches::names::read(path, cancel)?
        .into_iter()
        .map(|entry| BranchCandidate {
            occupied_at: occupied.remove(&entry.name),
            name: entry.name,
            subject: entry.subject,
        })
        .collect();
    Ok(AddCandidates { branches, start })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Branch {
    Existing,
    New,
}

#[derive(Debug, Clone)]
pub enum Request {
    Add {
        branch: Branch,
        name: String,
        directory: PathBuf,
    },
    Remove(Entry),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Operation {
    Add {
        branch: Branch,
        name: String,
        directory: PathBuf,
        start: Oid,
    },
    Remove(Entry),
}

#[derive(Debug, Clone)]
pub struct Plan {
    repository: PathBuf,
    git_dir: PathBuf,
    operation: Operation,
}

impl Plan {
    pub fn repository(&self) -> &Path {
        &self.repository
    }
    pub fn directory(&self) -> &Path {
        match &self.operation {
            Operation::Add { directory, .. } => directory,
            Operation::Remove(entry) => &entry.path,
        }
    }
    pub fn removing(&self) -> bool {
        matches!(self.operation, Operation::Remove(_))
    }
    pub fn commit(&self) -> Option<Oid> {
        match &self.operation {
            Operation::Add { start, .. } => Some(*start),
            Operation::Remove(Entry {
                state:
                    State::Available {
                        revision: Revision::Branch { oid, .. } | Revision::Detached(oid),
                        ..
                    },
                ..
            }) => Some(*oid),
            Operation::Remove(_) => None,
        }
    }
    pub fn branch(&self) -> Option<&str> {
        match &self.operation {
            Operation::Add { name, .. } => Some(name),
            Operation::Remove(Entry {
                state:
                    State::Available {
                        revision: Revision::Branch { name, .. } | Revision::Unborn(name),
                        ..
                    },
                ..
            }) => Some(name),
            Operation::Remove(_) => None,
        }
    }
}

fn destination(path: &Path) -> Result<PathBuf> {
    ensure!(path.is_absolute(), i18n::tr!(EnterAnAbsolutePath));
    let name = path.file_name().context(i18n::tr!(FolderInvalidPath))?;
    let parent = fs::canonicalize(path.parent().context(i18n::tr!(FolderInvalidPath))?)?;
    ensure!(parent.is_dir(), i18n::tr!(FolderInvalidPath));
    let path = parent.join(name);
    match fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(path),
        Err(error) => Err(error.into()),
        Ok(_) => anyhow::bail!(i18n::tr!(WorktreePathExists)),
    }
}

pub fn prepare(path: &Path, request: Request, cancel: &Cancel) -> Result<Plan> {
    cancel.check()?;
    let repo = open(path)?;
    let git_dir = fs::canonicalize(repo.path())?;
    let operation = match request {
        Request::Add {
            branch,
            name,
            directory,
        } => {
            ensure!(
                !name.starts_with('-') && git2::Branch::name_is_valid(&name)?,
                i18n::tr!(MutationLocalBranchRequired)
            );
            let directory = destination(&directory)?;
            let existing = match repo.find_branch(&name, BranchType::Local) {
                Ok(found) => Some(found),
                Err(error) if error.code() == ErrorCode::NotFound => None,
                Err(error) => return Err(error.into()),
            };
            let start = match branch {
                Branch::New => {
                    ensure!(existing.is_none(), i18n::tr!(WorktreeBranchExists));
                    new_branch_start(&repo)?.context(i18n::tr!(WorktreeNewBranchNeedsCommit))?
                }
                Branch::Existing => {
                    let entry = existing.context(i18n::tr!(MutationLocalBranchRequired))?;
                    if let Some(path) = occupied_branches(&repo, cancel)?.get(&name) {
                        anyhow::bail!(format!(
                            "{} {}",
                            i18n::tr!(WorktreeBranchOccupied),
                            path.display()
                        ));
                    }
                    entry.get().peel_to_commit()?.id()
                }
            };
            Operation::Add {
                branch,
                name,
                directory,
                start,
            }
        }
        Request::Remove(expected) => {
            // 核对选中的登记项，而不是将任意路径当作可移除的工作树。
            let snapshot = read(path, cancel)?;
            let current = snapshot
                .entries
                .into_iter()
                .find(|entry| entry.path == expected.path)
                .context(i18n::tr!(MutationChanged))?;
            ensure!(current == expected, i18n::tr!(MutationChanged));
            if let Some(reason) = current.removal_problem() {
                anyhow::bail!("{reason}");
            }
            Operation::Remove(current)
        }
    };
    Ok(Plan {
        repository: path.to_owned(),
        git_dir,
        operation,
    })
}

pub fn execute(git: &Git, plan: &Plan, cancel: &Cancel) -> Result<()> {
    let request = match &plan.operation {
        Operation::Add {
            branch,
            name,
            directory,
            ..
        } => Request::Add {
            branch: *branch,
            name: name.clone(),
            directory: directory.clone(),
        },
        Operation::Remove(entry) => Request::Remove(entry.clone()),
    };
    let current = prepare(&plan.repository, request, cancel)?;
    ensure!(
        current.git_dir == plan.git_dir && current.operation == plan.operation,
        i18n::tr!(MutationChanged)
    );
    match &plan.operation {
        Operation::Add {
            branch,
            name,
            directory,
            start,
        } => git.add_worktree(
            &plan.repository,
            directory,
            name,
            (*branch == Branch::New).then_some(*start),
            cancel,
        ),
        Operation::Remove(entry) => git.remove_worktree(&plan.repository, &entry.path, cancel),
    }
}
