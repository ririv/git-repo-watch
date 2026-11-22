//! Execute one validated workflow while its caller owns the repository lock.
#[cfg(feature = "developer-scheduler")]
use super::Plan;
use super::{Step, StepOutcome, StepRecord};
use crate::{
    cancel::Cancel,
    check,
    domain::{RepositoryResult, Timing},
    operations, state,
    transport::Git,
};
use anyhow::Result;
use std::{path::Path, time::Instant};

#[cfg(feature = "developer-scheduler")]
use crate::{domain::Publication, local::guard::Position};

#[derive(Clone, Copy)]
enum Action {
    Fetch,
    Inspect,
    #[cfg(feature = "developer-scheduler")]
    Pull,
    #[cfg(feature = "developer-scheduler")]
    Push,
}
impl From<Action> for Step {
    fn from(action: Action) -> Self {
        match action {
            Action::Fetch => Self::Fetch,
            Action::Inspect => Self::Inspect,
            #[cfg(feature = "developer-scheduler")]
            Action::Pull => Self::Pull,
            #[cfg(feature = "developer-scheduler")]
            Action::Push => Self::Push,
        }
    }
}

#[cfg(feature = "developer-scheduler")]
impl From<Step> for Action {
    fn from(step: Step) -> Self {
        match step {
            Step::Fetch => Self::Fetch,
            Step::Inspect => Self::Inspect,
            Step::Pull => Self::Pull,
            Step::Push => Self::Push,
        }
    }
}

pub(crate) fn fetch(
    git: &Git,
    path: &Path,
    cancel: &Cancel,
    progress: impl FnMut(Step) -> Result<()>,
) -> (RepositoryResult, Timing) {
    execute(
        git,
        path,
        [Action::Fetch, Action::Inspect],
        cancel,
        progress,
    )
}

#[cfg(feature = "developer-scheduler")]
pub(crate) fn scheduled(
    git: &Git,
    path: &Path,
    plan: &Plan,
    cancel: &Cancel,
    progress: impl FnMut(Step) -> Result<()>,
) -> (RepositoryResult, Timing) {
    execute(
        git,
        path,
        plan.steps().iter().copied().map(Action::from),
        cancel,
        progress,
    )
}

