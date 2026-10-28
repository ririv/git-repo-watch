//! Fetch once, review a pinned upstream, then integrate only by fast-forward.
use crate::{
    cancel::Cancel,
    local::{
        self,
        guard::{Guard, Position},
    },
    transport::Git,
};
use anyhow::{Context, Result, ensure};
use git2::{Oid, Repository};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Source {
    Local {
        reference: String,
    },
    Remote {
        remote: String,
        merge: String,
        reference: String,
    },
}
impl Source {
    pub(crate) fn reference(&self) -> &str {
        match self {
            Self::Local { reference } | Self::Remote { reference, .. } => reference,
        }
    }
    pub(crate) fn read(repo: &Repository, branch: &str) -> Result<Self> {
        fn one(repo: &Repository, key: &str) -> Result<String> {
            let config = repo.config()?;
            let mut values = Vec::new();
            let mut entries = config
                .multivar(key, None)
                .context(i18n::tr!(MutationUpstreamRequired))?;
            while let Some(entry) = entries.next() {
                values.push(
                    entry?
                        .value()
                        .context(i18n::tr!(MutationUpstreamRequired))?
                        .to_owned(),
                );
            }
            ensure!(values.len() == 1, i18n::tr!(MutationUpstreamRequired));
            Ok(values.remove(0))
        }
        let remote = one(repo, &format!("branch.{branch}.remote"))?;
        let merge = one(repo, &format!("branch.{branch}.merge"))?;
        ensure!(
            merge.starts_with("refs/heads/") && git2::Reference::is_valid_name(&merge),
            i18n::tr!(MutationUpstreamRequired)
        );
        if remote == "." {
            Ok(Self::Local { reference: merge })
        } else {
            let reference =
                local::branches::fetch::Remote::read(repo, &remote)?.upstream(&merge)?;
            Ok(Self::Remote {
                remote,
                merge,
                reference,
            })
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub enum Update {
    Current,
    FastForward { commits: usize },
}
#[derive(Debug, Clone)]
pub struct Plan {
    pub path: PathBuf,
    pub branch: String,
    pub oid: Oid,
    pub update: Update,
    source: Source,
    guard: Guard,
}
impl Plan {
    pub fn upstream(&self) -> &str {
        self.source.reference()
    }
}
pub fn prepare(git: &Git, path: &Path, cancel: &Cancel) -> Result<Plan> {
    prepare_with_refresh(git, path, Refresh::Fetch, cancel)
}
/// Prepare against remote-tracking references refreshed earlier in the same repository lock.
pub(crate) fn prepare_fetched(
    git: &Git,
    path: &Path,
    remotes: &[String],
    cancel: &Cancel,
) -> Result<Plan> {
    prepare_with_refresh(git, path, Refresh::Fetched(remotes), cancel)
}
enum Refresh<'a> {
    Fetch,
    Fetched(&'a [String]),
}
fn prepare_with_refresh(git: &Git, path: &Path, refresh: Refresh, cancel: &Cancel) -> Result<Plan> {
    let repo = local::open(path)?;
    local::guard::clean_tracked(&repo)?;
    let guard = Guard::capture(path, cancel)?;
    let Position::Attached { name, oid: head } = &guard.position else {
        anyhow::bail!(i18n::tr!(MutationPullAttached));
    };
    let branch = name
        .strip_prefix("refs/heads/")
        .expect("attached branch")
        .to_owned();
    let source = Source::read(&repo, &branch)?;
    if let Source::Remote { remote, .. } = &source
        && match refresh {
            Refresh::Fetch => true,
            Refresh::Fetched(remotes) => !remotes.contains(remote),
        }
    {
        git.fetch_tracking(
            path,
            &local::branches::fetch::Remote::read(&repo, remote)?,
            cancel,
        )?;
    }
    guard.validate(path, cancel)?;
    ensure!(
        Source::read(&repo, &branch)? == source,
        i18n::tr!(MutationChanged)
    );
    let oid = repo
        .find_reference(source.reference())
        .context(i18n::tr!(RemoteUpstreamBranchDoesNotExistAlt))?
        .peel_to_commit()?
        .id();
    let (ahead, behind) = repo.graph_ahead_behind(*head, oid)?;
    ensure!(ahead == 0 || behind == 0, i18n::tr!(MutationPullDiverged));
    let update = if behind == 0 {
        Update::Current
    } else {
        Update::FastForward { commits: behind }
    };
    Ok(Plan {
        path: path.into(),
        branch,
        oid,
        update,
        source,
        guard,
    })
}
pub fn execute(git: &Git, plan: &Plan, cancel: &Cancel) -> Result<()> {
    plan.guard.validate(&plan.path, cancel)?;
    let repo = local::open(&plan.path)?;
    local::guard::clean_tracked(&repo)?;
    ensure!(
        Source::read(&repo, &plan.branch)? == plan.source,
        i18n::tr!(MutationChanged)
    );
    ensure!(
        repo.find_reference(plan.source.reference())?
            .peel_to_commit()?
            .id()
            == plan.oid,
        i18n::tr!(MutationChanged)
    );
    if let Update::FastForward { .. } = plan.update {
        git.fast_forward(&plan.path, plan.oid, cancel)?;
    }
    Ok(())
}
