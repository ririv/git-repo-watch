//! Single-repository inspection; scheduling remains outside this module.
mod synchronization;
use crate::{
    cancel::Cancel,
    domain::*,
    local::{
        self,
        branches::{self, Tracking},
    },
    operations::push,
    transport::Git,
};
use anyhow::{Context, Result, ensure};
use std::{collections::BTreeMap, path::Path, time::Instant};
pub fn repository(
    git: &Git,
    path: &Path,
    policy: PushPolicy,
    cancel: &Cancel,
) -> (RepositoryResult, Timing) {
    inspect(git, path, policy, Refresh::Fetch, cancel)
}
/// Inspect current Git references without network requests or automatic publication.
pub fn local_repository(path: &Path, cancel: &Cancel) -> (RepositoryResult, Timing) {
    inspect(
        &Git::default(),
        path,
        PushPolicy::Disabled,
        Refresh::Local,
        cancel,
    )
}
pub(crate) fn publish_fetched(
    git: &Git,
    path: &Path,
    cancel: &Cancel,
) -> (RepositoryResult, Timing) {
    inspect(git, path, PushPolicy::FastForward, Refresh::Local, cancel)
}
#[derive(Clone, Copy)]
enum Refresh {
    Fetch,
    Local,
}
fn inspect(
    git: &Git,
    path: &Path,
    policy: PushPolicy,
    refresh: Refresh,
    cancel: &Cancel,
) -> (RepositoryResult, Timing) {
    let started = Instant::now();
    let mut timing = Timing {
        path: path.to_owned(),
        elapsed_ms: 0,
        fetch_ms: 0,
        local_ms: 0,
        verify_ms: 0,
        push_ms: 0,
        steps: vec![],
    };
    let mut pushes = Vec::new();
    let attempt = (|| -> Result<RepositoryResult> {
        cancel.check()?;
        let t = Instant::now();
        let repo = local::open(path)?;
        let remotes = repo
            .remotes()?
            .iter()
            .map(|name| {
                let name = name?.context(i18n::tr!(RemoteNameIsNotUtf8))?;
                branches::fetch::Remote::read(&repo, name)
            })
            .collect::<Result<Vec<_>>>()?;
        branches::fetch::validate_destinations(&remotes)?;
        for branch in branches::list(&repo)? {
            if let Tracking::Remote { remote, .. } = branch.tracking {
                let plan = remotes
                    .iter()
                    .find(|plan| plan.name == remote)
                    .context(i18n::tr!(RemoteQueryStateIsMissing))?;
                ensure!(
                    !plan.skipped(&repo)?,
                    i18n::tf!(UpstreamRemoteFetchSkipped, remote)
                );
            }
        }
        timing.local_ms += t.elapsed().as_millis();
        let t = Instant::now();
        let fetched = (|| -> Result<()> {
            for remote in &remotes {
                if matches!(refresh, Refresh::Fetch) && !remote.skipped(&repo)? {
                    git.fetch_tracking(path, remote, cancel)?;
                }
            }
            Ok(())
        })();
        timing.fetch_ms = t.elapsed().as_millis();
        fetched?;
        let t = Instant::now();
        let repo = local::open(path)?;
        let mut issues = Vec::new();
        let mut without = Vec::new();
        if local::files::dirty(&repo)? {
            issues.push(Issue {
                kind: IssueKind::Dirty,
                detail: i18n::tr!(StagedUnstagedUntrackedFilesOrSubmoduleChanges).into(),
            })
        }
        if repo.head_detached()? {
            issues.push(Issue {
                kind: IssueKind::DetachedHead,
                detail: i18n::tr!(HeadIsDetached).into(),
            })
        }
        let branches = branches::list(&repo)?;
        if branches.is_empty() {
            issues.push(Issue {
                kind: IssueKind::NoCommits,
                detail: i18n::tr!(NoLocalBranchCommitsToCheck).into(),
            })
        }
        timing.local_ms += t.elapsed().as_millis();
        // 固定 fetch 后的本地快照。普通检查不再发起 ls-remote；推送另行核验服务器。
        let snapshot_started = Instant::now();
        let mut heads = BTreeMap::new();
        for branch in &branches {
            if let Tracking::Remote {
                remote,
                remote_ref,
                upstream,
            } = &branch.tracking
            {
                let plan = remotes
                    .iter()
                    .find(|plan| &plan.name == remote)
                    .context(i18n::tr!(RemoteQueryStateIsMissing))?;
                ensure!(
                    !plan.skipped(&repo)? && plan.upstream(remote_ref)? == *upstream,
                    i18n::tr!(RemoteUpstreamReferenceIsInconsistentCheckFetch)
                );
                let oid = match branches::resolve(&repo, upstream) {
                    Ok(oid) => Some(oid),
                    Err(error)
                        if error
                            .downcast_ref::<git2::Error>()
                            .is_some_and(|error| error.code() == git2::ErrorCode::NotFound) =>
                    {
                        None
                    }
                    Err(error) => return Err(error),
                };
                heads.insert(upstream.clone(), oid);
            }
        }
        timing.local_ms += snapshot_started.elapsed().as_millis();
        for branch in branches {
            cancel.check()?;
            match branch.tracking {
                Tracking::Absent => without.push(BranchWithoutUpstream {
                    branch: branch.name,
                    reason: UpstreamReason::NotConfigured,
                }),
                Tracking::Local => without.push(BranchWithoutUpstream {
                    branch: branch.name,
                    reason: UpstreamReason::LocalUpstream,
                }),
                Tracking::Remote {
                    remote,
                    remote_ref,
                    upstream,
                } => {
                    let Some(server) = heads.get(&upstream).copied().flatten() else {
                        issues.push(Issue {
                            kind: IssueKind::MissingUpstream,
                            detail: i18n::tf!(RemoteUpstreamBranchDoesNotExist, branch.name),
                        });
                        continue;
                    };
                    let t = Instant::now();
                    let (ahead, behind) = repo.graph_ahead_behind(branch.oid, server)?;
                    timing.local_ms += t.elapsed().as_millis();
                    if ahead > 0 && behind == 0 && policy == PushPolicy::FastForward {
                        let target = PushTarget {
                            branch: branch.name.clone(),
                            local_ref: branch.reference.clone(),
                            local_oid: branch.oid.to_string(),
                            remote: remote.clone(),
                            remote_ref: remote_ref.clone(),
                            remote_oid: server.to_string(),
                            ahead_commits: ahead,
                        };
                        let t = Instant::now();
                        let result = push::forward(git, path, &target, cancel);
                        timing.push_ms += t.elapsed().as_millis();
                        let succeeded = matches!(result, Publication::Pushed { .. });
                        let error = result.error().map(str::to_owned);
                        pushes.push(result);
                        if succeeded {
                            heads.insert(upstream, Some(branch.oid));
                            let current = branches::resolve(&repo, &branch.reference)?;
                            if current != branch.oid {
                                issues.push(Issue {
                                    kind: IssueKind::OutOfSync,
                                    detail: i18n::tf!(
                                        LocalBranchChangedAfterPushCheckAgain,
                                        branch.name
                                    ),
                                })
                            }
                            continue;
                        }
                        issues.push(Issue {
                            kind: IssueKind::PushFailed,
                            detail: i18n::tf!(
                                AutomaticPushFailed,
                                branch.name,
                                error.context(i18n::tr!(FailedPushHasNoError))?
                            ),
                        });
                    }
                    if ahead > 0 || behind > 0 {
                        issues.push(Issue {
                            kind: IssueKind::OutOfSync,
                            detail: i18n::tf!(
                                AheadOfRemoteByCommitsBehindBy,
                                branch.name,
                                ahead,
                                behind
                            ),
                        })
                    }
                }
            }
        }
        let t = Instant::now();
        let synchronization = synchronization::read(&repo)?;
        timing.local_ms += t.elapsed().as_millis();
        let mut issues = issues.into_iter();
        Ok(match issues.next() {
            None => RepositoryResult::Clean {
                path: path.to_owned(),
                synchronization,
                branches_without_upstream: without,
                pushes: pushes.clone(),
            },
            Some(first) => RepositoryResult::Attention {
                path: path.to_owned(),
                synchronization,
                issues: Issues::new(first, issues.collect()),
                branches_without_upstream: without,
                pushes: pushes.clone(),
            },
        })
    })();
    timing.elapsed_ms = started.elapsed().as_millis();
    let result = match attempt {
        Ok(r) => r,
        Err(e) => RepositoryResult::Failed {
            path: path.to_owned(),
            error: format!("{e:#}"),
            pushes,
        },
    };
    (result, timing)
}
