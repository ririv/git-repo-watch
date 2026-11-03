//! Read local branch names and tip subjects without tracking or graph queries.
use crate::{cancel::Cancel, local::open};
use anyhow::{Context, Result};
use git2::BranchType;
use std::path::Path;

#[derive(Debug)]
pub struct Entry {
    pub name: String,
    pub subject: String,
}
pub fn read(path: &Path, cancel: &Cancel) -> Result<Vec<Entry>> {
    cancel.check()?;
    let repo = open(path)?;
    let mut entries = Vec::new();
    for branch in repo.branches(Some(BranchType::Local))? {
        cancel.check()?;
        let (branch, _) = branch?;
        let name = branch
            .name()?
            .context(i18n::tr!(BranchNameIsNotUtf8))?
            .to_owned();
        let commit = branch.get().peel_to_commit()?;
        let subject = std::str::from_utf8(commit.message_bytes())
            .context(i18n::tr!(BranchTipTextInvalid))?
            .split('\n')
            .next()
            .expect("split yields one part")
            .trim()
            .chars()
            .take(200)
            .collect();
        entries.push(Entry { name, subject });
    }
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(entries)
}
