//! Local repository capabilities implemented exclusively through libgit2.
pub mod branches;
pub mod files;
pub mod review;
use anyhow::{Context, Result, ensure};
use git2::{Repository, RepositoryOpenFlags, RepositoryState};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn open(path: &Path) -> Result<Repository> {
    let repo = Repository::open_ext(
        path,
        RepositoryOpenFlags::NO_SEARCH,
        std::iter::empty::<&Path>(),
    )?;
    let root = repo.workdir().context("不支持裸仓库")?;
    ensure!(
        fs::canonicalize(root)? == fs::canonicalize(path)?,
        "配置路径不是仓库根目录：{}",
        root.display()
    );
    Ok(repo)
}
pub fn idle(repo: &Repository) -> Result<()> {
    ensure!(
        repo.state() == RepositoryState::Clean,
        "仓库存在未完成的 Git 操作：{:?}",
        repo.state()
    );
    ensure!(!repo.index()?.has_conflicts(), "仓库存在未解决的冲突");
    Ok(())
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Head {
    Initial { branch: String },
    Existing { branch: String, oid: git2::Oid },
}
impl Head {
    pub fn branch(&self) -> &str {
        match self {
            Self::Initial { branch } | Self::Existing { branch, .. } => branch,
        }
    }
}
pub fn head(repo: &Repository) -> Result<Head> {
    ensure!(
        !repo.head_detached()?,
        "HEAD 处于游离状态，请先切换到本地分支"
    );
    match repo.head() {
        Ok(h) => Ok(Head::Existing {
            branch: h.shorthand().context("分支名不是 UTF-8")?.to_owned(),
            oid: h.target().context("分支缺少提交")?,
        }),
        Err(e) if e.code() == git2::ErrorCode::UnbornBranch => {
            let h = repo.find_reference("HEAD")?;
            let target = h.symbolic_target()?.context("HEAD 缺少符号分支")?;
            Ok(Head::Initial {
                branch: target
                    .strip_prefix("refs/heads/")
                    .context("HEAD 未关联本地分支")?
                    .to_owned(),
            })
        }
        Err(e) => Err(e.into()),
    }
}
pub fn common_dir(repo: &Repository) -> Result<PathBuf> {
    Ok(fs::canonicalize(repo.commondir())?)
}
pub fn utf8_path(path: &Path) -> Result<&str> {
    path.to_str()
        .context("Git 路径不是 UTF-8，当前界面无法表示；请使用 Git 处理该路径")
}
