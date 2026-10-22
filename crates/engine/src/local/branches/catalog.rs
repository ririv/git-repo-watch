//! Read-only branch catalog, independent of remote checks and branch mutations.
use super::checkout::{self, Checkout};
use super::upstream::{self, Tracking};
use crate::{cancel::Cancel, local::open};
use anyhow::{Context, Result};
use git2::{BranchType, Oid};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub enum Kind {
    Local(Result<Tracking, String>),
    Remote,
}
#[derive(Debug, Clone)]
pub struct Entry {
    pub name: String,
    pub reference: String,
    pub oid: Oid,
    pub kind: Kind,
    pub subject: String,
    pub author: String,
    pub time: i64,
}
#[derive(Debug)]
pub enum Head {
    Branch(String),
    Detached(Oid),
    Unborn(String),
}
#[derive(Debug)]
pub struct Snapshot {
    pub path: PathBuf,
    pub head: Head,
    pub entries: Vec<Entry>,
}
impl Snapshot {
    pub fn current(&self, entry: &Entry) -> bool {
        matches!((&self.head, &entry.kind), (Head::Branch(name), Kind::Local(_)) if name == &entry.name)
    }
}
pub fn read(path: &Path, cancel: &Cancel) -> Result<Snapshot> {
    cancel.check()?;
    let repo = open(path)?;
    let head = match checkout::read(&repo)? {
        Checkout::Branch { name, .. } => Head::Branch(name),
        Checkout::Detached(oid) => Head::Detached(oid),
        Checkout::Unborn(name) => Head::Unborn(name),
    };
    let mut entries = Vec::new();
    for item in repo.branches(None)? {
        cancel.check()?;
        let (branch, kind) = item?;
        // 远程 HEAD 是符号别名，不作为另一条分支重复展示。
        if kind == BranchType::Remote && branch.get().symbolic_target()?.is_some() {
            continue;
        }
        let name = branch
            .name()?
            .context(i18n::tr!(BranchNameIsNotUtf8))?
            .to_owned();
        let reference = branch
            .get()
            .name()
            .context(i18n::tr!(BranchReferenceIsNotUtf8))?
            .to_owned();
        let commit = branch.get().peel_to_commit()?;
        let oid = commit.id();
        let kind = match kind {
            BranchType::Local => {
                Kind::Local(upstream::read(&repo, &name, oid).map_err(|error| format!("{error:#}")))
            }
            BranchType::Remote => Kind::Remote,
        };
        entries.push(Entry {
            name,
            reference,
            oid,
            kind,
            subject: std::str::from_utf8(commit.message_bytes())
                .context(i18n::tr!(BranchTipTextInvalid))?
                .split('\n')
                .next()
                .expect("split yields one part")
                .trim()
                .chars()
                .take(200)
                .collect(),
            author: std::str::from_utf8(commit.author().name_bytes())
                .context(i18n::tr!(BranchTipTextInvalid))?
                .chars()
                .take(200)
                .collect(),
            time: commit.time().seconds(),
        });
    }
    let mut snapshot = Snapshot {
        path: path.to_owned(),
        head,
        entries: Vec::new(),
    };
    entries.sort_by(|a, b| {
        let key = |entry: &Entry| match &entry.kind {
            Kind::Local(_) if snapshot.current(entry) => 0,
            Kind::Local(_) => 1,
            Kind::Remote => 2,
        };
        key(a).cmp(&key(b)).then_with(|| a.name.cmp(&b.name))
    });
    snapshot.entries = entries;
    Ok(snapshot)
}
