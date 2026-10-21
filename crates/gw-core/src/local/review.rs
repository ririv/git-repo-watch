//! Immutable previews for commit validation.
use super::{Head, files, head, idle, open};
use anyhow::{Result, ensure};
use std::path::{Path, PathBuf};
#[derive(Debug, Clone)]
pub struct CommitPlan {
    pub path: PathBuf,
    pub(crate) head: Head,
    pub(crate) tree: git2::Oid,
    pub files: Vec<PathBuf>,
    pub stat: String,
}
impl CommitPlan {
    pub fn branch(&self) -> &str {
        self.head.branch()
    }
}
pub fn prepare(path: &Path) -> Result<CommitPlan> {
    let snapshot = files::read(path)?;
    let selected = snapshot
        .changes
        .iter()
        .filter(|c| c.area == files::Area::Staged)
        .collect::<Vec<_>>();
    ensure!(
        !selected.is_empty(),
        gw_i18n::tr!(NoStagedChangesUseStageToSelect)
    );
    let repo = open(path)?;
    let tree = match snapshot.head {
        Head::Initial { .. } => None,
        Head::Existing { oid, .. } => Some(repo.find_commit(oid)?.tree()?),
    };
    let diff = repo.diff_tree_to_index(tree.as_ref(), None, None)?;
    let stats = diff.stats()?;
    let plan = CommitPlan {
        path: path.to_owned(),
        head: snapshot.head.clone(),
        tree: snapshot.tree,
        files: selected
            .iter()
            .map(|c| c.location.name().to_owned())
            .collect(),
        stat: gw_i18n::tf!(
            FilesInsertionsDeletions,
            stats.files_changed(),
            stats.insertions(),
            stats.deletions()
        ),
    };
    validate(&plan)?;
    Ok(plan)
}
pub fn validate(plan: &CommitPlan) -> Result<()> {
    let repo = open(&plan.path)?;
    idle(&repo)?;
    ensure!(
        head(&repo)? == plan.head,
        gw_i18n::tr!(BranchOrHeadChangedReviewTheCommit)
    );
    ensure!(
        repo.index()?.write_tree()? == plan.tree,
        gw_i18n::tr!(StagedContentsChangedReviewTheCommitAgain)
    );
    Ok(())
}
