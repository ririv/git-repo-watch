//! Validate reviewed identities before transferring an exact commit with an explicit lease.
use super::{
    PushPlan,
    plan::{Update, Upstream},
};
use crate::{
    cancel::Cancel,
    domain::{Phase, Publication},
    local::{self, branches},
    transport::{Git, PushMode},
};
use anyhow::{Result, ensure};

pub(super) fn run(git: &Git, plan: &PushPlan, cancel: &Cancel) -> Publication {
    let mut phase = Phase::Validate;
    let attempt = (|| -> Result<()> {
        cancel.check()?;
        let repo = local::open(&plan.path)?;
        plan.validate(&repo)?;
        let mapping = branches::fetch::Remote::read(&repo, &plan.identity.remote)?;
        let tracking = mapping.upstream(&plan.identity.reference)?;
        ensure!(
            git.branch_tip(
                &plan.path,
                &plan.identity.remote,
                &plan.identity.reference,
                cancel
            )? == plan.update.expected(),
            i18n::tr!(RemoteBranchChangedAfterCheckingPushCancelled)
        );
        plan.validate(&repo)?;
        let target = plan.target();
        if !matches!(plan.update, Update::Track { .. }) {
            let mode = match plan.update {
                Update::Create { .. } => PushMode::Create,
                Update::Advance { .. } => PushMode::FastForward,
                Update::Overwrite { expected, .. } => PushMode::ForceWithLease(expected),
                Update::Track { .. } => unreachable!(),
            };
            phase = Phase::Transfer;
            git.push_mode(&plan.path, &target, mode, cancel)?;
        }
        phase = Phase::Verify;
        ensure!(
            git.branch_tip(
                &plan.path,
                &plan.identity.remote,
                &plan.identity.reference,
                cancel
            )? == Some(plan.identity.oid),
            i18n::tr!(PushCommandReturnedButTheRemoteCommit)
        );
        if plan.identity.upstream == Upstream::Absent {
            plan.validate(&repo)?;
            ensure!(
                branches::resolve(&repo, &tracking)? == plan.identity.oid,
                i18n::tr!(RemoteUpstreamReferenceIsInconsistentCheckFetch)
            );
            git.set_upstream(&plan.path, &plan.identity.branch, &tracking, cancel)?;
            ensure!(
                Upstream::read(&repo, &plan.identity.branch)?
                    == Upstream::Configured {
                        remote: plan.identity.remote.clone(),
                        reference: plan.identity.reference.clone()
                    },
                i18n::tr!(MutationChanged)
            );
        }
        Ok(())
    })();
    match attempt {
        Ok(()) => Publication::Pushed {
            target: plan.target(),
        },
        Err(error) => Publication::Failed {
            target: plan.target(),
            phase,
            error: format!("{error:#}"),
        },
    }
}
