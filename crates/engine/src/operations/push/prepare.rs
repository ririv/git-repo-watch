//! Separate interactive publication from non-forced automated workflows.
use super::{
    Preparation, PushPlan, TargetRequest,
    plan::{Identity, Remote, Update, Upstream},
};
use crate::{
    cancel::Cancel,
    local::{self, Head, branches},
    transport::Git,
};
use anyhow::{Context, Result, ensure};
use git2::{ErrorCode, Repository};
use std::path::Path;

pub(super) fn existing(
    git: &Git,
    path: &Path,
    fetched: &[String],
    cancel: &Cancel,
) -> Result<PushPlan> {
    let plan = configured(git, path, fetched, cancel)?;
    match plan.update {
        Update::Advance { .. } => Ok(plan),
        Update::Overwrite { ahead, behind, .. } => anyhow::bail!(i18n::tf!(
            CurrentBranchIsAheadCommitsAheadAnd,
            ahead = ahead,
            behind = behind
        )),
        Update::Track { .. } => anyhow::bail!(i18n::tr!(CurrentBranchMatchesTheRemoteNothingTo)),
        Update::Create { .. } => anyhow::bail!(i18n::tr!(RemoteUpstreamBranchDoesNotExistAlt)),
    }
}

pub(super) fn interactive(git: &Git, path: &Path, cancel: &Cancel) -> Result<Preparation> {
    let repo = local::open(path)?;
    local::idle(&repo)?;
    let (branch, oid) = attached(&repo)?;
    match Upstream::read(&repo, &branch)? {
        Upstream::Configured { .. } => {
            drop(repo);
            let plan = configured(git, path, &[], cancel)?;
            if matches!(plan.update, Update::Track { .. }) {
                Ok(Preparation::Current)
            } else {
                Ok(Preparation::Ready(plan))
            }
        }
        Upstream::Absent => {
            let mut remotes = repo
                .remotes()?
                .iter()
                .map(|name| {
                    let name = name?.context(i18n::tr!(RemoteUrlIsNotUtf8))?;
                    Ok(Remote {
                        name: name.into(),
                        url: branches::url(&repo, name)?,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            remotes.sort_by(|a, b| a.name.cmp(&b.name));
            ensure!(!remotes.is_empty(), i18n::tr!(SyncNoRemotes));
            let request = TargetRequest {
                path: path.into(),
                branch,
                oid,
                first: remotes.remove(0),
                rest: remotes,
            };
            let automatic = auto_remote(&repo, &request)?;
            drop(repo);
            match automatic {
                Some(remote) => {
                    target(git, &request, &remote, &request.branch, cancel).map(Preparation::Ready)
                }
                None => Ok(Preparation::Target(request)),
            }
        }
    }
}

fn configured(git: &Git, path: &Path, fetched: &[String], cancel: &Cancel) -> Result<PushPlan> {
    let repo = local::open(path)?;
    local::idle(&repo)?;
    let (branch, oid) = attached(&repo)?;
    let upstream = Upstream::read(&repo, &branch)?;
    let Upstream::Configured { remote, reference } = &upstream else {
        anyhow::bail!(i18n::tr!(CurrentBranchHasNoRemoteUpstreamExplicitly))
    };
    ensure!(remote != ".", i18n::tr!(SyncLocalUpstream));
    let identity = Identity {
        url: branches::url(&repo, remote)?,
        branch,
        oid,
        remote: remote.clone(),
        reference: reference.clone(),
        upstream,
    };
    let update = inspect(git, &repo, path, &identity, fetched, cancel)?;
    Ok(PushPlan {
        path: path.into(),
        identity,
        update,
    })
}

pub(super) fn target(
    git: &Git,
    request: &TargetRequest,
    remote: &str,
    branch: &str,
    cancel: &Cancel,
) -> Result<PushPlan> {
    let repo = local::open(&request.path)?;
    local::idle(&repo)?;
    request.validate(&repo)?;
    let destination = request
        .remotes()
        .find(|item| item.name == remote)
        .context(i18n::tr!(PushRemoteRequired))?;
    ensure!(
        branches::url(&repo, remote)? == destination.url,
        i18n::tr!(MutationChanged)
    );
    let reference = format!("refs/heads/{branch}");
    ensure!(
        !branch.is_empty()
            && !branch.starts_with('-')
            && git2::Reference::is_valid_name(&reference),
        i18n::tr!(PushInvalidBranch)
    );
    let identity = Identity {
        url: destination.url.clone(),
        branch: request.branch.clone(),
        oid: request.oid,
        remote: remote.into(),
        reference,
        upstream: Upstream::Absent,
    };
    let update = inspect(git, &repo, &request.path, &identity, &[], cancel)?;
    Ok(PushPlan {
        path: request.path.clone(),
        identity,
        update,
    })
}

fn inspect(
    git: &Git,
    repo: &Repository,
    path: &Path,
    identity: &Identity,
    fetched: &[String],
    cancel: &Cancel,
) -> Result<Update> {
    cancel.check()?;
    identity.validate(repo)?;
    let mapping = branches::fetch::Remote::read(repo, &identity.remote)?;
    let tracking = mapping.upstream(&identity.reference)?;
    if !fetched.contains(&identity.remote) {
        git.fetch_tracking(path, &mapping, cancel)?;
    }
    let server = git.branch_tip(path, &identity.remote, &identity.reference, cancel)?;
    let update = match server {
        None => {
            let mut walk = repo.revwalk()?;
            walk.push(identity.oid)?;
            let commits = walk.try_fold(0, |count, oid| oid.map(|_| count + 1))?;
            Update::Create { commits }
        }
        Some(expected) => {
            ensure!(
                branches::resolve(repo, &tracking)? == expected,
                i18n::tr!(RemoteUpstreamReferenceIsInconsistentCheckFetch)
            );
            let (ahead, behind) = repo.graph_ahead_behind(identity.oid, expected)?;
            match (ahead, behind) {
                (0, 0) => Update::Track { expected },
                (_, 0) => Update::Advance {
                    expected,
                    commits: ahead,
                },
                _ => Update::Overwrite {
                    expected,
                    ahead,
                    behind,
                },
            }
        }
    };
    identity.validate(repo)?;
    Ok(update)
}

fn attached(repo: &Repository) -> Result<(String, git2::Oid)> {
    match local::head(repo)? {
        Head::Existing { branch, oid } => Ok((branch, oid)),
        Head::Initial { .. } => anyhow::bail!(i18n::tr!(CurrentBranchHasNoCommits)),
    }
}

fn string(config: &git2::Config, key: &str) -> Result<Option<String>> {
    match config.get_string(key) {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.code() == ErrorCode::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}
fn auto_remote(repo: &Repository, request: &TargetRequest) -> Result<Option<String>> {
    let config = repo.config()?;
    let enabled = match config.get_bool("push.autoSetupRemote") {
        Ok(value) => value,
        Err(error) if error.code() == ErrorCode::NotFound => false,
        Err(error) => return Err(error.into()),
    };
    if !enabled
        || !matches!(
            string(&config, "push.default")?.as_deref(),
            None | Some("simple" | "current" | "upstream")
        )
    {
        return Ok(None);
    }
    let configured = match string(&config, &format!("branch.{}.pushRemote", request.branch))? {
        Some(name) => Some(name),
        None => string(&config, "remote.pushDefault")?,
    };
    let selected = if let Some(name) = configured {
        ensure!(
            request.remotes().any(|remote| remote.name == name),
            i18n::tr!(PushRemoteRequired)
        );
        Some(name)
    } else if request.rest.is_empty() {
        Some(request.first.name.clone())
    } else {
        request
            .remotes()
            .find(|remote| remote.name == "origin")
            .map(|remote| remote.name.clone())
    };
    if let Some(remote) = &selected {
        let mirror = match config.get_bool(&format!("remote.{remote}.mirror")) {
            Ok(value) => value,
            Err(error) if error.code() == ErrorCode::NotFound => false,
            Err(error) => return Err(error.into()),
        };
        if mirror || string(&config, &format!("remote.{remote}.push"))?.is_some() {
            // 自定义推送规则不等于同名分支发布，必须由用户明确选择目标。
            return Ok(None);
        }
    }
    Ok(selected)
}