#[cfg(feature = "developer-scheduler")]
enum Refresh {
    Cached,
    Fetched(Vec<String>),
}
enum State {
    Running,
    Inspected(RepositoryResult),
    Failed(String),
}
fn execute(
    git: &Git,
    path: &Path,
    actions: impl IntoIterator<Item = Action>,
    cancel: &Cancel,
    mut progress: impl FnMut(Step) -> Result<()>,
) -> (RepositoryResult, Timing) {
    let started = Instant::now();
    let mut timing = Timing {
        path: path.into(),
        elapsed_ms: 0,
        fetch_ms: 0,
        local_ms: 0,
        verify_ms: 0,
        push_ms: 0,
        steps: vec![],
    };
    let mut state = State::Running;
    #[cfg(feature = "developer-scheduler")]
    let mut refresh = Refresh::Cached;
    #[cfg(feature = "developer-scheduler")]
    let mut publications = Vec::new();
    for action in actions {
        let step = Step::from(action);
        let started_at = state::now();
        let at = Instant::now();
        let result = (|| -> Result<StepOutcome> {
            cancel.check()?;
            progress(step)?;
            #[cfg(feature = "developer-scheduler")]
            if skip_without_remotes(path, step)? {
                return Ok(StepOutcome::Skipped {
                    reason: operations::SkipReason::NoRemotes,
                });
            }
            let detail = match action {
                Action::Fetch => {
                    let remotes = match operations::fetch::all(git, path, cancel)? {
                        operations::fetch::Outcome::Fetched { remotes } => remotes,
                        operations::fetch::Outcome::Skipped { reason } => {
                            return Ok(StepOutcome::Skipped { reason });
                        }
                    };
                    let detail = remotes.join(", ");
                    #[cfg(feature = "developer-scheduler")]
                    {
                        refresh = Refresh::Fetched(remotes);
                    }
                    detail
                }
                #[cfg(feature = "developer-scheduler")]
                Action::Pull => {
                    let plan = match &refresh {
                        Refresh::Cached => operations::pull::prepare(git, path, cancel)?,
                        Refresh::Fetched(remotes) => {
                            operations::pull::prepare_fetched(git, path, remotes, cancel)?
                        }
                    };
                    let before = crate::local::open(path)?.head()?.peel_to_commit()?.id();
                    operations::pull::execute(git, &plan, cancel)?;
                    format!("{}: {} → {}", plan.branch, before, plan.oid)
                }
                #[cfg(feature = "developer-scheduler")]
                Action::Push => {
                    let (result, current) = check::publish_fetched(git, path, cancel);
                    publications.extend_from_slice(result.pushes());
                    timing.verify_ms += current.verify_ms;
                    timing.push_ms += current.push_ms;
                    if let RepositoryResult::Failed { error, .. } = &result {
                        if result.pushes().is_empty() {
                            anyhow::bail!("{error}");
                        }
                        anyhow::bail!("{error}\n{}", publication_detail(result.pushes()));
                    }
                    if result.pushes().iter().any(|push| push.error().is_some()) {
                        anyhow::bail!("{}", publication_detail(result.pushes()));
                    }
                    publication_detail(result.pushes())
                }
                Action::Inspect => {
                    let (result, current) = check::local_repository(path, cancel);
                    timing.verify_ms += current.verify_ms;
                    timing.push_ms += current.push_ms;
                    if let RepositoryResult::Failed { error, .. } = &result {
                        anyhow::bail!("{error}");
                    }
                    let detail = result.label().to_owned();
                    state = State::Inspected(result);
                    detail
                }
            };
            Ok(StepOutcome::Succeeded { detail })
        })();
        let elapsed = at.elapsed().as_millis();
        match action {
            Action::Fetch => timing.fetch_ms += elapsed,
            #[cfg(feature = "developer-scheduler")]
            Action::Push => {}
            #[cfg(feature = "developer-scheduler")]
            Action::Pull => timing.local_ms += elapsed,
            Action::Inspect => timing.local_ms += elapsed,
        }
        let outcome = match result {
            Ok(outcome) => outcome,
            Err(error) => {
                let error = format!("{}: {error:#}", step.label());
                state = State::Failed(error.clone());
                StepOutcome::Failed { error }
            }
        };
        timing.steps.push(StepRecord {
            step,
            started_at,
            finished_at: state::now(),
            outcome,
        });
        if matches!(state, State::Failed(_)) {
            break;
        }
    }
    timing.elapsed_ms = started.elapsed().as_millis();
    let result = match state {
        State::Inspected(result) => result,
        State::Failed(error) => RepositoryResult::Failed {
            path: path.into(),
            error,
            pushes: vec![],
        },
        State::Running => unreachable!("validated workflow ends with inspection"),
    };
    #[cfg(feature = "developer-scheduler")]
    let mut result = result;
    #[cfg(feature = "developer-scheduler")]
    match &mut result {
        RepositoryResult::Clean { pushes, .. }
        | RepositoryResult::Attention { pushes, .. }
        | RepositoryResult::Failed { pushes, .. } => *pushes = publications,
    }
    (result, timing)
}
#[cfg(feature = "developer-scheduler")]
fn skip_without_remotes(path: &Path, step: Step) -> Result<bool> {
    if !matches!(step, Step::Pull | Step::Push) {
        return Ok(false);
    }
    let repo = crate::local::open(path)?;
    if !repo.remotes()?.is_empty() {
        return Ok(false);
    }
    if step == Step::Pull
        && let Position::Attached { name, .. } = Position::read(&repo)?
    {
        let branch = name
            .strip_prefix("refs/heads/")
            .expect("local branch reference");
        match repo
            .config()?
            .get_string(&format!("branch.{branch}.remote"))
        {
            Ok(remote) if remote == "." => return Ok(false),
            Ok(_) => {}
            Err(error) if error.code() == git2::ErrorCode::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(true)
}
#[cfg(feature = "developer-scheduler")]
fn publication_detail(publications: &[Publication]) -> String {
    if publications.is_empty() {
        return i18n::tr!(WorkflowNoPush).into();
    }
    publications
        .iter()
        .map(|publication| match publication {
            Publication::Pushed { target } => format!(
                "✓ {} → {}:{} · {} → {}",
                target.branch,
                target.remote,
                target.remote_ref,
                target.remote_oid,
                target.local_oid
            ),
            Publication::Failed { target, error, .. } => format!(
                "✗ {} → {}:{} · {error}",
                target.branch, target.remote, target.remote_ref
            ),
        })
        .collect::<Vec<_>>()
        .join("\n")
}
