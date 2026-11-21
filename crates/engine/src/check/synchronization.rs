//! Capture branch comparisons after remote updates and publications have completed.
use crate::{
    cancel::Cancel,
    domain::{
        BranchWithoutUpstream, UpstreamReason,
        synchronization::{Checkout, RemoteBranch, RemoteComparison, Synchronization},
    },
    local::branches::{
        self, checkout,
        upstream::{Comparison, Tracking},
    },
};
use anyhow::Result;
use git2::{Error, ErrorCode, Oid, Repository};

pub(super) fn read(
    repo: &Repository,
    cancel: &Cancel,
) -> Result<(Synchronization, Vec<BranchWithoutUpstream>)> {
    let checkout = match checkout::read(repo)? {
        checkout::Checkout::Branch {
            name: branch,
            tracking,
            ..
        } => match tracking.map_err(anyhow::Error::msg)? {
            Tracking::Absent => Checkout::NoUpstream { branch },
            Tracking::Local { comparison, .. } | Tracking::Remote { comparison, .. } => {
                match comparison {
                    Comparison::Missing => Checkout::MissingUpstream { branch },
                    Comparison::Known { ahead, behind } => Checkout::Compared {
                        branch,
                        ahead,
                        behind,
                    },
                }
            }
        },
        checkout::Checkout::Detached(_) => Checkout::Detached,
        checkout::Checkout::Unborn(branch) => Checkout::Unborn { branch },
    };
    let mut remote_branches = Vec::new();
    let mut without = Vec::new();
    for branch in branches::list(repo)? {
        cancel.check()?;
        let reason = match branch.tracking {
            branches::Tracking::Absent => UpstreamReason::NotConfigured,
            branches::Tracking::Local => UpstreamReason::LocalUpstream,
            branches::Tracking::Remote { upstream, .. } => {
                let comparison = match upstream_tip(repo, &upstream)? {
                    Some(tip) => {
                        let (ahead, behind) = repo.graph_ahead_behind(branch.oid, tip)?;
                        RemoteComparison::Compared { ahead, behind }
                    }
                    None => RemoteComparison::MissingUpstream,
                };
                remote_branches.push(RemoteBranch {
                    name: branch.name,
                    comparison,
                });
                continue;
            }
        };
        without.push(BranchWithoutUpstream {
            branch: branch.name,
            reason,
        });
    }
    Ok((
        Synchronization::Current {
            checkout,
            remote_branches: Some(remote_branches),
        },
        without,
    ))
}

pub(super) fn upstream_tip(repo: &Repository, reference: &str) -> Result<Option<Oid>> {
    match branches::resolve(repo, reference) {
        Ok(oid) => Ok(Some(oid)),
        Err(error)
            if error
                .downcast_ref::<Error>()
                .is_some_and(|error| error.code() == ErrorCode::NotFound) =>
        {
            Ok(None)
        }
        Err(error) => Err(error),
    }
}
