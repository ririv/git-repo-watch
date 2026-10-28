//! Execute one validated workflow while its caller owns the repository lock.
use super::{Plan, Step, StepOutcome, StepRecord};
use crate::{
    cancel::Cancel,
    check,
    domain::{Publication, RepositoryResult, Timing},
    local::guard::Position,
    operations, state,
    transport::Git,
};
use anyhow::Result;
use std::{path::Path, time::Instant};

enum Refresh {
    Cached,
    Fetched(Vec<String>),
}
enum State {
    Running,
    Inspected(RepositoryResult),
    Failed(String),
}
pub(crate) fn execute(
    git: &Git,
    path: &Path,
    plan: &Plan,
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
    let mut refresh = Refresh::Cached;
    let mut publications = Vec::new();
    for &step in plan.steps() {
        let started_at = state::now();
        let at = Instant::now();
        let result = (|| -> Result<StepOutcome> {
            cancel.check()?;
            progress(step)?;
            if skip_without_remotes(path, step)? {
                return Ok(StepOutcome::Skipped {
                    reason: operations::SkipReason::NoRemotes,
                });
            }
            let detail = match step {
                Step::Fetch => {
                    let remotes = match operations::fetch::all(git, path, cancel)? {
                        operations::fetch::Outcome::Fetched { remotes } => remotes,
                        operations::fetch::Outcome::Skipped { reason } => {
                            return Ok(StepOutcome::Skipped { reason });
                        }
                    };
                    let detail = remotes.join(", ");
                    refresh = Refresh::Fetched(remotes);
                    detail
                }
                Step::Pull => {
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
                Step::Push | Step::Inspect => {
                    let (result, current) = match step {
                        Step::Push => check::publish_fetched(git, path, cancel),
                        Step::Inspect => check::local_repository(path, cancel),
                        _ => unreachable!(),
                    };
                    publications.extend_from_slice(result.pushes());
                    timing.verify_ms += current.verify_ms;
                    timing.push_ms += current.push_ms;
                    if let RepositoryResult::Failed { error, .. } = &result {
                        anyhow::bail!("{}\n{}", error, publication_detail(result.pushes()));
                    }
                    for publication in result.pushes() {
                        if let Publication::Failed { error, .. } = publication {
                            anyhow::bail!("{}\n{}", error, publication_detail(result.pushes()));
                        }
                    }
                    let detail = if step == Step::Push {
                        publication_detail(result.pushes())
                    } else {
                        result.label().to_owned()
                    };
                    if step == Step::Inspect {
                        state = State::Inspected(result);
                    }
                    detail
                }
            };
            Ok(StepOutcome::Succeeded { detail })
        })();
        let elapsed = at.elapsed().as_millis();
        match step {
            Step::Fetch => timing.fetch_ms += elapsed,
            Step::Push => {}
            Step::Pull | Step::Inspect => timing.local_ms += elapsed,
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
    let mut result = match state {
        State::Inspected(result) => result,
        State::Failed(error) => RepositoryResult::Failed {
            path: path.into(),
            error,
            pushes: vec![],
        },
        State::Running => unreachable!("validated workflow ends with inspection"),
    };
    match &mut result {
        RepositoryResult::Clean { pushes, .. }
        | RepositoryResult::Attention { pushes, .. }
        | RepositoryResult::Failed { pushes, .. } => *pushes = publications,
    }
    (result, timing)
}
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
