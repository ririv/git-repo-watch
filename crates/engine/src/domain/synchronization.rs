//! Structured comparisons count branches rather than summing overlapping commits.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Comparison {
    Equal,
    Ahead,
    Behind,
    Diverged,
}
impl Comparison {
    pub fn from_counts(ahead: usize, behind: usize) -> Self {
        match (ahead > 0, behind > 0) {
            (false, false) => Self::Equal,
            (true, false) => Self::Ahead,
            (false, true) => Self::Behind,
            (true, true) => Self::Diverged,
        }
    }
}
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Synchronization {
    #[default]
    Unknown,
    Known {
        branches: Vec<Comparison>,
    },
}
impl Synchronization {
    pub fn counts(&self) -> Option<(usize, usize)> {
        let Self::Known { branches } = self else {
            return None;
        };
        if branches.is_empty() {
            return None;
        }
        Some((
            branches
                .iter()
                .filter(|state| matches!(state, Comparison::Ahead | Comparison::Diverged))
                .count(),
            branches
                .iter()
                .filter(|state| matches!(state, Comparison::Behind | Comparison::Diverged))
                .count(),
        ))
    }
}
