//! Capture the current checkout after remote updates and publications have completed.
use crate::{
    domain::synchronization::{Checkout, Synchronization},
    local::branches::{
        checkout,
        upstream::{Comparison, Tracking},
    },
};
use anyhow::Result;
use git2::Repository;

pub(super) fn read(repo: &Repository) -> Result<Synchronization> {
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
    Ok(Synchronization::Current { checkout })
}
