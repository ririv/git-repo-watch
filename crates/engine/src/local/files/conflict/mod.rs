//! Immutable index stages remain separate from the editable worktree result.
mod alignment;
pub mod pair;
pub mod resolve;
mod working;
use super::{
    Change, Files,
    patch::Source,
    source::{FileInfo, Storage},
};
use crate::{
    cancel::Cancel,
    local::{open, utf8_path},
};
pub use alignment::{Cell, Comparison, Row};
use anyhow::{Context, Result, ensure};
use git2::{FileMode, IndexEntry, Oid, Repository};
use std::path::PathBuf;
pub use working::read as working;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Base,
    Current,
    Incoming,
}
impl Role {
    pub const ALL: [Self; 3] = [Self::Base, Self::Current, Self::Incoming];
    pub fn label(self) -> &'static str {
        match self {
            Self::Base => i18n::tr!(ConflictBase),
            Self::Current => i18n::tr!(ConflictCurrent),
            Self::Incoming => i18n::tr!(ConflictIncoming),
        }
    }
    pub fn stage(self) -> u8 {
        match self {
            Self::Base => 1,
            Self::Current => 2,
            Self::Incoming => 3,
        }
    }
}
#[derive(Debug)]
pub enum Version {
    Absent,
    Present {
        path: PathBuf,
        oid: Oid,
        mode: u32,
        source: Source,
    },
}
impl Version {
    pub fn source(&self) -> &Source {
        match self {
            Self::Absent => &Source::Absent,
            Self::Present { source, .. } => source,
        }
    }
    pub fn text(&self) -> Option<&str> {
        match self {
            Self::Absent => Some(""),
            Self::Present {
                source: Source::Text { text, .. },
                ..
            } => Some(text),
            _ => None,
        }
    }
}
#[derive(Debug)]
pub struct Snapshot {
    pub path: PathBuf,
    pub base: Version,
    pub current: Version,
    pub incoming: Version,
    pub comparison: Comparison,
}
impl Snapshot {
    pub fn version(&self, role: Role) -> &Version {
        match role {
            Role::Base => &self.base,
            Role::Current => &self.current,
            Role::Incoming => &self.incoming,
        }
    }
}
pub fn read(files: &Files, change: &Change, cancel: &Cancel) -> Result<Snapshot> {
    cancel.check()?;
    let repo = open(&files.path)?;
    super::validate(&repo, files)?;
    ensure!(
        files.changes.contains(change) && change.kind == 'U',
        i18n::tr!(ConflictFileRequired)
    );
    let stages = repo.index()?.conflict_get(change.location.name())?;
    let base = version(&repo, stages.ancestor, cancel)?;
    let current = version(&repo, stages.our, cancel)?;
    let incoming = version(&repo, stages.their, cancel)?;
    let comparison = alignment::prepare(&base, &current, &incoming, cancel)?;
    super::validate(&repo, files)?;
    cancel.check()?;
    Ok(Snapshot {
        path: change.location.name().to_owned(),
        base,
        current,
        incoming,
        comparison,
    })
}
fn version(repo: &Repository, entry: Option<IndexEntry>, cancel: &Cancel) -> Result<Version> {
    cancel.check()?;
    let Some(entry) = entry else {
        return Ok(Version::Absent);
    };
    let path =
        PathBuf::from(std::str::from_utf8(&entry.path).context(i18n::tr!(GitPathIsNotUtf8And))?);
    utf8_path(&path)?;
    let mode = match entry.mode {
        0o100644 => FileMode::Blob,
        0o100755 => FileMode::BlobExecutable,
        0o100664 => FileMode::BlobGroupWritable,
        _ => {
            return Ok(Version::Present {
                path,
                oid: entry.id,
                mode: entry.mode,
                source: Source::Unavailable(super::patch::SourceOmission::NonRegular),
            });
        }
    };
    let binary = repo.find_blob(entry.id)?.is_binary();
    let source = FileInfo {
        path: path.clone(),
        oid: entry.id,
        mode,
        binary,
        exists: true,
    }
    .read(repo, Storage::Object)?;
    Ok(Version::Present {
        path,
        oid: entry.id,
        mode: entry.mode,
        source,
    })
}
