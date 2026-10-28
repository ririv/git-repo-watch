//! Reviewed commit replay composes native Git sequencing with immutable repository guards.
pub mod state;
use crate::{
    cancel::Cancel,
    local::{
        self,
        guard::{Guard, Position},
    },
    transport::Git,
};
use anyhow::{Context, Result, ensure};
use git2::Oid;
use serde::{Deserialize, Serialize};
pub use state::{Active, Observation, State};
use std::{
    num::NonZeroUsize,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    CherryPick,
    Revert,
}
impl Kind {
    pub fn command(self) -> &'static str {
        match self {
            Self::CherryPick => "cherry-pick",
            Self::Revert => "revert",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::CherryPick => i18n::tr!(ReplayCherryPick),
            Self::Revert => i18n::tr!(ReplayRevert),
        }
    }
    pub(crate) fn head_file(self) -> &'static str {
        match self {
            Self::CherryPick => "CHERRY_PICK_HEAD",
            Self::Revert => "REVERT_HEAD",
        }
    }
    pub(crate) fn todo_action(self) -> &'static str {
        match self {
            Self::CherryPick => "pick",
            Self::Revert => "revert",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Control {
    Continue,
    Skip,
    Abort,
}
impl Control {
    pub fn argument(self) -> &'static str {
        match self {
            Self::Continue => "--continue",
            Self::Skip => "--skip",
            Self::Abort => "--abort",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Continue => i18n::tr!(ReplayContinue),
            Self::Skip => i18n::tr!(ReplaySkip),
            Self::Abort => i18n::tr!(ReplayAbort),
        }
    }
}
#[derive(Debug, Clone)]
pub struct Parent {
    pub number: NonZeroUsize,
    pub oid: Oid,
    pub summary: String,
}
#[derive(Debug, Clone)]
pub struct Candidate {
    pub path: PathBuf,
    pub kind: Kind,
    pub branch: String,
    pub commit: Oid,
    pub summary: String,
    pub parents: Vec<Parent>,
    guard: Guard,
}
#[derive(Debug, Clone)]
pub enum Preparation {
    Ready(Plan),
    Parents(Candidate),
}
#[derive(Debug, Clone)]
pub enum Mainline {
    Ordinary,
    Merge(Parent),
}
#[derive(Debug, Clone)]
pub struct Start {
    pub candidate: Candidate,
    pub mainline: Mainline,
    pub files: Vec<PathBuf>,
    pub summary: String,
}
#[derive(Debug, Clone)]
pub struct Resume {
    pub path: PathBuf,
    pub control: Control,
    pub active: Active,
    pub branch: String,
    guard: Guard,
}
#[derive(Debug, Clone)]
pub enum Plan {
    Start(Start),
    Control(Resume),
}
impl Plan {
    pub fn path(&self) -> &Path {
        match self {
            Self::Start(p) => &p.candidate.path,
            Self::Control(p) => &p.path,
        }
    }
    pub fn kind(&self) -> Kind {
        match self {
            Self::Start(p) => p.candidate.kind,
            Self::Control(p) => p.active.kind,
        }
    }
    pub fn commit(&self) -> Oid {
        match self {
            Self::Start(p) => p.candidate.commit,
            Self::Control(p) => p.active.commit,
        }
    }
    pub fn title(&self) -> &'static str {
        match self {
            Self::Start(p) => p.candidate.kind.label(),
            Self::Control(p) => p.control.label(),
        }
    }
}
pub fn prepare(path: &Path, kind: Kind, commit: Oid, cancel: &Cancel) -> Result<Preparation> {
    let repo = local::open(path)?;
    local::guard::clean_tracked(&repo)?;
    let guard = Guard::capture(path, cancel)?;
    let Position::Attached { name, .. } = &guard.position else {
        anyhow::bail!(i18n::tr!(ReplayAttachedRequired));
    };
    let source = repo.find_commit(commit)?;
    let parents = source
        .parents()
        .enumerate()
        .map(|(i, parent)| {
            Ok(Parent {
                number: NonZeroUsize::new(i + 1).expect("one-based parent"),
                oid: parent.id(),
                summary: parent
                    .summary()
                    .context(i18n::tr!(ReplayInvalidMessage))?
                    .unwrap_or_default()
                    .into(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let candidate = Candidate {
        path: path.into(),
        kind,
        branch: name.trim_start_matches("refs/heads/").into(),
        commit,
        summary: source
            .summary()
            .context(i18n::tr!(ReplayInvalidMessage))?
            .unwrap_or_default()
            .into(),
        parents,
        guard,
    };
    if source.parent_count() > 1 {
        Ok(Preparation::Parents(candidate))
    } else {
        Ok(Preparation::Ready(finish(
            candidate,
            Mainline::Ordinary,
            cancel,
        )?))
    }
}
pub fn choose(candidate: Candidate, parent: NonZeroUsize, cancel: &Cancel) -> Result<Plan> {
    ensure!(candidate.parents.len() > 1, i18n::tr!(ReplayParentRequired));
    let selected = candidate
        .parents
        .iter()
        .find(|p| p.number == parent)
        .context(i18n::tr!(ReplayParentRequired))?
        .clone();
    finish(candidate, Mainline::Merge(selected), cancel)
}
fn finish(candidate: Candidate, mainline: Mainline, cancel: &Cancel) -> Result<Plan> {
    candidate.guard.validate(&candidate.path, cancel)?;
    let repo = local::open(&candidate.path)?;
    let commit = repo.find_commit(candidate.commit)?;
    let before = match &mainline {
        Mainline::Merge(p) => Some(repo.find_commit(p.oid)?.tree()?),
        Mainline::Ordinary => match commit.parents().next() {
            Some(p) => Some(p.tree()?),
            None => None,
        },
    };
    let after = commit.tree()?;
    let diff = repo.diff_tree_to_tree(before.as_ref(), Some(&after), None)?;
    let stats = diff.stats()?;
    let (insertions, deletions) = match candidate.kind {
        Kind::CherryPick => (stats.insertions(), stats.deletions()),
        Kind::Revert => (stats.deletions(), stats.insertions()),
    };
    let summary = i18n::tf!(ReplayStats, stats.files_changed(), insertions, deletions);
    let files = diff
        .deltas()
        .map(|d| {
            d.new_file()
                .path()
                .or_else(|| d.old_file().path())
                .map(Path::to_owned)
                .context(i18n::tr!(ChangeHasNoPath))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(Plan::Start(Start {
        candidate,
        mainline,
        files,
        summary,
    }))
}
pub fn prepare_control(path: &Path, control: Control, cancel: &Cancel) -> Result<Plan> {
    let repo = local::open(path)?;
    let State::Active(active) = state::read(&repo)? else {
        anyhow::bail!(i18n::tr!(ReplayNotActive));
    };
    if control == Control::Continue {
        ensure!(
            !repo.index()?.has_conflicts(),
            i18n::tr!(ReplayResolveFirst)
        );
    }
    let guard = Guard::capture_current(path, cancel)?;
    let branch = match &guard.position {
        Position::Attached { name, .. } => name.trim_start_matches("refs/heads/").into(),
        Position::Detached(oid) => oid.to_string(),
        Position::Unborn(_) => anyhow::bail!(i18n::tr!(ReplayAttachedRequired)),
    };
    Ok(Plan::Control(Resume {
        path: path.into(),
        control,
        active,
        branch,
        guard,
    }))
}
pub fn execute(git: &Git, plan: &Plan, cancel: &Cancel) -> Result<()> {
    match plan {
        Plan::Start(p) => {
            p.candidate.guard.validate(&p.candidate.path, cancel)?;
            let parent = match &p.mainline {
                Mainline::Ordinary => None,
                Mainline::Merge(parent) => Some(parent.number),
            };
            git.replay(
                &p.candidate.path,
                p.candidate.kind,
                p.candidate.commit,
                parent,
                cancel,
            )
        }
        Plan::Control(p) => {
            p.guard.validate_current(&p.path, cancel)?;
            ensure!(
                state::read(&local::open(&p.path)?)? == State::Active(p.active.clone()),
                i18n::tr!(MutationChanged)
            );
            git.replay_control(&p.path, p.active.kind, p.control, cancel)
        }
    }
}
