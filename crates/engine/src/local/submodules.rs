//! Read declared submodules through their initialized local checkouts.
use super::open;
use crate::{cancel::Cancel, platform::files::is_link};
use anyhow::{Result, ensure};
use git2::Repository;
use std::{
    fs,
    io::ErrorKind,
    path::{Component, Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub path: PathBuf,
    pub entries: Vec<Entry>,
}
#[derive(Debug, Clone)]
pub struct Entry {
    pub path: PathBuf,
    pub state: State,
}
#[derive(Debug, Clone)]
pub enum State {
    Initialized { children: Vec<Entry> },
    Uninitialized,
    Unavailable(String),
}

pub fn read(path: &Path, cancel: &Cancel) -> Result<Snapshot> {
    cancel.check()?;
    let repo = open(path)?;
    Ok(Snapshot {
        path: path.to_owned(),
        entries: entries(&repo, path, cancel)?,
    })
}

fn entries(repo: &Repository, root: &Path, cancel: &Cancel) -> Result<Vec<Entry>> {
    let mut entries = Vec::new();
    for submodule in repo.submodules()? {
        cancel.check()?;
        let path = submodule.path().to_owned();
        let state = match child(root, &path, cancel) {
            Ok(state) => state,
            Err(error) => {
                cancel.check()?;
                State::Unavailable(format!("{error:#}"))
            }
        };
        entries.push(Entry { path, state });
    }
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(entries)
}

fn child(root: &Path, path: &Path, cancel: &Cancel) -> Result<State> {
    let relative = relative_directory(path)?;
    let directory = checked_directory(root, &relative)?;
    match fs::symlink_metadata(directory.join(".git")) {
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(State::Uninitialized),
        Err(error) => Err(error.into()),
        Ok(_) => {
            let repo = open(&directory)?;
            Ok(State::Initialized {
                children: entries(&repo, &directory, cancel)?,
            })
        }
    }
}

pub(crate) fn relative_directory(path: &Path) -> Result<PathBuf> {
    let mut relative = PathBuf::new();
    for component in path.components() {
        let Component::Normal(name) = component else {
            anyhow::bail!(i18n::tr!(SubmoduleRelativePath));
        };
        ensure!(
            !name.eq_ignore_ascii_case(".git") && !name.as_encoded_bytes().contains(&0),
            i18n::tr!(SubmoduleRelativePath)
        );
        relative.push(name);
    }
    ensure!(
        !relative.as_os_str().is_empty(),
        i18n::tr!(SubmoduleRelativePath)
    );
    Ok(relative)
}

/// Resolve a lexical child directory without traversing links or files.
pub(crate) fn checked_directory(root: &Path, relative: &Path) -> Result<PathBuf> {
    let mut current = root.to_owned();
    for component in relative.components() {
        current.push(component);
        match fs::symlink_metadata(&current) {
            Ok(metadata) => ensure!(
                metadata.is_dir() && !is_link(&metadata),
                i18n::tr!(SubmoduleDirectoryBlocked)
            ),
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(current)
}
