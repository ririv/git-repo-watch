//! Typed branch tracking and local graph queries.
use super::utf8_path;
use anyhow::{Context, Result, ensure};
use git2::{BranchType, Oid, Repository};
use std::path::Path;
#[derive(Debug, Clone)]
pub enum Tracking {
    Absent,
    Local,
    Remote {
        remote: String,
        remote_ref: String,
        upstream: String,
    },
}
#[derive(Debug, Clone)]
pub struct Branch {
    pub name: String,
    pub reference: String,
    pub oid: Oid,
    pub tracking: Tracking,
}
pub fn list(repo: &Repository) -> Result<Vec<Branch>> {
    let mut result = Vec::new();
    for entry in repo.branches(Some(BranchType::Local))? {
        let (branch, _) = entry?;
        let reference = branch
            .get()
            .name()
            .context("分支引用不是 UTF-8")?
            .to_owned();
        let name = branch.name()?.context("分支名不是 UTF-8")?.to_owned();
        let oid = branch.get().target().context("分支缺少提交")?;
        let tracking = match repo.branch_upstream_remote(&reference) {
            Ok(remote) => {
                let remote = remote.as_str().context("远程名不是 UTF-8")?.to_owned();
                if remote == "." {
                    Tracking::Local
                } else {
                    let config = repo.config()?;
                    let remote_ref = config.get_string(&format!("branch.{name}.merge"))?;
                    let upstream = repo
                        .branch_upstream_name(&reference)?
                        .as_str()
                        .context("上游引用不是 UTF-8")?
                        .to_owned();
                    Tracking::Remote {
                        remote,
                        remote_ref,
                        upstream,
                    }
                }
            }
            Err(e) if e.code() == git2::ErrorCode::NotFound => Tracking::Absent,
            Err(e) => return Err(e.into()),
        };
        result.push(Branch {
            name,
            reference,
            oid,
            tracking,
        });
    }
    result.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(result)
}
pub fn resolve(repo: &Repository, reference: &str) -> Result<Oid> {
    repo.find_reference(reference)?
        .resolve()?
        .target()
        .context("引用缺少对象 ID")
}
pub fn url(repo: &Repository, name: &str) -> Result<String> {
    let config = repo.config()?;
    let mut fetch = Vec::new();
    let mut entries = config.multivar(&format!("remote.{name}.url"), None)?;
    while let Some(e) = entries.next() {
        fetch.push(e?.value().context("远程地址不是 UTF-8")?.to_owned());
    }
    ensure!(fetch.len() == 1, "远程必须有且仅有一个获取地址");
    let mut push = Vec::new();
    match config.multivar(&format!("remote.{name}.pushurl"), None) {
        Ok(mut entries) => {
            while let Some(e) = entries.next() {
                push.push(e?.value().context("推送地址不是 UTF-8")?.to_owned())
            }
        }
        Err(e) if e.code() == git2::ErrorCode::NotFound => {}
        Err(e) => return Err(e.into()),
    };
    ensure!(
        push.len() <= 1,
        "推送地址与获取地址不同，或存在多个推送地址"
    );
    let remote = repo.find_remote(name)?;
    let resolved = remote.url().context("远程地址不是 UTF-8")?;
    ensure!(
        remote.pushurl()?.is_none_or(|u| u == resolved),
        "推送地址与检查地址不同"
    );
    Ok(resolved.to_owned())
}
pub fn is_root(path: &Path) -> Result<()> {
    let _ = super::open(Path::new(utf8_path(path)?))?;
    Ok(())
}
