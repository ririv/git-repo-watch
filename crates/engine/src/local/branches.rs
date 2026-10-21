//! Typed branch tracking and local graph queries.
use super::utf8_path;
use anyhow::{Context, Result, ensure};
use git2::{BranchType, Oid, Repository};
use std::path::Path;
#[derive(Debug, Clone)]
pub enum Tracking {
    Absent,
    Local,
    Remote {
        remote: String,
        remote_ref: String,
        upstream: String,
    },
}
#[derive(Debug, Clone)]
pub struct Branch {
    pub name: String,
    pub reference: String,
    pub oid: Oid,
    pub tracking: Tracking,
}
pub fn list(repo: &Repository) -> Result<Vec<Branch>> {
    let mut result = Vec::new();
    for entry in repo.branches(Some(BranchType::Local))? {
        let (branch, _) = entry?;
        let reference = branch
            .get()
            .name()
            .context(i18n::tr!(BranchReferenceIsNotUtf8))?
            .to_owned();
        let name = branch
            .name()?
            .context(i18n::tr!(BranchNameIsNotUtf8))?
            .to_owned();
        let oid = branch
            .get()
            .target()
            .context(i18n::tr!(BranchHasNoCommit))?;
        let tracking = match repo.branch_upstream_remote(&reference) {
            Ok(remote) => {
                let remote = remote
                    .as_str()
                    .context(i18n::tr!(RemoteNameIsNotUtf8))?
                    .to_owned();
                if remote == "." {
                    Tracking::Local
                } else {
                    let config = repo.config()?;
                    let remote_ref = config.get_string(&format!("branch.{name}.merge"))?;
                    let upstream = repo
                        .branch_upstream_name(&reference)?
                        .as_str()
                        .context(i18n::tr!(UpstreamReferenceIsNotUtf8))?
                        .to_owned();
                    Tracking::Remote {
                        remote,
                        remote_ref,
                        upstream,
                    }
                }
            }
            Err(e) if e.code() == git2::ErrorCode::NotFound => Tracking::Absent,
            Err(e) => return Err(e.into()),
        };
        result.push(Branch {
            name,
            reference,
            oid,
            tracking,
        });
    }
    result.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(result)
}
pub fn resolve(repo: &Repository, reference: &str) -> Result<Oid> {
    repo.find_reference(reference)?
        .resolve()?
        .target()
        .context(i18n::tr!(ReferenceHasNoObjectId))
}
pub fn url(repo: &Repository, name: &str) -> Result<String> {
    let config = repo.config()?;
    let mut fetch = Vec::new();
    let mut entries = config.multivar(&format!("remote.{name}.url"), None)?;
    while let Some(e) = entries.next() {
        fetch.push(
            e?.value()
                .context(i18n::tr!(RemoteUrlIsNotUtf8))?
                .to_owned(),
        );
    }
    ensure!(
        fetch.len() == 1,
        i18n::tr!(RemoteMustHaveExactlyOneFetchUrl)
    );
    let mut push = Vec::new();
    match config.multivar(&format!("remote.{name}.pushurl"), None) {
        Ok(mut entries) => {
            while let Some(e) = entries.next() {
                push.push(e?.value().context(i18n::tr!(PushUrlIsNotUtf8))?.to_owned())
            }
        }
        Err(e) if e.code() == git2::ErrorCode::NotFound => {}
        Err(e) => return Err(e.into()),
    };
    ensure!(push.len() <= 1, i18n::tr!(PushUrlDiffersFromFetchUrlOr));
    let remote = repo.find_remote(name)?;
    let resolved = remote.url().context(i18n::tr!(RemoteUrlIsNotUtf8))?;
    ensure!(
        remote.pushurl()?.is_none_or(|u| u == resolved),
        i18n::tr!(PushUrlDiffersFromTheCheckedUrl)
    );
    Ok(resolved.to_owned())
}
pub fn is_root(path: &Path) -> Result<()> {
    let _ = super::open(Path::new(utf8_path(path)?))?;
    Ok(())
}
