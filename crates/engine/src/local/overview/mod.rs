//! Read-only repository overview, independent of network checks and file editing.
mod checkout;
mod worktree;
use crate::{cancel::Cancel, local::open};
use anyhow::Result;
pub use checkout::{Checkout, Comparison, Tracking};
pub use git2::RepositoryState as Operation;
use std::path::{Path, PathBuf};
pub use worktree::Changes;

pub type Section<T> = std::result::Result<T, String>;

#[derive(Debug)]
pub struct Snapshot {
    pub path: PathBuf,
    pub captured_at: String,
    pub checkout: Section<Checkout>,
    pub operation: Operation,
    pub changes: Section<Changes>,
    pub depth: crate::local::history::Depth,
}

fn section<T>(result: Result<T>) -> Section<T> {
    result.map_err(|error| format!("{error:#}"))
}

pub fn read(path: &Path, cancel: &Cancel) -> Result<Snapshot> {
    cancel.check()?;
    let repo = open(path)?;
    let checkout = section(checkout::read(&repo));
    cancel.check()?;
    let changes = section(worktree::read(&repo, cancel));
    cancel.check()?;
    Ok(Snapshot {
        path: path.to_owned(),
        captured_at: chrono::Utc::now().to_rfc3339(),
        checkout,
        operation: repo.state(),
        changes,
        depth: if repo.is_shallow() {
            crate::local::history::Depth::Shallow
        } else {
            crate::local::history::Depth::Complete
        },
    })
}
