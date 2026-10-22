//! Read-only, paginated commit history from captured local refs and immutable Git objects.
mod commit;
mod parents;
mod walk;
pub use commit::{Base, Detail, Identity, Message, Parent, detail, document};
pub use commit::{Change, Kind};
pub use git2::Oid;
pub use parents::{Ancestry, Parents};
use std::{collections::BTreeMap, path::PathBuf, sync::Arc};
pub use walk::read;

pub const PAGE_SIZE: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Head {
    Attached { branch: String, oid: Oid },
    Detached(Oid),
    Unborn(String),
}
impl Head {
    pub fn oid(&self) -> Option<Oid> {
        match self {
            Self::Attached { oid, .. } | Self::Detached(oid) => Some(*oid),
            Self::Unborn(_) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Label {
    Local(String),
    Remote(String),
    Tag(String),
}
impl Label {
    pub fn name(&self) -> &str {
        match self {
            Self::Local(s) | Self::Remote(s) | Self::Tag(s) => s,
        }
    }
}

#[derive(Debug)]
pub struct References {
    pub path: PathBuf,
    pub head: Head,
    pub labels: BTreeMap<Oid, Vec<Label>>,
    pub depth: Depth,
    tips: Vec<Oid>,
    boundaries: std::collections::BTreeSet<Oid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Depth {
    Complete,
    Shallow,
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub oid: Oid,
    pub ancestry: Ancestry,
    pub subject: String,
    pub author: String,
    pub seconds: i64,
}
impl Entry {
    pub fn parents(&self) -> &[Oid] {
        self.ancestry.parents()
    }
    pub fn graph_parents(&self) -> &[Oid] {
        match &self.ancestry {
            Ancestry::Linked(parents) => parents.as_slice(),
            Ancestry::Root | Ancestry::Boundary(_) => &[],
        }
    }
}

#[derive(Debug, Clone)]
pub struct Cursor {
    references: Arc<References>,
    offset: usize,
}

#[derive(Debug, Clone)]
pub enum Request {
    Start,
    Continue(Cursor),
}

#[derive(Debug, Clone)]
pub enum Continuation {
    End,
    More(Cursor),
}

#[derive(Debug)]
pub struct Page {
    pub references: Arc<References>,
    pub entries: Vec<Entry>,
    pub continuation: Continuation,
}
