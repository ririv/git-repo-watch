//! Immutable text selections project changes onto exactly one index entry or worktree file.
mod capture;
mod coherence;
mod execute;
mod preview;
mod project;
use super::{Change, Files};
use anyhow::{Result, ensure};
use std::{collections::BTreeSet, num::NonZeroU32, sync::Arc};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LineId {
    Old(NonZeroU32),
    New(NonZeroU32),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Stage,
    Unstage,
    Discard,
}
#[derive(Debug, Clone, PartialEq, Eq)]
enum Version {
    Absent,
    Text { data: Arc<str>, mode: u32 },
}
impl Version {
    fn text(&self) -> &str {
        match self {
            Self::Absent => "",
            Self::Text { data, .. } => data,
        }
    }
}
#[derive(Debug, Clone)]
enum Layer {
    Staged {
        before: Version,
        after: Version,
    },
    Working {
        before: Version,
        indexed: Version,
        checkout: Version,
        after: Version,
    },
}
#[derive(Debug, Clone)]
pub struct Snapshot {
    files: Files,
    change: Change,
    hunks: Vec<Vec<LineId>>,
    blocks: Vec<Vec<LineId>>,
    layer: Layer,
}
#[derive(Debug, Clone)]
pub struct Plan {
    action: Action,
    snapshot: Snapshot,
    next: Version,
}
impl Snapshot {
    pub fn files(&self) -> &Files {
        &self.files
    }
    pub fn change(&self) -> &Change {
        &self.change
    }
    pub fn hunks(&self) -> &[Vec<LineId>] {
        &self.hunks
    }
    pub fn prepare(&self, action: Action, selected: BTreeSet<LineId>) -> Result<Plan> {
        ensure!(!selected.is_empty(), i18n::tr!(PartialSelectRequired));
        let available = self
            .hunks
            .iter()
            .flatten()
            .copied()
            .collect::<BTreeSet<_>>();
        ensure!(selected.is_subset(&available), i18n::tr!(PartialChanged));
        let next = match (&self.layer, action) {
            (
                Layer::Working {
                    before, indexed, ..
                },
                Action::Stage,
            ) => project::apply(
                self,
                before,
                indexed,
                &selected,
                project::Direction::Forward,
            )?,
            (Layer::Staged { before, after }, Action::Unstage) => {
                project::apply(self, after, before, &selected, project::Direction::Reverse)?
            }
            (
                Layer::Working {
                    checkout, after, ..
                },
                Action::Discard,
            ) => project::apply(
                self,
                after,
                checkout,
                &selected,
                project::Direction::Reverse,
            )?,
            _ => anyhow::bail!(i18n::tr!(ChangesWrongArea)),
        };
        Ok(Plan {
            action,
            snapshot: self.clone(),
            next,
        })
    }
}
impl Plan {
    pub fn action(&self) -> Action {
        self.action
    }
    pub fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }
}
pub use execute::execute;
