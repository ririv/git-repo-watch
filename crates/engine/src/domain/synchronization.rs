//! Checked-out branch commit differences and explicitly retained legacy aggregates.
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
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Synchronization {
    #[default]
    Unknown,
    /// Legacy branch aggregates do not describe the checked-out branch.
    #[serde(rename = "known")]
    LegacyBranches {
        branches: Vec<Comparison>,
    },
    Current {
        checkout: Checkout,
    },
}
impl Synchronization {
    pub fn current_commits(&self) -> Option<(usize, usize)> {
        match self {
            Self::Current {
                checkout: Checkout::Compared { ahead, behind, .. },
            } => Some((*ahead, *behind)),
            Self::Unknown | Self::LegacyBranches { .. } | Self::Current { .. } => None,
        }
    }
}
