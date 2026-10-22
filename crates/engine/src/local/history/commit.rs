//! Immutable commit inspection and per-parent tree comparisons reuse the structured diff pipeline.
use crate::{
    cancel::Cancel,
    local::{
        files::{self, Location, Storage, patch::Document},
        open, utf8_path,
    },
};
use anyhow::{Context, Result, ensure};
use git2::{DiffOptions, Oid};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Base {
    Empty,
    Commit(Oid),
}

#[derive(Debug, Clone, Copy)]
pub enum Parent {
    First,
    Commit(Oid),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Added,
    Deleted,
    Modified,
    Renamed,
    Copied,
    Typechange,
}
impl Kind {
    pub fn symbol(self) -> char {
        match self {
            Self::Added => 'A',
            Self::Deleted => 'D',
            Self::Modified => 'M',
            Self::Renamed => 'R',
            Self::Copied => 'C',
            Self::Typechange => 'T',
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    pub kind: Kind,
    pub location: Location,
}

#[derive(Debug, Clone)]
pub struct Identity {
    pub name: String,
    pub email: String,
    pub seconds: i64,
}

#[derive(Debug, Clone)]
pub enum Message {
    Complete(String),
    Truncated(String),
}
impl Message {
    pub fn text(&self) -> &str {
        match self {
            Self::Complete(text) | Self::Truncated(text) => text,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Detail {
    pub path: PathBuf,
    pub oid: Oid,
    pub parents: Vec<Oid>,
    pub base: Base,
    pub author: Identity,
    pub committer: Identity,
    pub message: Message,
    pub changes: Vec<Change>,
}

fn identity(signature: git2::Signature<'_>) -> Result<Identity> {
    Ok(Identity {
        name: std::str::from_utf8(signature.name_bytes())
            .context(i18n::tr!(HistoryTextNotUtf8))?
            .to_owned(),
        email: std::str::from_utf8(signature.email_bytes())
            .context(i18n::tr!(HistoryTextNotUtf8))?
            .to_owned(),
        seconds: signature.when().seconds(),
    })
}

pub fn detail(path: &Path, oid: Oid, parent: Parent, cancel: &Cancel) -> Result<Detail> {
    cancel.check()?;
    let repo = open(path)?;
    let commit = repo.find_commit(oid)?;
    let parents = super::parents::original(&repo, &commit)?;
    let base = match parent {
        Parent::First => parents
            .first()
            .copied()
            .map(Base::Commit)
            .unwrap_or(Base::Empty),
        Parent::Commit(parent) => {
            ensure!(
                parents.contains(&parent),
                i18n::tr!(HistoryParentNotInCommit)
            );
            Base::Commit(parent)
        }
    };
    let before = match base {
        Base::Empty => None,
        Base::Commit(oid) => Some(
            repo.find_commit(oid)
                .context(i18n::tr!(HistoryParentUnavailable))?
                .tree()?,
        ),
    };
    let after = commit.tree()?;
    let mut options = DiffOptions::new();
    options.include_typechange(true);
    let mut diff = repo.diff_tree_to_tree(before.as_ref(), Some(&after), Some(&mut options))?;
    diff.find_similar(None)?;
    let mut changes = Vec::new();
    for delta in diff.deltas() {
        cancel.check()?;
        let kind = match delta.status() {
            git2::Delta::Added => Kind::Added,
            git2::Delta::Deleted => Kind::Deleted,
            git2::Delta::Modified => Kind::Modified,
            git2::Delta::Renamed => Kind::Renamed,
            git2::Delta::Copied => Kind::Copied,
            git2::Delta::Typechange => Kind::Typechange,
            status => anyhow::bail!("{}: {status:?}", i18n::tr!(HistoryUnsupportedChange)),
        };
        changes.push(Change {
            kind,
            location: files::location(delta)?,
        });
    }
    let text =
        std::str::from_utf8(commit.message_bytes()).context(i18n::tr!(HistoryTextNotUtf8))?;
    let message = if text.len() > 64 * 1024 {
        let mut end = 64 * 1024;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        Message::Truncated(text[..end].to_owned())
    } else {
        Message::Complete(text.to_owned())
    };
    cancel.check()?;
    Ok(Detail {
        path: path.to_owned(),
        oid,
        parents,
        base,
        author: identity(commit.author())?,
        committer: identity(commit.committer())?,
        message,
        changes,
    })
}

pub fn document(detail: &Detail, change: &Change, cancel: &Cancel) -> Result<Document> {
    cancel.check()?;
    ensure!(
        detail.changes.contains(change),
        i18n::tr!(FileDoesNotBelongToThisSnapshot)
    );
    let repo = open(&detail.path)?;
    let commit = repo.find_commit(detail.oid)?;
    let before = match detail.base {
        Base::Empty => {
            ensure!(
                super::parents::original(&repo, &commit)?.is_empty(),
                i18n::tr!(HistoryParentNotInCommit)
            );
            None
        }
        Base::Commit(oid) => {
            ensure!(
                super::parents::original(&repo, &commit)?.contains(&oid),
                i18n::tr!(HistoryParentNotInCommit)
            );
            Some(
                repo.find_commit(oid)
                    .context(i18n::tr!(HistoryParentUnavailable))?
                    .tree()?,
            )
        }
    };
    let after = commit.tree()?;
    let mut options = DiffOptions::new();
    options
        .disable_pathspec_match(true)
        .include_typechange(true);
    for path in change.location.paths() {
        options.pathspec(utf8_path(path)?);
    }
    let mut diff = repo.diff_tree_to_tree(before.as_ref(), Some(&after), Some(&mut options))?;
    diff.find_similar(None)?;
    files::collect(&repo, &diff, Storage::Object, cancel)
}
