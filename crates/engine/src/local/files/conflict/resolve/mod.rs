//! Resolve one captured marker block with guarded, atomic worktree writes.
mod markers;
mod reset;
use crate::{
    cancel::Cancel,
    local::{
        files::{
            Change, Files, Location,
            partial::{Version, capture},
            patch::Source,
        },
        index_lock::IndexLock,
        open,
    },
};
use anyhow::{Context, Result, ensure};
pub use markers::Block;
use std::{fs, io::Write, os::unix::fs::PermissionsExt, path::Path};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Choice {
    Left,
    Right,
    Both,
}
impl Choice {
    pub fn label(self) -> &'static str {
        match self {
            Self::Left => i18n::tr!(ConflictAcceptLeft),
            Self::Right => i18n::tr!(ConflictAcceptRight),
            Self::Both => i18n::tr!(ConflictAcceptBoth),
        }
    }
}
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Intent {
    Accept { choice: Choice },
    Undo,
    Reset,
}
#[derive(Debug, Clone)]
pub struct Snapshot {
    files: Files,
    change: Change,
    working: Version,
    blocks: Vec<Block>,
    marker_size: usize,
}
#[derive(Debug, Clone)]
pub struct Plan {
    intent: Intent,
    files: Files,
    change: Change,
    before: Version,
    after: Version,
}
impl Snapshot {
    pub fn capture(
        files: &Files,
        change: &Change,
        source: &Source,
        cancel: &Cancel,
    ) -> Result<Self> {
        cancel.check()?;
        ensure!(
            change.kind == 'U'
                && matches!(change.location, Location::Path(_))
                && files.changes.contains(change),
            i18n::tr!(ConflictFileRequired)
        );
        let repo = open(&files.path)?;
        super::super::validate(&repo, files)?;
        let working = capture::working(&capture::checked_worktree_path(
            &files.path,
            change.location.name(),
        )?)?;
        let (Version::Text { data, .. }, Source::Text { text, .. }) = (&working, source) else {
            anyhow::bail!(i18n::tr!(ConflictResolveUnsupported));
        };
        ensure!(data.as_ref() == text, i18n::tr!(ConflictResolveChanged));
        ensure!(
            data.lines().count() <= 50_000,
            i18n::tr!(ConflictResolveUnsupported)
        );
        let size = match git2::AttrValue::from_bytes(repo.get_attr_bytes(
            change.location.name(),
            "conflict-marker-size",
            git2::AttrCheckFlags::FILE_THEN_INDEX,
        )?) {
            git2::AttrValue::Unspecified => 7,
            git2::AttrValue::String(value) => value
                .parse::<usize>()
                .context(i18n::tr!(ConflictMarkersInvalid))?,
            _ => anyhow::bail!(i18n::tr!(ConflictMarkersInvalid)),
        };
        ensure!(
            (1..=1024).contains(&size),
            i18n::tr!(ConflictMarkersInvalid)
        );
        let blocks = markers::parse(data, size, cancel)?;
        super::super::validate(&repo, files)?;
        Ok(Self {
            files: files.clone(),
            change: change.clone(),
            working,
            blocks,
            marker_size: size,
        })
    }
    pub fn blocks(&self) -> &[Block] {
        &self.blocks
    }
    pub fn source(&self) -> &str {
        self.working.text()
    }
    pub fn prepare(&self, selected: usize, choice: Choice) -> Result<Plan> {
        let block = self
            .blocks
            .get(selected)
            .context(i18n::tr!(ConflictBlockRequired))?;
        let Version::Text { data, mode } = &self.working else {
            unreachable!("captured regular text")
        };
        let selected = match choice {
            Choice::Left => data[block.left.clone()].to_owned(),
            Choice::Right => data[block.right.clone()].to_owned(),
            Choice::Both => format!(
                "{}{}",
                &data[block.left.clone()],
                &data[block.right.clone()]
            ),
        };
        let next = format!(
            "{}{}{}",
            &data[..block.whole.start],
            selected,
            &data[block.whole.end..]
        );
        Ok(Plan {
            intent: Intent::Accept { choice },
            files: self.files.clone(),
            change: self.change.clone(),
            before: self.working.clone(),
            after: Version::Text {
                data: next.into(),
                mode: *mode,
            },
        })
    }
}
impl Plan {
    pub fn file(&self) -> &Path {
        self.change.location.name()
    }
    pub fn intent(&self) -> Intent {
        self.intent
    }
    pub fn path(&self) -> &Path {
        &self.files.path
    }
    pub fn inverse(&self) -> Self {
        Self {
            intent: Intent::Undo,
            files: self.files.clone(),
            change: self.change.clone(),
            before: self.after.clone(),
            after: self.before.clone(),
        }
    }
    pub fn expects(&self, source: &str) -> bool {
        self.before.text() == source
    }
    fn validate(&self, repo: &git2::Repository, cancel: &Cancel) -> Result<std::path::PathBuf> {
        cancel.check()?;
        super::super::validate(repo, &self.files)?;
        let full = capture::checked_worktree_path(&self.files.path, self.change.location.name())?;
        ensure!(
            capture::working(&full)? == self.before,
            i18n::tr!(ConflictResolveChanged)
        );
        Ok(full)
    }
}
pub fn execute(plan: &Plan, cancel: &Cancel) -> Result<Files> {
    let repo = open(&plan.files.path)?;
    let index = repo
        .index()?
        .path()
        .context(i18n::tr!(ConflictResolveChanged))?
        .to_owned();
    let _lock = IndexLock::new(&index)?;
    let full = plan.validate(&repo, cancel)?;
    let parent = full.parent().context(i18n::tr!(ConflictResolveChanged))?;
    let Version::Text { data, mode } = &plan.after else {
        unreachable!("resolution preserves a text file")
    };
    let mut pending = tempfile::NamedTempFile::new_in(parent)?;
    pending
        .as_file()
        .set_permissions(fs::Permissions::from_mode(mode & 0o777))?;
    pending.write_all(data.as_bytes())?;
    pending.as_file().sync_all()?;
    ensure!(
        plan.validate(&repo, cancel)? == full,
        i18n::tr!(ConflictResolveChanged)
    );
    pending.persist(&full).map_err(|error| error.error)?;
    fs::File::open(parent)?.sync_all()?;
    super::super::read(&plan.files.path)
}
