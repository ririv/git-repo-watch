//! Git commits preserving user hooks and signing configuration.
use crate::{
    cancel::Cancel,
    local::{self, Head, review::CommitPlan},
    transport::Git,
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Review {
    Exact,
    Changed,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitResult {
    pub oid: String,
    pub review: Review,
}
pub fn execute(
    git: &Git,
    plan: &CommitPlan,
    message: &str,
    cancel: &Cancel,
) -> Result<CommitResult> {
    ensure!(
        !message.trim().is_empty() && !message.contains('\0'),
        i18n::tr!(CommitMessageMustNotBeEmptyOr)
    );
    cancel.check()?;
    local::review::validate(plan)?;
    git.commit(&plan.path, message, cancel)?;
    let repo =
        local::open(&plan.path).context(i18n::tr!(CommitCommandCompletedButTheRepositoryCould))?;
    let oid = repo
        .refname_to_id("HEAD")
        .context(i18n::tr!(CommitCommandCompletedButHeadCouldNot))?;
    let commit = repo.find_commit(oid)?;
    let expected_parent = match plan.head {
        Head::Initial { .. } => commit.parent_count() == 0,
        Head::Existing { oid, .. } => commit.parent_count() == 1 && commit.parent_id(0)? == oid,
    };
    let review = if commit.tree_id() == plan.tree
        && expected_parent
        && local::head(&repo)?.branch() == plan.head.branch()
    {
        Review::Exact
    } else {
        Review::Changed
    };
    Ok(CommitResult {
        oid: oid.to_string(),
        review,
    })
}
