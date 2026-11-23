//! Small reviewed operations compose native local mutations and Git transport.
use super::{pull, submodule, sync};
use crate::{
    cancel::Cancel,
    local::{branches::edit, stash, worktrees},
    transport::Git,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone)]
pub enum Request {
    Branch {
        action: edit::Action,
        name: String,
    },
    Pull,
    ReplayControl(super::replay::Control),
    Reset {
        target: git2::Oid,
        mode: super::reset::Mode,
    },
    Sync,
    Stash(stash::Request),
    Worktree(worktrees::Request),
    AddSubmodule(submodule::Add),
}
#[derive(Debug, Clone)]
pub enum Plan {
    Branch(edit::Plan),
    Pull(pull::Plan),
    Replay(super::replay::Plan),
    Reset(super::reset::Plan),
    Sync(sync::Plan),
    Stash(stash::Plan),
    Worktree(worktrees::Plan),
    AddSubmodule(submodule::Plan),
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Action {
    ResetBranch {
        #[serde(default)]
        mode: super::reset::Mode,
        branch: String,
        from: String,
        to: String,
    },
    ResolveConflict {
        file: std::path::PathBuf,
        intent: crate::local::files::conflict::resolve::Intent,
    },
    ReplaySequence {
        commits: Vec<String>,
    },
    PrepareSync,
    Replay {
        kind: super::replay::Kind,
        commit: String,
    },
    ReplayControl {
        kind: super::replay::Kind,
        control: super::replay::Control,
        commit: String,
    },
    CreateBranch {
        name: String,
    },
    SwitchBranch {
        name: String,
    },
    AddWorktree {
        directory: std::path::PathBuf,
    },
    AddSubmodule {
        directory: std::path::PathBuf,
    },
    RemoveWorktree {
        directory: std::path::PathBuf,
    },
    Fetch,
    Pull {
        branch: String,
        upstream: String,
    },
    Sync,
    SaveStash {
        mode: stash::SaveMode,
        message: String,
    },
    ApplyStash {
        oid: String,
    },
    PopStash {
        oid: String,
    },
    DropStash {
        oid: String,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "change", rename_all = "snake_case")]
pub enum Change {
    ResetBranch {
        #[serde(default)]
        mode: super::reset::Mode,
        branch: String,
        from: String,
        to: String,
    },
    ResolvedConflict {
        file: std::path::PathBuf,
        intent: crate::local::files::conflict::resolve::Intent,
    },
    ReplayedSequence {
        commits: Vec<String>,
    },
    SynchronizationPrepared,
    Replayed {
        kind: super::replay::Kind,
        commit: String,
    },
    ReplayControlled {
        kind: super::replay::Kind,
        control: super::replay::Control,
    },
    CreatedBranch {
        name: String,
        oid: String,
    },
    SwitchedBranch {
        name: String,
        oid: String,
    },
    AddedWorktree {
        directory: std::path::PathBuf,
    },
    AddedSubmodule {
        directory: std::path::PathBuf,
    },
    RemovedWorktree {
        directory: std::path::PathBuf,
    },
    Fetched {
        remotes: Vec<String>,
    },
    FetchSkipped {
        reason: super::SkipReason,
    },
    Pulled {
        branch: String,
        oid: String,
    },
    Synchronized {
        result: sync::Change,
    },
    SavedStash {
        oid: String,
    },
    AppliedStash {
        oid: String,
    },
    PoppedStash {
        oid: String,
    },
    DroppedStash {
        oid: String,
    },
}
impl Plan {
    pub fn path(&self) -> &Path {
        match self {
            Self::Branch(p) => p.path(),
            Self::Pull(p) => &p.path,
            Self::Replay(p) => p.path(),
            Self::Reset(p) => p.path(),
            Self::Sync(p) => p.path(),
            Self::Stash(p) => p.path(),
            Self::Worktree(p) => p.repository(),
            Self::AddSubmodule(p) => p.repository(),
        }
    }
    pub fn current(&self) -> bool {
        matches!(self,Self::Pull(p) if matches!(p.update,pull::Update::Current))
    }
    pub fn notice(&self) -> Option<&'static str> {
        match self {
            Self::Pull(_) if self.current() => Some(i18n::tr!(PullCurrent)),
            Self::Sync(plan) => plan.notice(),
            _ => None,
        }
    }
    pub fn action(&self) -> Action {
        match self {
            Self::Branch(edit::Plan::Create { name, .. }) => {
                Action::CreateBranch { name: name.clone() }
            }
            Self::Branch(edit::Plan::Switch { name, .. }) => {
                Action::SwitchBranch { name: name.clone() }
            }
            Self::Pull(p) => Action::Pull {
                branch: p.branch.clone(),
                upstream: p.upstream().into(),
            },
            Self::Sync(_) => Action::Sync,
            Self::Reset(plan) => Action::ResetBranch {
                mode: plan.mode(),
                branch: plan.branch().into(),
                from: plan.before().to_string(),
                to: plan.target().oid.to_string(),
            },
            Self::Replay(p) => match p {
                super::replay::Plan::Batch(batch) => Action::ReplaySequence {
                    commits: batch
                        .steps()
                        .map(|step| step.candidate.commit.to_string())
                        .collect(),
                },
                super::replay::Plan::Start(_) => Action::Replay {
                    kind: p.kind(),
                    commit: p.commit().to_string(),
                },
                super::replay::Plan::Control(c) => Action::ReplayControl {
                    kind: p.kind(),
                    control: c.control,
                    commit: p.commit().to_string(),
                },
            },
            Self::AddSubmodule(plan) => Action::AddSubmodule {
                directory: plan.directory().into(),
            },
            Self::Worktree(plan) => {
                let directory = plan.directory().to_owned();
                if plan.removing() {
                    Action::RemoveWorktree { directory }
                } else {
                    Action::AddWorktree { directory }
                }
            }
            Self::Stash(stash::Plan::Save(plan)) => Action::SaveStash {
                mode: plan.mode.clone(),
                message: plan.message.input().into(),
            },
            Self::Stash(stash::Plan::Restore {
                entry,
                action: stash::Restore::Apply,
                ..
            }) => Action::ApplyStash {
                oid: entry.oid.to_string(),
            },
            Self::Stash(stash::Plan::Restore {
                entry,
                action: stash::Restore::Pop,
                ..
            }) => Action::PopStash {
                oid: entry.oid.to_string(),
            },
            Self::Stash(stash::Plan::Drop { entry, .. }) => Action::DropStash {
                oid: entry.oid.to_string(),
            },
        }
    }
}
pub fn execute(git: &Git, plan: &Plan, cancel: &Cancel) -> Result<Change> {
    match plan {
        Plan::Branch(p) => {
            edit::execute(p, cancel)?;
            Ok(match p {
                edit::Plan::Create { name, oid, .. } => Change::CreatedBranch {
                    name: name.clone(),
                    oid: oid.to_string(),
                },
                edit::Plan::Switch {
                    path,
                    name,
                    oid,
                    guard,
                } => {
                    let before = match guard.position {
                        crate::local::guard::Position::Attached { oid, .. }
                        | crate::local::guard::Position::Detached(oid) => oid,
                        crate::local::guard::Position::Unborn(_) => git2::Oid::ZERO_SHA1,
                    };
                    git.post_checkout(path, before, *oid, cancel)?;
                    Change::SwitchedBranch {
                        name: name.clone(),
                        oid: oid.to_string(),
                    }
                }
            })
        }
        Plan::Pull(p) => {
            pull::execute(git, p, cancel)?;
            Ok(Change::Pulled {
                branch: p.branch.clone(),
                oid: p.oid.to_string(),
            })
        }
        Plan::Sync(plan) => sync::execute(git, plan, cancel)
            .into_result()
            .map(|change| Change::Synchronized { result: change }),
        Plan::Reset(plan) => {
            super::reset::execute(git, plan, cancel)?;
            Ok(Change::ResetBranch {
                mode: plan.mode(),
                branch: plan.branch().into(),
                from: plan.before().to_string(),
                to: plan.target().oid.to_string(),
            })
        }
        Plan::Replay(p) => {
            super::replay::execute(git, p, cancel)?;
            Ok(match p {
                super::replay::Plan::Batch(batch) => Change::ReplayedSequence {
                    commits: batch
                        .steps()
                        .map(|step| step.candidate.commit.to_string())
                        .collect(),
                },
                super::replay::Plan::Start(_) => Change::Replayed {
                    kind: p.kind(),
                    commit: p.commit().to_string(),
                },
                super::replay::Plan::Control(c) => Change::ReplayControlled {
                    kind: p.kind(),
                    control: c.control,
                },
            })
        }
        Plan::AddSubmodule(plan) => {
            submodule::execute(git, plan, cancel)?;
            Ok(Change::AddedSubmodule {
                directory: plan.directory().into(),
            })
        }
        Plan::Worktree(plan) => {
            worktrees::execute(git, plan, cancel)?;
            let directory = plan.directory().to_owned();
            Ok(if plan.removing() {
                Change::RemovedWorktree { directory }
            } else {
                Change::AddedWorktree { directory }
            })
        }
        Plan::Stash(p) => {
            let oid = stash::execute(git, p, cancel)?.to_string();
            Ok(match p {
                stash::Plan::Save(_) => Change::SavedStash { oid },
                stash::Plan::Restore {
                    action: stash::Restore::Apply,
                    ..
                } => Change::AppliedStash { oid },
                stash::Plan::Restore {
                    action: stash::Restore::Pop,
                    ..
                } => Change::PoppedStash { oid },
                stash::Plan::Drop { .. } => Change::DroppedStash { oid },
            })
        }
    }
}
