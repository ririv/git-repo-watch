//! Reviewed resets move the current branch and explicitly control which local changes survive.
mod hard;
use crate::{
    cancel::Cancel,
    local::{self, guard::Position, index_lock::IndexLock},
    transport::Git,
};
use anyhow::{Context, Result, ensure};
use git2::{Oid, Sort};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const PREVIEW_LIMIT: usize = 30;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    #[default]
    Soft,
    Hard,
}
impl Mode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Soft => i18n::tr!(ResetCommand),
            Self::Hard => i18n::tr!(ResetHardCommand),
        }
    }
    pub fn token(self) -> &'static str {
        match self {
            Self::Soft => "soft",
            Self::Hard => "hard",
        }
    }
}

#[derive(Debug, Clone)]
pub enum Effect {
    Soft,
    Hard(hard::Review),
}

#[derive(Debug, Clone)]
pub struct Commit {
    pub oid: Oid,
    pub subject: String,
}

#[derive(Debug, Clone)]
pub struct Plan {
    path: PathBuf,
    reference: String,
    before: Oid,
    target: Commit,
    removed: Vec<Commit>,
    removed_count: usize,
    effect: Effect,
}

impl Plan {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn branch(&self) -> &str {
        self.reference.trim_start_matches("refs/heads/")
    }

    pub fn before(&self) -> Oid {
        self.before
    }

    pub fn target(&self) -> &Commit {
        &self.target
    }

    pub fn removed(&self) -> (&[Commit], usize) {
        (&self.removed, self.removed_count)
    }

    pub fn effect(&self) -> &Effect {
        &self.effect
    }

    pub fn mode(&self) -> Mode {
        match self.effect {
            Effect::Soft => Mode::Soft,
            Effect::Hard(_) => Mode::Hard,
        }
    }
}

fn preview(commit: &git2::Commit<'_>) -> Commit {
    Commit {
        oid: commit.id(),
        subject: commit
            .summary_bytes()
            .map(|text| String::from_utf8_lossy(text).chars().take(160).collect())
            .unwrap_or_default(),
    }
}

pub fn prepare(path: &Path, target: Oid, mode: Mode, cancel: &Cancel) -> Result<Plan> {
    cancel.check()?;
    let repo = local::open(path)?;
    local::idle(&repo)?;
    let Position::Attached { name, oid: before } = Position::read(&repo)? else {
        anyhow::bail!(i18n::tr!(ResetAttachedRequired));
    };
    ensure!(
        name.starts_with("refs/heads/"),
        i18n::tr!(ResetAttachedRequired)
    );
    ensure!(
        before != target || mode == Mode::Hard,
        i18n::tr!(ResetCurrentCommit)
    );
    let commit = repo.find_commit(target)?;
    ensure!(
        before == target || repo.graph_descendant_of(before, target)?,
        i18n::tr!(ResetAncestorRequired)
    );
    let mut walk = repo.revwalk()?;
    walk.set_sorting(Sort::TOPOLOGICAL | Sort::TIME)?;
    walk.push(before)?;
    walk.hide(target)?;
    let mut removed = Vec::new();
    let mut removed_count = 0;
    for oid in walk {
        cancel.check()?;
        let oid = oid?;
        if removed.len() < PREVIEW_LIMIT {
            removed.push(preview(&repo.find_commit(oid)?));
        }
        removed_count += 1;
    }
    let effect = match mode {
        Mode::Soft => Effect::Soft,
        Mode::Hard => Effect::Hard(hard::Review::capture(path, &repo, &commit.tree()?, cancel)?),
    };
    Ok(Plan {
        path: path.into(),
        reference: name,
        before,
        target: preview(&commit),
        removed,
        removed_count,
        effect,
    })
}

pub fn execute(git: &Git, plan: &Plan, cancel: &Cancel) -> Result<()> {
    cancel.check()?;
    let repo = local::open(plan.path())?;
    if let Effect::Hard(review) = &plan.effect {
        validate_position(&repo, plan)?;
        let tree = repo.find_commit(plan.target.oid)?.tree()?;
        review.validate(plan.path(), &repo, &tree, cancel)?;
        return git.reset_hard(plan.path(), &repo, plan.target.oid, cancel);
    }
    let index = repo.index()?;
    let _index = IndexLock::new(index.path().context(i18n::tr!(MutationChanged))?)?;
    let mut head = repo.transaction()?;
    head.lock_ref("HEAD")?;
    let mut branch = repo.transaction()?;
    branch.lock_ref(&plan.reference)?;
    validate_position(&repo, plan)?;
    repo.find_commit(plan.target.oid)?;
    let message = format!("reset: moving to {}", plan.target.oid);
    branch.set_target(&plan.reference, plan.target.oid, None, &message)?;
    cancel.check()?;
    // 先记录撤回前的位置。分支更新后不再取消，HEAD 锁一直持有到引用写完。
    repo.reference("ORIG_HEAD", plan.before, true, &message)?;
    branch.commit()?;
    Ok(())
}

fn validate_position(repo: &git2::Repository, plan: &Plan) -> Result<()> {
    local::idle(repo)?;
    ensure!(
        Position::read(repo)?
            == (Position::Attached {
                name: plan.reference.clone(),
                oid: plan.before,
            }),
        i18n::tr!(MutationChanged)
    );
    Ok(())
}
