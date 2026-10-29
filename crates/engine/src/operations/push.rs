//! Exact-object publication; interactive force pushes are distinct from automated forwarding.
mod execute;
mod plan;
mod prepare;
use crate::{
    cancel::Cancel,
    domain::{Phase, Publication, PushTarget, RemoteTip},
    local::{self, branches},
    transport::Git,
};
use anyhow::{Result, ensure};
pub use plan::{Preparation, PushPlan, Remote, TargetRequest};
use std::path::Path;

pub fn prepare(git: &Git, path: &Path, cancel: &Cancel) -> Result<PushPlan> {
    prepare::existing(git, path, &[], cancel)
}
/// Prepare against tracking references already refreshed under the repository lock.
pub(crate) fn prepare_fetched(
    git: &Git,
    path: &Path,
    remotes: &[String],
    cancel: &Cancel,
) -> Result<PushPlan> {
    prepare::existing(git, path, remotes, cancel)
}
pub fn prepare_interactive(git: &Git, path: &Path, cancel: &Cancel) -> Result<Preparation> {
    prepare::interactive(git, path, cancel)
}
pub fn prepare_target(
    git: &Git,
    request: &TargetRequest,
    remote: &str,
    branch: &str,
    cancel: &Cancel,
) -> Result<PushPlan> {
    prepare::target(git, request, remote, branch, cancel)
}
pub fn execute(git: &Git, plan: &PushPlan, cancel: &Cancel) -> Publication {
    execute::run(git, plan, cancel)
}
pub fn forward(git: &Git, path: &Path, target: &PushTarget, cancel: &Cancel) -> Publication {
    let mut phase = Phase::Validate;
    let attempt = (|| -> Result<()> {
        cancel.check()?;
        ensure!(
            target.local_ref.starts_with("refs/heads/")
                && target.remote_ref.starts_with("refs/heads/")
                && target.ahead_commits > 0,
            i18n::tr!(InvalidPushTarget)
        );
        let repo = local::open(path)?;
        local::idle(&repo)?;
        local::head(&repo)?;
        let url = branches::url(&repo, &target.remote)?;
        let local: git2::Oid = target.local_oid.parse()?;
        let RemoteTip::Existing(expected) = &target.remote_oid else {
            anyhow::bail!(i18n::tr!(InvalidPushTarget))
        };
        let remote: git2::Oid = expected.parse()?;
        ensure!(
            branches::resolve(&repo, &target.local_ref)? == local,
            i18n::tr!(LocalBranchChangedAfterCheckingPushCancelled)
        );
        ensure!(
            git.head(path, &target.remote, &target.remote_ref, cancel)? == remote,
            i18n::tr!(RemoteBranchChangedAfterCheckingPushCancelled)
        );
        ensure!(
            repo.graph_descendant_of(local, remote)?,
            i18n::tr!(LocalCommitIsNotADescendantOf)
        );
        ensure!(
            branches::url(&repo, &target.remote)? == url,
            i18n::tr!(RemoteUrlChangedDuringValidationPushCancelled)
        );
        ensure!(
            branches::resolve(&repo, &target.local_ref)? == local,
            i18n::tr!(LocalBranchChangedDuringValidationPushCancelled)
        );
        phase = Phase::Transfer;
        git.push(path, target, cancel)?;
        phase = Phase::Verify;
        ensure!(
            git.head(path, &target.remote, &target.remote_ref, cancel)? == local,
            i18n::tr!(PushCommandReturnedButTheRemoteCommit)
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
