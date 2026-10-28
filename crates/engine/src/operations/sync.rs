//! Review a current-branch synchronization after one fetch, then reuse exact-target operations.
use super::{pull, push};
use crate::{
    cancel::Cancel,
    domain::{Publication, PushTarget},
    local::{self, branches::upstream, guard::Position},
    transport::Git,
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Unavailable {
    NoRemotes,
    NoUpstream,
    LocalUpstream,
    Detached,
    Unborn,
}
impl Unavailable {
    pub fn label(self) -> &'static str {
        match self {
            Self::NoRemotes => i18n::tr!(SyncNoRemotes),
            Self::NoUpstream => i18n::tr!(SyncNoUpstream),
            Self::LocalUpstream => i18n::tr!(SyncLocalUpstream),
            Self::Detached => i18n::tr!(SyncDetached),
            Self::Unborn => i18n::tr!(SyncUnborn),
        }
    }
}
#[derive(Debug, Clone)]
pub enum Plan {
    Current { path: PathBuf },
    Unavailable { path: PathBuf, reason: Unavailable },
    Pull(pull::Plan),
    Push(push::PushPlan),
}
impl Plan {
    pub fn path(&self) -> &Path {
        match self {
            Self::Current { path } | Self::Unavailable { path, .. } => path,
            Self::Pull(plan) => &plan.path,
            Self::Push(plan) => &plan.path,
        }
    }
    pub fn notice(&self) -> Option<&'static str> {
        match self {
            Self::Current { .. } => Some(i18n::tr!(SyncCurrent)),
            Self::Unavailable { reason, .. } => Some(reason.label()),
            Self::Pull(_) | Self::Push(_) => None,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "update", rename_all = "snake_case")]
pub enum Change {
    Current,
    Unavailable { reason: Unavailable },
    Pulled { branch: String, oid: String },
    Pushed { target: PushTarget },
}
impl Change {
    pub fn detail(&self) -> String {
        match self {
            Self::Current => i18n::tr!(SyncCurrent).into(),
            Self::Unavailable { reason } => reason.label().into(),
            Self::Pulled { branch, oid } => i18n::tf!(SyncPulled, branch = branch, oid = oid),
            Self::Pushed { target } => i18n::tf!(
                SyncPushed,
                branch = target.branch,
                commits = target.ahead_commits,
                remote = target.remote,
                reference = target.remote_ref.trim_start_matches("refs/heads/"),
                oid = target.local_oid
            ),
        }
    }
}
pub(crate) enum Execution {
    Local(Result<Change>),
    Published(Publication),
}
impl Execution {
    pub fn into_result(self) -> Result<Change> {
        match self {
            Self::Local(result) => result,
            Self::Published(publication) => published_result(&publication),
        }
    }
}
pub(crate) fn published_result(publication: &Publication) -> Result<Change> {
    match publication {
        Publication::Pushed { target } => Ok(Change::Pushed {
            target: target.clone(),
        }),
        Publication::Failed { phase, error, .. } => {
            anyhow::bail!(i18n::tf!(PhasePhaseError, phase = phase, error = error))
        }
    }
}

pub fn prepare(git: &Git, path: &Path, cancel: &Cancel) -> Result<Plan> {
    cancel.check()?;
    let repo = local::open(path)?;
    local::idle(&repo)?;
    let unavailable = |reason| Plan::Unavailable {
        path: path.into(),
        reason,
    };
    if repo.remotes()?.is_empty() {
        return Ok(unavailable(Unavailable::NoRemotes));
    }
    let before = Position::read(&repo)?;
    let (branch, head) = match &before {
        Position::Attached { name, oid } => (
            name.strip_prefix("refs/heads/")
                .context(i18n::tr!(HeadIsNotAttachedToALocal))?,
            *oid,
        ),
        Position::Detached(_) => return Ok(unavailable(Unavailable::Detached)),
        Position::Unborn(_) => return Ok(unavailable(Unavailable::Unborn)),
    };
    match upstream::read(&repo, branch, head)? {
        upstream::Tracking::Absent => return Ok(unavailable(Unavailable::NoUpstream)),
        upstream::Tracking::Local { .. } => return Ok(unavailable(Unavailable::LocalUpstream)),
        upstream::Tracking::Remote { .. } => {}
    }
    let source = pull::Source::read(&repo, branch)?;
    let pull::Source::Remote { remote, .. } = &source else {
        anyhow::bail!(i18n::tr!(MutationChanged))
    };
    git.fetch_tracking(
        path,
        &local::branches::fetch::Remote::read(&repo, remote)?,
        cancel,
    )?;
    ensure!(Position::read(&repo)? == before, i18n::tr!(MutationChanged));
    ensure!(
        pull::Source::read(&repo, branch)? == source,
        i18n::tr!(MutationChanged)
    );
    let upstream = repo
        .find_reference(source.reference())
        .context(i18n::tr!(RemoteUpstreamBranchDoesNotExistAlt))?
        .peel_to_commit()?
        .id();
    let (ahead, behind) = repo.graph_ahead_behind(head, upstream)?;
    let fetched = [remote.clone()];
    let plan = match (ahead, behind) {
        (0, 0) => Plan::Current { path: path.into() },
        (0, _) => Plan::Pull(pull::prepare_fetched(git, path, &fetched, cancel)?),
        (_, 0) => Plan::Push(push::prepare_fetched(git, path, &fetched, cancel)?),
        _ => anyhow::bail!(i18n::tr!(SyncDiverged)),
    };
    ensure!(Position::read(&repo)? == before, i18n::tr!(MutationChanged));
    ensure!(
        pull::Source::read(&repo, branch)? == source,
        i18n::tr!(MutationChanged)
    );
    Ok(plan)
}

pub(crate) fn execute(git: &Git, plan: &Plan, cancel: &Cancel) -> Execution {
    match plan {
        Plan::Current { .. } => Execution::Local(Ok(Change::Current)),
        Plan::Unavailable { reason, .. } => {
            Execution::Local(Ok(Change::Unavailable { reason: *reason }))
        }
        Plan::Pull(plan) => {
            Execution::Local(pull::execute(git, plan, cancel).map(|()| Change::Pulled {
                branch: plan.branch.clone(),
                oid: plan.oid.to_string(),
            }))
        }
        Plan::Push(plan) => Execution::Published(push::execute(git, plan, cancel)),
    }
}
