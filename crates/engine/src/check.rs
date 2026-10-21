//! Single-repository inspection; scheduling remains outside this module.
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
    let started = Instant::now();
    let mut timing = Timing {
        path: path.to_owned(),
        elapsed_ms: 0,
        fetch_ms: 0,
        local_ms: 0,
        verify_ms: 0,
        push_ms: 0,
    };
    let mut pushes = Vec::new();
    let attempt = (|| -> Result<RepositoryResult> {
        cancel.check()?;
        let t = Instant::now();
        let _ = local::open(path)?;
        timing.local_ms += t.elapsed().as_millis();
        let t = Instant::now();
        let fetched = git.fetch_all(path, cancel);
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
        let mut heads = BTreeMap::new();
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
                    if !heads.contains_key(&remote) {
                        let t = Instant::now();
                        let advertised = git.heads(path, &remote, cancel);
                        timing.verify_ms += t.elapsed().as_millis();
                        heads.insert(remote.clone(), advertised?);
                    }
                    let Some(server) = heads
                        .get(&remote)
                        .context(i18n::tr!(RemoteQueryStateIsMissing))?
                        .get(&remote_ref)
                        .copied()
                    else {
                        issues.push(Issue {
                            kind: IssueKind::MissingUpstream,
                            detail: i18n::tf!(RemoteUpstreamBranchDoesNotExist, branch.name),
                        });
                        continue;
                    };
                    let t = Instant::now();
                    let tracked = branches::resolve(&repo, &upstream)?;
                    ensure!(
                        tracked == server,
                        i18n::tf!(RemoteChangedAfterFetchOrFetchRefspec, branch.name)
                    );
                    let (ahead, behind) = repo.graph_ahead_behind(branch.oid, tracked)?;
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
                            heads
                                .get_mut(&remote)
                                .context(i18n::tr!(RemoteQueryStateIsMissing))?
                                .insert(remote_ref, branch.oid);
                            if branches::resolve(&repo, &branch.reference)? != branch.oid {
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
        let mut issues = issues.into_iter();
        Ok(match issues.next() {
            None => RepositoryResult::Clean {
                path: path.to_owned(),
                branches_without_upstream: without,
                pushes: pushes.clone(),
            },
            Some(first) => RepositoryResult::Attention {
                path: path.to_owned(),
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
