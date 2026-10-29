//! Frozen push identity and explicit remote-update states.
use crate::{
    domain::{PushTarget, RemoteTip},
    local::{self, Head},
};
use anyhow::{Result, ensure};
use git2::{Oid, Repository};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Upstream {
    Absent,
    Configured { remote: String, reference: String },
}
impl Upstream {
    pub(super) fn read(repo: &Repository, branch: &str) -> Result<Self> {
        let config = repo.config()?;
        let read = |suffix| -> Result<Option<String>> {
            match config.get_string(&format!("branch.{branch}.{suffix}")) {
                Ok(value) => Ok(Some(value)),
                Err(error) if error.code() == git2::ErrorCode::NotFound => Ok(None),
                Err(error) => Err(error.into()),
            }
        };
        match (read("remote")?, read("merge")?) {
            (None, None) => Ok(Self::Absent),
            (Some(remote), Some(reference)) => Ok(Self::Configured { remote, reference }),
            _ => anyhow::bail!(i18n::tf!(IncompleteBranchUpstream, branch)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Remote {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct TargetRequest {
    pub path: PathBuf,
    pub branch: String,
    pub(super) oid: Oid,
    pub(super) first: Remote,
    pub(super) rest: Vec<Remote>,
}
impl TargetRequest {
    pub fn remotes(&self) -> impl Iterator<Item = &Remote> {
        std::slice::from_ref(&self.first)
            .iter()
            .chain(self.rest.iter())
    }
    pub(super) fn validate(&self, repo: &Repository) -> Result<()> {
        ensure!(
            local::head(repo)?
                == Head::Existing {
                    branch: self.branch.clone(),
                    oid: self.oid
                },
            i18n::tr!(MutationChanged)
        );
        ensure!(
            Upstream::read(repo, &self.branch)? == Upstream::Absent,
            i18n::tr!(MutationChanged)
        );
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub(super) enum Update {
    Create {
        commits: usize,
    },
    Advance {
        expected: Oid,
        commits: usize,
    },
    Overwrite {
        expected: Oid,
        ahead: usize,
        behind: usize,
    },
    Track {
        expected: Oid,
    },
}
impl Update {
    pub(super) fn expected(&self) -> Option<Oid> {
        match self {
            Self::Create { .. } => None,
            Self::Advance { expected, .. }
            | Self::Overwrite { expected, .. }
            | Self::Track { expected } => Some(*expected),
        }
    }
    fn ahead(&self) -> usize {
        match self {
            Self::Create { commits } | Self::Advance { commits, .. } => *commits,
            Self::Overwrite { ahead, .. } => *ahead,
            Self::Track { .. } => 0,
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct Identity {
    pub branch: String,
    pub oid: Oid,
    pub remote: String,
    pub reference: String,
    pub url: String,
    pub upstream: Upstream,
}
impl Identity {
    pub(super) fn validate(&self, repo: &Repository) -> Result<()> {
        local::idle(repo)?;
        ensure!(
            self.reference.starts_with("refs/heads/")
                && git2::Reference::is_valid_name(&self.reference),
            i18n::tr!(InvalidPushTarget)
        );
        ensure!(
            local::head(repo)?
                == Head::Existing {
                    branch: self.branch.clone(),
                    oid: self.oid
                },
            i18n::tr!(BranchOrHeadChangedReviewThePush)
        );
        ensure!(
            Upstream::read(repo, &self.branch)? == self.upstream,
            i18n::tr!(MutationChanged)
        );
        ensure!(
            local::branches::url(repo, &self.remote)? == self.url,
            i18n::tr!(RemoteUrlChangedReviewThePushAgain)
        );
        Ok(())
    }
}
#[derive(Debug, Clone)]
pub struct PushPlan {
    pub path: PathBuf,
    pub(super) identity: Identity,
    pub(super) update: Update,
}
impl PushPlan {
    pub fn url(&self) -> &str {
        &self.identity.url
    }
    pub fn target(&self) -> PushTarget {
        PushTarget {
            branch: self.identity.branch.clone(),
            local_ref: format!("refs/heads/{}", self.identity.branch),
            local_oid: self.identity.oid.to_string(),
            remote: self.identity.remote.clone(),
            remote_ref: self.identity.reference.clone(),
            remote_oid: match self.update.expected() {
                Some(oid) => RemoteTip::Existing(oid.to_string()),
                None => RemoteTip::Absent,
            },
            ahead_commits: self.update.ahead(),
        }
    }
    pub fn force_behind(&self) -> Option<usize> {
        match self.update {
            Update::Overwrite { behind, .. } => Some(behind),
            _ => None,
        }
    }
    pub fn needs_confirmation(&self) -> bool {
        self.force_behind().is_some()
            || matches!(
                (&self.update, &self.identity.upstream),
                (Update::Create { .. }, Upstream::Configured { .. })
            )
    }
    pub(super) fn validate(&self, repo: &Repository) -> Result<()> {
        self.identity.validate(repo)
    }
}

#[derive(Debug, Clone)]
pub enum Preparation {
    Ready(PushPlan),
    Target(TargetRequest),
    Current,
}
