//! Branch creation and safe checkout use immutable local targets.
use crate::{
    cancel::Cancel,
    local::{
        self,
        guard::{Guard, Position},
    },
};
use anyhow::{Context, Result, ensure};
use git2::{BranchType, Oid, Repository};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy)]
pub enum Action {
    Create,
    Switch,
}
#[derive(Debug, Clone)]
pub enum Plan {
    Create {
        path: PathBuf,
        name: String,
        position: Position,
        oid: Oid,
    },
    Switch {
        path: PathBuf,
        name: String,
        oid: Oid,
        guard: Guard,
    },
}
impl Plan {
    pub fn path(&self) -> &Path {
        match self {
            Self::Create { path, .. } | Self::Switch { path, .. } => path,
        }
    }
    pub fn name(&self) -> &str {
        match self {
            Self::Create { name, .. } | Self::Switch { name, .. } => name,
        }
    }
    pub fn oid(&self) -> Oid {
        match self {
            Self::Create { oid, .. } | Self::Switch { oid, .. } => *oid,
        }
    }
}
pub fn prepare(path: &Path, name: &str, action: Action, cancel: &Cancel) -> Result<Plan> {
    ensure!(
        git2::Branch::name_is_valid(name)? && !name.starts_with('-'),
        i18n::tr!(MutationInvalidBranch)
    );
    let repo = local::open(path)?;
    local::idle(&repo)?;
    cancel.check()?;
    match action {
        Action::Switch => {
            local::guard::clean_tracked(&repo)?;
            let branch = repo
                .find_branch(name, BranchType::Local)
                .context(i18n::tr!(MutationLocalBranchRequired))?;
            let reference = branch
                .get()
                .name()
                .context(i18n::tr!(BranchNameIsNotUtf8))?;
            ensure!(
                !matches!(Position::read(&repo)?,Position::Attached{name:current,..} if current==reference),
                i18n::tr!(MutationAlreadyOnBranch)
            );
            available(&repo, reference)?;
            Ok(Plan::Switch {
                path: path.into(),
                name: name.into(),
                oid: branch.get().peel_to_commit()?.id(),
                guard: Guard::capture(path, cancel)?,
            })
        }
        Action::Create => {
            match repo.find_branch(name, BranchType::Local) {
                Ok(_) => anyhow::bail!(i18n::tr!(MutationBranchExists)),
                Err(error) if error.code() == git2::ErrorCode::NotFound => {}
                Err(error) => return Err(error.into()),
            }
            let position = Position::read(&repo)?;
            let oid = position.oid()?;
            Ok(Plan::Create {
                path: path.into(),
                name: name.into(),
                position,
                oid,
            })
        }
    }
}
pub fn execute(plan: &Plan, cancel: &Cancel) -> Result<()> {
    cancel.check()?;
    let repo = local::open(plan.path())?;
    local::idle(&repo)?;
    match plan {
        Plan::Create {
            name,
            position,
            oid,
            ..
        } => {
            ensure!(
                Position::read(&repo)? == *position,
                i18n::tr!(MutationChanged)
            );
            repo.branch(name, &repo.find_commit(*oid)?, false)?;
        }
        Plan::Switch {
            path,
            name,
            oid,
            guard,
        } => {
            guard.validate(path, cancel)?;
            local::guard::clean_tracked(&repo)?;
            let branch = repo.find_branch(name, BranchType::Local)?;
            ensure!(
                branch.get().peel_to_commit()?.id() == *oid,
                i18n::tr!(MutationChanged)
            );
            let reference = branch
                .get()
                .name()
                .context(i18n::tr!(BranchNameIsNotUtf8))?;
            available(&repo, reference)?;
            let target = repo.find_object(*oid, None)?;
            let mut checkout = git2::build::CheckoutBuilder::new();
            checkout.safe();
            checkout
                .notify_on(git2::CheckoutNotificationType::all())
                .notify(|_, _, _, _, _| !cancel.is_cancelled());
            repo.checkout_tree(&target, Some(&mut checkout))?;
            // 检出完成后立即同步 HEAD，不能在两步之间取消而留下错误分支。
            repo.set_head(reference)?;
        }
    }
    Ok(())
}
pub(crate) fn available(repo: &Repository, reference: &str) -> Result<()> {
    let mut others = vec![Repository::open(repo.commondir())?];
    for name in repo.worktrees()?.iter() {
        let name = name?.context(i18n::tr!(GitPathIsNotUtf8And))?;
        others.push(Repository::open_from_worktree(&repo.find_worktree(name)?)?);
    }
    for other in others {
        if std::fs::canonicalize(other.path())? == std::fs::canonicalize(repo.path())? {
            continue;
        }
        ensure!(
            other.find_reference("HEAD")?.symbolic_target()? != Some(reference),
            i18n::tr!(MutationBranchInOtherWorktree)
        );
    }
    Ok(())
}
