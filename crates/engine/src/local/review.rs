//! Immutable previews for commit validation.
use super::{Head, files, head, idle, open};
use anyhow::{Context, Result, ensure};
use git2::{DiffFindOptions, DiffFlags, Patch};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitFile {
    pub location: files::Location,
    pub diff: FileDiff,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct LineCounts {
    pub added: usize,
    pub removed: usize,
}
impl LineCounts {
    fn merge(self, other: Self) -> Self {
        Self {
            added: self.added + other.added,
            removed: self.removed + other.removed,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileDiff {
    Text(LineCounts),
    Binary,
}
impl FileDiff {
    fn merge(self, other: Self) -> Self {
        match (self, other) {
            (Self::Text(first), Self::Text(second)) => Self::Text(first.merge(second)),
            _ => Self::Binary,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitPlan {
    pub path: PathBuf,
    pub(crate) head: Head,
    pub(crate) tree: git2::Oid,
    pub files: Vec<CommitFile>,
    pub totals: LineCounts,
}
impl CommitPlan {
    pub fn branch(&self) -> &str {
        self.head.branch()
    }
}
pub fn prepare(path: &Path) -> Result<CommitPlan> {
    let snapshot = files::read(path)?;
    let repo = open(path)?;
    let base = match snapshot.head {
        Head::Initial { .. } => None,
        Head::Existing { oid, .. } => Some(repo.find_commit(oid)?.tree()?),
    };
    let tree = snapshot.index.tree()?;
    let staged = repo.find_tree(tree)?;
    let mut diff = repo.diff_tree_to_tree(base.as_ref(), Some(&staged), None)?;
    diff.find_similar(Some(DiffFindOptions::new().renames(true)))?;
    let count = diff.deltas().len();
    ensure!(count > 0, i18n::tr!(NoStagedChangesUseStageToSelect));
    let mut files: Vec<CommitFile> = Vec::with_capacity(count);
    for index in 0..count {
        let patch = Patch::from_diff(&diff, index)?.context(i18n::tr!(StagedDiffIsMissing))?;
        let delta = patch.delta();
        let binary = delta.flags().contains(DiffFlags::BINARY);
        let location = files::location(delta)?;
        let diff = if binary {
            FileDiff::Binary
        } else {
            let (_, added, removed) = patch.line_stats()?;
            FileDiff::Text(LineCounts { added, removed })
        };
        // Git 将文件类型变更表现为同一路径的删除和新增。
        if let Some(previous) = files.last_mut().filter(|file| file.location == location) {
            previous.diff = previous.diff.merge(diff);
        } else {
            files.push(CommitFile { location, diff });
        }
    }
    let mut totals = LineCounts::default();
    for file in &files {
        if let FileDiff::Text(counts) = file.diff {
            totals = totals.merge(counts);
        }
    }
    let plan = CommitPlan {
        path: path.to_owned(),
        head: snapshot.head,
        tree,
        files,
        totals,
    };
    validate(&plan)?;
    Ok(plan)
}
pub fn validate(plan: &CommitPlan) -> Result<()> {
    let repo = open(&plan.path)?;
    idle(&repo)?;
    ensure!(
        head(&repo)? == plan.head,
        i18n::tr!(BranchOrHeadChangedReviewTheCommit)
    );
    ensure!(
        repo.index()?.write_tree()? == plan.tree,
        i18n::tr!(StagedContentsChangedReviewTheCommitAgain)
    );
    Ok(())
}
