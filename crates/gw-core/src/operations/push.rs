//! Reviewed, exact-OID, non-forced publication with remote verification.
use crate::{
    cancel::Cancel,
    domain::{Phase, Publication, PushTarget},
    local::{
        self, Head,
        branches::{self, Tracking},
    },
    transport::Git,
};
use anyhow::{Context, Result, ensure};
use std::path::{Path, PathBuf};
#[derive(Debug, Clone)]
pub struct PushPlan {
    pub path: PathBuf,
    pub target: PushTarget,
    pub url: String,
}
pub fn prepare(git: &Git, path: &Path, cancel: &Cancel) -> Result<PushPlan> {
    let repo = local::open(path)?;
    local::idle(&repo)?;
    let head = local::head(&repo)?;
    let Head::Existing {
        branch: ref name,
        oid,
    } = head
    else {
        anyhow::bail!(gw_i18n::tr!(CurrentBranchHasNoCommits))
    };
    let branch = branches::list(&repo)?
        .into_iter()
        .find(|b| b.name == *name)
        .context(gw_i18n::tr!(CurrentLocalBranchDoesNotExist))?;
    let Tracking::Remote {
        remote,
        remote_ref,
        upstream,
    } = branch.tracking
    else {
        anyhow::bail!(gw_i18n::tr!(CurrentBranchHasNoRemoteUpstreamExplicitly))
    };
    let url = branches::url(&repo, &remote)?;
    drop(repo);
    git.fetch(path, &remote, cancel)?;
    let server = git.head(path, &remote, &remote_ref, cancel)?;
    let repo = local::open(path)?;
    ensure!(
        branches::resolve(&repo, &upstream)? == server,
        gw_i18n::tr!(RemoteUpstreamReferenceIsInconsistentCheckFetch)
    );
    let (ahead, behind) = repo.graph_ahead_behind(oid, server)?;
    ensure!(
        behind == 0,
        gw_i18n::tf!(
            CurrentBranchIsAheadCommitsAheadAnd,
            ahead = ahead,
            behind = behind
        )
    );
    ensure!(
        ahead > 0,
        gw_i18n::tr!(CurrentBranchMatchesTheRemoteNothingTo)
    );
    ensure!(
        local::head(&repo)? == head,
        gw_i18n::tr!(BranchOrHeadChangedDuringPreparationStart)
    );
    Ok(PushPlan {
        path: path.to_owned(),
        target: PushTarget {
            branch: name.clone(),
            local_ref: branch.reference,
            local_oid: oid.to_string(),
            remote,
            remote_ref,
            remote_oid: server.to_string(),
            ahead_commits: ahead,
        },
        url,
    })
}
pub fn execute(git: &Git, plan: &PushPlan, cancel: &Cancel) -> Publication {
    match validate_review(plan) {
        Ok(()) => forward(git, &plan.path, &plan.target, cancel),
        Err(e) => Publication::Failed {
            target: plan.target.clone(),
            phase: Phase::Validate,
            error: format!("{e:#}"),
        },
    }
}
fn validate_review(plan: &PushPlan) -> Result<()> {
    let repo = local::open(&plan.path)?;
    ensure!(
        local::head(&repo)?
            == Head::Existing {
                branch: plan.target.branch.clone(),
                oid: plan.target.local_oid.parse()?
            },
        gw_i18n::tr!(BranchOrHeadChangedReviewThePush)
    );
    ensure!(
        branches::url(&repo, &plan.target.remote)? == plan.url,
        gw_i18n::tr!(RemoteUrlChangedReviewThePushAgain)
    );
    Ok(())
}
pub fn forward(git: &Git, path: &Path, target: &PushTarget, cancel: &Cancel) -> Publication {
    let mut phase = Phase::Validate;
    let attempt = (|| -> Result<()> {
        cancel.check()?;
        ensure!(
            target.local_ref.starts_with("refs/heads/")
                && target.remote_ref.starts_with("refs/heads/")
                && target.ahead_commits > 0,
            gw_i18n::tr!(InvalidPushTarget)
        );
        let repo = local::open(path)?;
        local::idle(&repo)?;
        local::head(&repo)?;
        let url = branches::url(&repo, &target.remote)?;
        let local: git2::Oid = target.local_oid.parse()?;
        let remote: git2::Oid = target.remote_oid.parse()?;
        ensure!(
            branches::resolve(&repo, &target.local_ref)? == local,
            gw_i18n::tr!(LocalBranchChangedAfterCheckingPushCancelled)
        );
        ensure!(
            git.head(path, &target.remote, &target.remote_ref, cancel)? == remote,
            gw_i18n::tr!(RemoteBranchChangedAfterCheckingPushCancelled)
        );
        ensure!(
            repo.graph_descendant_of(local, remote)?,
            gw_i18n::tr!(LocalCommitIsNotADescendantOf)
        );
        ensure!(
            branches::url(&repo, &target.remote)? == url,
            gw_i18n::tr!(RemoteUrlChangedDuringValidationPushCancelled)
        );
        ensure!(
            branches::resolve(&repo, &target.local_ref)? == local,
            gw_i18n::tr!(LocalBranchChangedDuringValidationPushCancelled)
        );
        phase = Phase::Transfer;
        git.push(path, target, cancel)?;
        phase = Phase::Verify;
        ensure!(
            git.head(path, &target.remote, &target.remote_ref, cancel)? == local,
            gw_i18n::tr!(PushCommandReturnedButTheRemoteCommit)
        );
        Ok(())
    })();
    match attempt {
        Ok(()) => Publication::Pushed {
            target: target.clone(),
        },
        Err(e) => Publication::Failed {
            target: target.clone(),
            phase,
            error: format!("{e:#}"),
        },
    }
}
