//! Immutable change units, editable commit plans, and one-step execution.
mod capture;
mod execute;
mod generate;
mod projection;
use crate::local::Head;
use anyhow::{Result, ensure};
pub use capture::capture;
pub use execute::{Progress, execute};
pub use generate::generate;
use git2::Oid;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    Staged,
    Worktree,
}
#[derive(Debug, Clone)]
pub enum Change {
    Hunk {
        patch: String,
    },
    Whole {
        before: PathBuf,
        after: PathBuf,
        source: Oid,
    },
}
#[derive(Debug, Clone)]
pub struct Unit {
    pub id: String,
    pub layer: Layer,
    pub path: PathBuf,
    pub preview: String,
    pub change: Change,
    pub dependencies: Vec<String>,
}
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub path: PathBuf,
    pub(crate) head: Head,
    pub(crate) index: Oid,
    pub(crate) worktree: Oid,
    pub units: Vec<Unit>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    pub message: String,
    pub units: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub commits: Vec<Step>,
}
#[derive(Debug, Clone)]
pub struct Prepared {
    pub snapshot: Snapshot,
    pub plan: Plan,
    pub completed: Vec<crate::operations::commit::CommitResult>,
}
impl Prepared {
    pub fn validate_structure(&self) -> Result<()> {
        ensure!(
            !self.plan.commits.is_empty() && self.plan.commits.len() <= 64,
            i18n::tr!(ComposeInvalid)
        );
        let mut seen = std::collections::HashSet::new();
        for step in &self.plan.commits {
            ensure!(
                !step.message.trim().is_empty()
                    && step.message.len() <= crate::commit_message::MESSAGE_LIMIT
                    && !step
                        .message
                        .chars()
                        .any(|c| c.is_control() && !matches!(c, '\n' | '\t'))
                    && !step.units.is_empty(),
                i18n::tr!(ComposeInvalid)
            );
            for id in &step.units {
                ensure!(
                    self.snapshot.units.iter().any(|unit| unit.id == *id) && seen.insert(id),
                    i18n::tr!(ComposeInvalid)
                );
            }
        }
        ensure!(
            seen.len() == self.snapshot.units.len(),
            i18n::tr!(ComposeCoverage)
        );
        let mut applied = std::collections::HashSet::new();
        for step in &self.plan.commits {
            for unit in self
                .snapshot
                .units
                .iter()
                .filter(|unit| step.units.contains(&unit.id))
            {
                ensure!(
                    unit.dependencies.iter().all(|id| applied.contains(id)),
                    "{}: {}",
                    i18n::tr!(ComposeDependency),
                    unit.id
                );
                applied.insert(unit.id.clone());
            }
        }
        Ok(())
    }
    pub fn validate(&self) -> Result<()> {
        self.validate_with_cancel(&crate::cancel::Cancel::default())
    }
    pub fn validate_with_cancel(&self, cancel: &crate::cancel::Cancel) -> Result<()> {
        cancel.check()?;
        self.validate_structure()?;
        projection::validate(self, cancel)
    }
}

#[cfg(test)]
mod tests;
