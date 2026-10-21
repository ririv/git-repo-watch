//! Reviewed, exact-OID, non-forced publication with remote verification.
use crate::{
    cancel::Cancel,
    domain::{Phase, Publication, PushTarget},
    local::{
        self, Head,
        branches::{self, Tracking},
    },
    transport::Git,
};
use anyhow::{Context, Result, ensure};
use std::path::{Path, PathBuf};
#[derive(Debug, Clone)]
pub struct PushPlan {
    pub path: PathBuf,
    pub target: PushTarget,
    pub url: String,
}
pub fn prepare(git: &Git, path: &Path, cancel: &Cancel) -> Result<PushPlan> {
    let repo = local::open(path)?;
    local::idle(&repo)?;
    let head = local::head(&repo)?;
    let Head::Existing {
        branch: ref name,
        oid,
    } = head
    else {
        anyhow::bail!("当前分支还没有提交")
    };
    let branch = branches::list(&repo)?
        .into_iter()
        .find(|b| b.name == *name)
        .context("当前本地分支不存在")?;
    let Tracking::Remote {
        remote,
        remote_ref,
        upstream,
    } = branch.tracking
    else {
        anyhow::bail!("当前分支没有配置远程上游，请先显式配置跟踪关系")
    };
    let url = branches::url(&repo, &remote)?;
    drop(repo);
    git.fetch(path, &remote, cancel)?;
    let server = git.head(path, &remote, &remote_ref, cancel)?;
    let repo = local::open(path)?;
    ensure!(
        branches::resolve(&repo, &upstream)? == server,
        "远程上游引用不一致，请检查 fetch 配置或稍后重试"
    );
    let (ahead, behind) = repo.graph_ahead_behind(oid, server)?;
    ensure!(
        behind == 0,
        "当前分支领先 {ahead}、落后 {behind} 个提交；请先手动处理远程更新"
    );
    ensure!(ahead > 0, "当前分支已与远程一致，没有待推送提交");
    ensure!(
        local::head(&repo)? == head,
        "准备期间分支或 HEAD 已变化，请重新操作"
    );
    Ok(PushPlan {
        path: path.to_owned(),
        target: PushTarget {
            branch: name.clone(),
            local_ref: branch.reference,
            local_oid: oid.to_string(),
            remote,
            remote_ref,
            remote_oid: server.to_string(),
            ahead_commits: ahead,
        },
        url,
    })
}
pub fn execute(git: &Git, plan: &PushPlan, cancel: &Cancel) -> Publication {
    match validate_review(plan) {
        Ok(()) => forward(git, &plan.path, &plan.target, cancel),
        Err(e) => Publication::Failed {
            target: plan.target.clone(),
            phase: Phase::Validate,
            error: format!("{e:#}"),
        },
    }
}
fn validate_review(plan: &PushPlan) -> Result<()> {
    let repo = local::open(&plan.path)?;
    ensure!(
        local::head(&repo)?
            == Head::Existing {
                branch: plan.target.branch.clone(),
                oid: plan.target.local_oid.parse()?
            },
        "分支或 HEAD 已变化，请重新确认推送"
    );
    ensure!(
        branches::url(&repo, &plan.target.remote)? == plan.url,
        "远程地址已变化，请重新确认推送"
    );
    Ok(())
}
pub fn forward(git: &Git, path: &Path, target: &PushTarget, cancel: &Cancel) -> Publication {
    let mut phase = Phase::Validate;
    let attempt = (|| -> Result<()> {
        cancel.check()?;
        ensure!(
            target.local_ref.starts_with("refs/heads/")
                && target.remote_ref.starts_with("refs/heads/")
                && target.ahead_commits > 0,
            "无效的推送目标"
        );
        let repo = local::open(path)?;
        local::idle(&repo)?;
        local::head(&repo)?;
        let url = branches::url(&repo, &target.remote)?;
        let local: git2::Oid = target.local_oid.parse()?;
        let remote: git2::Oid = target.remote_oid.parse()?;
        ensure!(
            branches::resolve(&repo, &target.local_ref)? == local,
            "本地分支在检查后发生变化，取消推送"
        );
        ensure!(
            git.head(path, &target.remote, &target.remote_ref, cancel)? == remote,
            "远程分支在检查后发生变化，取消推送"
        );
        ensure!(
            repo.graph_descendant_of(local, remote)?,
            "本地提交不是远程上游的后代"
        );
        ensure!(
            branches::url(&repo, &target.remote)? == url,
            "远程地址在核验期间发生变化，取消推送"
        );
        ensure!(
            branches::resolve(&repo, &target.local_ref)? == local,
            "本地分支在核验期间发生变化，取消推送"
        );
        phase = Phase::Transfer;
        git.push(path, target, cancel)?;
        phase = Phase::Verify;
        ensure!(
            git.head(path, &target.remote, &target.remote_ref, cancel)? == local,
            "推送命令已返回，但远程提交与预期不一致"
        );
        Ok(())
    })();
    match attempt {
        Ok(()) => Publication::Pushed {
            target: target.clone(),
        },
        Err(e) => Publication::Failed {
            target: target.clone(),
            phase,
            error: format!("{e:#}"),
        },
    }
}
