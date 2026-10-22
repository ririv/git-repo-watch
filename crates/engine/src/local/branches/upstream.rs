//! Explicit local and remote upstream identity with local reference comparison.
use super::fetch;
use anyhow::Result;
use git2::{ErrorCode, Oid, Repository};

#[derive(Debug, Clone)]
pub enum Tracking {
    Absent,
    Local {
        name: String,
        comparison: Comparison,
    },
    Remote {
        name: String,
        comparison: Comparison,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Comparison {
    Missing,
    Known { ahead: usize, behind: usize },
}

fn configured(repo: &Repository, key: &str) -> Result<Option<String>> {
    match repo.config()?.get_string(key) {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.code() == ErrorCode::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

pub fn read(repo: &Repository, name: &str, oid: Oid) -> Result<Tracking> {
    let (remote, merge) = match (
        configured(repo, &format!("branch.{name}.remote"))?,
        configured(repo, &format!("branch.{name}.merge"))?,
    ) {
        (None, None) => return Ok(Tracking::Absent),
        (Some(remote), Some(merge)) => (remote, merge),
        _ => anyhow::bail!(i18n::tf!(IncompleteBranchUpstream, name)),
    };
    let reference = if remote == "." {
        merge
    } else {
        fetch::Remote::read(repo, &remote)?.upstream(&merge)?
    };
    let comparison = match repo.find_reference(&reference) {
        Ok(reference) => {
            let upstream = reference.peel_to_commit()?.id();
            let (ahead, behind) = repo.graph_ahead_behind(oid, upstream)?;
            Comparison::Known { ahead, behind }
        }
        Err(error) if error.code() == ErrorCode::NotFound => Comparison::Missing,
        Err(error) => return Err(error.into()),
    };
    let name = reference
        .strip_prefix("refs/remotes/")
        .or_else(|| reference.strip_prefix("refs/heads/"))
        .unwrap_or(&reference)
        .to_owned();
    Ok(if remote == "." {
        Tracking::Local { name, comparison }
    } else {
        Tracking::Remote { name, comparison }
    })
}
