//! Attached, detached, unborn, and tracking states for a local checkout.
use crate::local::branches::upstream;
pub use crate::local::branches::upstream::{Comparison, Tracking};
use anyhow::{Context, Result};
use git2::{ErrorCode, Oid, Repository};

#[derive(Debug)]
pub enum Checkout {
    Branch {
        name: String,
        oid: Oid,
        tracking: std::result::Result<Tracking, String>,
    },
    Detached(Oid),
    Unborn(String),
}

pub(crate) fn read(repo: &Repository) -> Result<Checkout> {
    match repo.head() {
        Ok(head) => {
            let oid = head.peel_to_commit()?.id();
            if repo.head_detached()? {
                return Ok(Checkout::Detached(oid));
            }
            let name = head
                .shorthand()
                .context(i18n::tr!(BranchNameIsNotUtf8))?
                .to_owned();
            let tracking = upstream::read(repo, &name, oid).map_err(|error| format!("{error:#}"));
            Ok(Checkout::Branch {
                name,
                oid,
                tracking,
            })
        }
        Err(error) if error.code() == ErrorCode::UnbornBranch => {
            let head = repo.find_reference("HEAD")?;
            let name = head
                .symbolic_target()?
                .context(i18n::tr!(HeadHasNoSymbolicBranch))?
                .strip_prefix("refs/heads/")
                .context(i18n::tr!(HeadIsNotAttachedToALocal))?
                .to_owned();
            Ok(Checkout::Unborn(name))
        }
        Err(error) => Err(error.into()),
    }
}
