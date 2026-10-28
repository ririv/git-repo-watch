//! Refresh configured remote-tracking references without checking out files.
use super::SkipReason;
use crate::{cancel::Cancel, local, transport::Git};
use anyhow::{Context, Result, ensure};
use std::path::Path;

#[derive(Debug)]
pub enum Outcome {
    Fetched { remotes: Vec<String> },
    Skipped { reason: SkipReason },
}
pub fn all(git: &Git, path: &Path, cancel: &Cancel) -> Result<Outcome> {
    cancel.check()?;
    let repo = local::open(path)?;
    let configured = repo.remotes()?;
    if configured.is_empty() {
        return Ok(Outcome::Skipped {
            reason: SkipReason::NoRemotes,
        });
    }
    let mut remotes = Vec::new();
    for name in configured.iter() {
        let remote = local::branches::fetch::Remote::read(
            &repo,
            name?.context(i18n::tr!(MutationRemoteNotUtf8))?,
        )?;
        if remote.skipped(&repo)? {
            continue;
        }
        ensure!(
            !remote.arguments().is_empty(),
            i18n::tr!(MutationFetchMappingRequired)
        );
        remotes.push(remote);
    }
    ensure!(!remotes.is_empty(), i18n::tr!(MutationNoRemote));
    local::branches::fetch::validate_destinations(&remotes)?;
    for remote in &remotes {
        git.fetch_tracking(path, remote, cancel)?;
    }
    Ok(Outcome::Fetched {
        remotes: remotes.into_iter().map(|remote| remote.name).collect(),
    })
}
