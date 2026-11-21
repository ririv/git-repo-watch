//! Branch comparison snapshots and explicitly retained legacy report formats.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Comparison {
    Equal,
    Ahead,
    Behind,
    Diverged,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Checkout {
    Compared {
        branch: String,
        ahead: usize,
        behind: usize,
    },
    NoUpstream {
        branch: String,
    },
    MissingUpstream {
        branch: String,
    },
    Detached,
    Unborn {
        branch: String,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteBranch {
    pub name: String,
    pub comparison: RemoteComparison,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum RemoteComparison {
    Compared { ahead: usize, behind: usize },
    MissingUpstream,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Synchronization {
    #[default]
    Unknown,
    /// Legacy branch aggregates do not describe the checked-out branch.
    #[serde(rename = "known")]
    LegacyBranches { branches: Vec<Comparison> },
    Current {
        checkout: Checkout,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        remote_branches: Option<Vec<RemoteBranch>>,
    },
}
impl Synchronization {
    pub fn remote_branches(&self) -> Option<&[RemoteBranch]> {
        match self {
            Self::Current {
                remote_branches, ..
            } => remote_branches.as_deref(),
            Self::Unknown | Self::LegacyBranches { .. } => None,
        }
    }

    pub fn current_commits(&self) -> Option<(usize, usize)> {
        match self {
            Self::Current {
                checkout: Checkout::Compared { ahead, behind, .. },
                ..
            } => Some((*ahead, *behind)),
            Self::Unknown | Self::LegacyBranches { .. } | Self::Current { .. } => None,
        }
    }
}
