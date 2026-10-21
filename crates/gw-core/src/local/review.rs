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
        "没有已暂存的改动；请通过 stage 选择提交内容"
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
        stat: format!(
            "{} 个文件，新增 {} 行，删除 {} 行",
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
        "分支或 HEAD 已变化，请重新查看提交内容"
    );
    ensure!(
        repo.index()?.write_tree()? == plan.tree,
        "暂存内容已变化，请重新查看提交内容"
    );
    Ok(())
}
