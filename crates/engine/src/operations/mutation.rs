//! Small reviewed operations compose native local mutations and Git transport.
use super::{pull, sync};
use crate::{
    cancel::Cancel,
    local::{branches::edit, stash},
    transport::Git,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone)]
pub enum Request {
    Branch { action: edit::Action, name: String },
    Pull,
    Sync,
    Stash(stash::Request),
}
#[derive(Debug, Clone)]
pub enum Plan {
    Branch(edit::Plan),
    Pull(pull::Plan),
    Sync(sync::Plan),
    Stash(stash::Plan),
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Action {
    PrepareSync,
    CreateBranch {
        name: String,
    },
    SwitchBranch {
        name: String,
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
    SynchronizationPrepared,
    CreatedBranch { name: String, oid: String },
    SwitchedBranch { name: String, oid: String },
    Fetched { remotes: Vec<String> },
    FetchSkipped { reason: super::SkipReason },
    Pulled { branch: String, oid: String },
    Synchronized { result: sync::Change },
    SavedStash { oid: String },
    AppliedStash { oid: String },
    PoppedStash { oid: String },
    DroppedStash { oid: String },
}
impl Plan {
    pub fn path(&self) -> &Path {
        match self {
            Self::Branch(p) => p.path(),
            Self::Pull(p) => &p.path,
            Self::Sync(p) => p.path(),
            Self::Stash(p) => p.path(),
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
            Self::Stash(stash::Plan::Save { mode, message, .. }) => Action::SaveStash {
                mode: *mode,
                message: message.clone(),
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
        Plan::Stash(p) => {
            let oid = stash::execute(p, cancel)?.to_string();
            Ok(match p {
                stash::Plan::Save { .. } => Change::SavedStash { oid },
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
