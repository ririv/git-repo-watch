//! Reviewed stash scopes keep selected paths explicit and preserve default Git messages.
mod selected;
use super::{Guard, Position, open};
use crate::{
    cancel::Cancel,
    transport::{Git, RepositoryStash},
};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    path::{Component, Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectedFiles {
    first: PathBuf,
    rest: Vec<PathBuf>,
}
impl SelectedFiles {
    pub fn new(files: impl IntoIterator<Item = PathBuf>) -> Result<Self> {
        let mut files = files.into_iter().collect::<BTreeSet<_>>().into_iter();
        let first = files
            .next()
            .ok_or_else(|| anyhow::anyhow!(i18n::tr!(StashSelectFiles)))?;
        let selected = Self {
            first,
            rest: files.collect(),
        };
        for path in selected.paths() {
            ensure!(
                !path.as_os_str().is_empty()
                    && path
                        .components()
                        .all(|part| matches!(part, Component::Normal(_))),
                i18n::tr!(StashInvalidPath)
            );
        }
        Ok(selected)
    }
    pub fn paths(&self) -> impl Iterator<Item = &Path> {
        std::iter::once(self.first.as_path()).chain(self.rest.iter().map(PathBuf::as_path))
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SaveMode {
    Tracked,
    IncludeUntracked,
    Selected { files: SelectedFiles },
}
impl SaveMode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Tracked => i18n::tr!(StashScopeTracked),
            Self::IncludeUntracked => i18n::tr!(StashScopeAll),
            Self::Selected { .. } => i18n::tr!(StashScopeSelected),
        }
    }
}
#[derive(Debug, Clone)]
pub enum Message {
    Automatic,
    Custom(String),
}
impl Message {
    pub fn text(&self) -> Option<&str> {
        match self {
            Self::Automatic => None,
            Self::Custom(text) => Some(text),
        }
    }
    pub fn input(&self) -> &str {
        self.text().unwrap_or("")
    }
}
impl From<String> for Message {
    fn from(text: String) -> Self {
        if text.trim().is_empty() {
            Self::Automatic
        } else {
            Self::Custom(text)
        }
    }
}
impl From<&str> for Message {
    fn from(text: &str) -> Self {
        text.to_owned().into()
    }
}

#[derive(Debug, Clone)]
pub struct SavePlan {
    pub path: PathBuf,
    pub mode: SaveMode,
    pub message: Message,
    pub files: Vec<PathBuf>,
    guard: Guard,
    head: git2::Oid,
}
pub(super) fn prepare(
    path: &Path,
    mode: SaveMode,
    message: Message,
    cancel: &Cancel,
) -> Result<SavePlan> {
    ensure!(
        !message.input().contains('\0'),
        i18n::tr!(StashInvalidMessage)
    );
    let repo = open(path)?;
    repo.signature()?;
    let guard = Guard::capture(path, cancel)?;
    let head = guard.position.oid()?;
    let snapshot = super::super::files::read(path)?;
    let candidates = snapshot
        .changes
        .iter()
        .filter(|change| {
            mode != SaveMode::Tracked || change.area != super::super::files::Area::Untracked
        })
        .flat_map(|change| change.location.paths().into_iter().map(Path::to_owned))
        .collect::<BTreeSet<_>>();
    let files = match &mode {
        SaveMode::Selected { files } => {
            ensure!(
                files.paths().all(|path| candidates.contains(path)),
                i18n::tr!(StashSelectionChanged)
            );
            files.paths().map(Path::to_owned).collect()
        }
        _ => candidates.into_iter().collect::<Vec<_>>(),
    };
    ensure!(!files.is_empty(), i18n::tr!(MutationNothingToStash));
    guard.validate(path, cancel)?;
    Ok(SavePlan {
        path: path.into(),
        mode,
        message,
        files,
        guard,
        head,
    })
}

impl SavePlan {
    pub(crate) fn head_oid(&self) -> git2::Oid {
        self.head
    }
    pub(super) fn execute(&self, git: &Git, cancel: &Cancel) -> Result<git2::Oid> {
        self.guard.validate(&self.path, cancel)?;
        ensure!(
            !self.message.input().contains('\0'),
            i18n::tr!(StashInvalidMessage)
        );
        let repo = open(&self.path)?;
        let mode = match self.mode {
            SaveMode::Selected { .. } => return selected::execute(self, git, cancel),
            SaveMode::Tracked => RepositoryStash::Tracked,
            SaveMode::IncludeUntracked => RepositoryStash::IncludeUntracked,
        };
        let before = current(&repo)?;
        git.save_stash(&self.path, mode, self.message.text(), cancel)?;
        let oid = repo.refname_to_id("refs/stash")?;
        ensure!(Some(oid) != before, i18n::tr!(MutationNothingToStash));
        Ok(oid)
    }
}

fn current(repo: &git2::Repository) -> Result<Option<git2::Oid>> {
    match repo.refname_to_id("refs/stash") {
        Ok(oid) => Ok(Some(oid)),
        Err(error) if error.code() == git2::ErrorCode::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}
