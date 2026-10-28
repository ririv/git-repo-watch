//! Read Git's persisted sequencer state, including worktree-specific metadata.
use super::Kind;
use crate::local;
use anyhow::{Context, Result, ensure};
use git2::{Oid, Repository, RepositoryState};
use sha2::{Digest, Sha256};
use std::{fs, io::Read, path::Path, time::Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Active {
    pub kind: Kind,
    pub commit: Oid,
    fingerprint: [u8; 32],
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
            } else if name == "sequencer/todo" && current.is_none() {
                let row = std::str::from_utf8(&bytes)?
                    .lines()
                    .find(|line| !line.trim().is_empty() && !line.starts_with('#'))
                    .context(i18n::tr!(ReplayInvalidState))?;
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
                current = Some(repo.revparse_single(revision)?.peel_to_commit()?.id());
            }
        } else {
            hash.update(b"absent");
        }
    }
    Ok(State::Active(Active {
        kind,
        commit: current.context(i18n::tr!(ReplayInvalidState))?,
        fingerprint: hash.finalize().into(),
    }))
}
