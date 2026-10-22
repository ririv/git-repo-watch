//! Typed operation events remain independent of the persistence backend.
use crate::{cancel::Cancel, domain::Publication, operations::commit::CommitResult};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Completion<T> {
    Succeeded { value: T },
    Failed { error: String },
    Cancelled { error: String },
}
impl<T> Completion<T> {
    pub fn capture<U>(
        result: &anyhow::Result<U>,
        cancel: &Cancel,
        value: impl FnOnce(&U) -> T,
    ) -> Self {
        match result {
            Ok(result) => Self::Succeeded {
                value: value(result),
            },
            Err(error) if cancel.is_cancelled() => Self::Cancelled {
                error: format!("{error:#}"),
            },
            Err(error) => Self::Failed {
                error: format!("{error:#}"),
            },
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Operation {
    Stage {
        completion: Completion<()>,
    },
    Unstage {
        completion: Completion<()>,
    },
    Commit {
        branch: String,
        completion: Completion<CommitResult>,
    },
    Push {
        publication: Publication,
    },
}
impl Operation {
    pub(super) fn kind(&self) -> &'static str {
        match self {
            Self::Stage { .. } => "stage",
            Self::Unstage { .. } => "unstage",
            Self::Commit { .. } => "commit",
            Self::Push { .. } => "push",
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationRecord {
    pub path: PathBuf,
    pub at: String,
    pub operation: Operation,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "snake_case")]
pub enum OperationOrigin {
    Imported,
    Observation { revision: std::num::NonZeroU64 },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredOperation {
    pub id: i64,
    pub origin: OperationOrigin,
    pub record: OperationRecord,
}
