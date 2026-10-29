//! Read Git's persisted sequencer state, including worktree-specific metadata.
use super::Kind;
use crate::local;
use anyhow::{Context, Result, ensure};
use git2::{Oid, Repository, RepositoryState};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    time::Instant,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Queue {
    Single,
    Sequence { start: Oid, remaining: Vec<Oid> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Active {
    pub kind: Kind,
    pub commit: Oid,
    pub queue: Queue,
    pub conflicts: Vec<PathBuf>,
    fingerprint: [u8; 32],
}
impl Active {
    pub fn operation(&self) -> crate::tasks::Operation {
        match &self.queue {
            Queue::Single => crate::tasks::Operation::Replay {
                action: self.kind,
                commit: self.commit.to_string(),
            },
            Queue::Sequence { start, .. } => crate::tasks::Operation::ReplaySequence {
                action: self.kind,
                start: start.to_string(),
            },
        }
    }
    pub fn remaining(&self) -> &[Oid] {
        match &self.queue {
            Queue::Single => &[],
            Queue::Sequence { remaining, .. } => remaining,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    Idle,
    Active(Active),
    Other,
}
#[derive(Debug, Clone)]
pub struct Observation {
    pub at: Instant,
    pub state: State,
}
impl Observation {
    pub fn read(path: &Path) -> Result<Self> {
        let repo = local::open(path)?;
        let state = read(&repo)?;
        Ok(Self {
            at: Instant::now(),
            state,
        })
    }
}
pub fn kind(state: RepositoryState) -> Option<Kind> {
    match state {
        RepositoryState::CherryPick | RepositoryState::CherryPickSequence => Some(Kind::CherryPick),
        RepositoryState::Revert | RepositoryState::RevertSequence => Some(Kind::Revert),
        _ => None,
    }
}
fn metadata(repo: &Repository, name: &str) -> Result<Option<Vec<u8>>> {
    match fs::File::open(repo.path().join(name)) {
        Ok(file) => {
            let mut bytes = Vec::new();
            file.take(1_048_577).read_to_end(&mut bytes)?;
            ensure!(bytes.len() <= 1_048_576, i18n::tr!(ReplayMetadataTooLarge));
            Ok(Some(bytes))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}
pub fn read(repo: &Repository) -> Result<State> {
    let operation = repo.state();
    let Some(kind) = kind(operation) else {
        return Ok(if operation == RepositoryState::Clean {
            State::Idle
        } else {
            State::Other
        });
    };
    let mut hash = Sha256::new();
    let mut current = None;
    let mut start = None;
    let mut pending = None;
    for name in [
        "CHERRY_PICK_HEAD",
        "REVERT_HEAD",
        "MERGE_MSG",
        "sequencer/head",
        "sequencer/todo",
        "sequencer/opts",
        "sequencer/abort-safety",
    ] {
        hash.update(name.as_bytes());
        if let Some(bytes) = metadata(repo, name)? {
            hash.update((bytes.len() as u64).to_le_bytes());
            hash.update(&bytes);
            if name == kind.head_file() {
                current = Some(std::str::from_utf8(&bytes)?.trim().parse::<Oid>()?);
            } else if name == "sequencer/head" {
                start = Some(std::str::from_utf8(&bytes)?.trim().parse::<Oid>()?);
            } else if name == "sequencer/todo" {
                let mut commits = Vec::new();
                for row in std::str::from_utf8(&bytes)?
                    .lines()
                    .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
                {
                    let mut parts = row.split_whitespace();
                    ensure!(
                        parts.next() == Some(kind.todo_action()),
                        i18n::tr!(ReplayInvalidState)
                    );
                    let revision = parts.next().context(i18n::tr!(ReplayInvalidState))?;
                    ensure!(
                        revision.bytes().all(|b| b.is_ascii_hexdigit()),
                        i18n::tr!(ReplayInvalidState)
                    );
                    commits.push(repo.revparse_single(revision)?.peel_to_commit()?.id());
                }
                ensure!(!commits.is_empty(), i18n::tr!(ReplayInvalidState));
                if current.is_none() {
                    current = commits.first().copied();
                }
                pending = Some(commits);
            }
        } else {
            hash.update(b"absent");
        }
    }
    let commit = current.context(i18n::tr!(ReplayInvalidState))?;
    let queue = match (start, pending) {
        (None, None) => Queue::Single,
        (Some(start), Some(pending)) => {
            ensure!(
                pending.first() == Some(&commit),
                i18n::tr!(ReplayInvalidState)
            );
            Queue::Sequence {
                start,
                remaining: pending.into_iter().skip(1).collect(),
            }
        }
        _ => anyhow::bail!(i18n::tr!(ReplayInvalidState)),
    };
    let mut conflicts = Vec::new();
    for conflict in repo.index()?.conflicts()? {
        let conflict = conflict?;
        let entry = conflict
            .our
            .or(conflict.their)
            .or(conflict.ancestor)
            .context(i18n::tr!(ReplayInvalidState))?;
        conflicts.push(PathBuf::from(std::str::from_utf8(&entry.path)?));
    }
    conflicts.sort();
    conflicts.dedup();
    Ok(State::Active(Active {
        kind,
        commit,
        queue,
        conflicts,
        fingerprint: hash.finalize().into(),
    }))
}
