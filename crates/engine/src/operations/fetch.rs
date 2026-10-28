//! Refresh configured remote-tracking references without checking out files.
use crate::{cancel::Cancel, local, transport::Git};
use anyhow::{Context, Result, ensure};
use std::path::Path;

pub fn all(git: &Git, path: &Path, cancel: &Cancel) -> Result<Vec<String>> {
    let repo = local::open(path)?;
    let mut remotes = Vec::new();
    for name in repo.remotes()?.iter() {
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
    Ok(remotes.into_iter().map(|remote| remote.name).collect())
}
