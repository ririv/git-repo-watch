//! Commit replay commands retain their source selection and native recovery state.
pub(crate) mod controls;
use crate::{Action, Confirmation, Effect, Mode, Outcome, Screen, bar::CommandDraft};
use engine::{
    local::history::Oid,
    operations::{
        mutation,
        replay::{self, Observation, State},
    },
};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[derive(Debug, Default)]
pub(crate) struct Cache(BTreeMap<PathBuf, Observation>);
impl Cache {
    pub fn observe(&mut self, path: PathBuf, observation: Observation) {
        if self
            .0
            .get(&path)
            .is_none_or(|previous| observation.at >= previous.at)
        {
            self.0.insert(path, observation);
        }
    }
}
pub(crate) fn admits(state: Option<&State>, action: Action) -> bool {
    unavailable(state, action).is_none()
}
pub(crate) fn unavailable(state: Option<&State>, action: Action) -> Option<&'static str> {
    match action {
        Action::ReplayControl(_) if state.is_none() => Some(i18n::tr!(CommandReplayStateUnknown)),
        Action::ReplayControl(replay::Control::Continue) if matches!(state, Some(State::Active(active)) if !active.conflicts.is_empty()) => {
            Some(i18n::tr!(ReplayResolveFirst))
        }
        Action::ReplayControl(_) if !matches!(state, Some(State::Active(_))) => {
            Some(i18n::tr!(ReplayNotActive))
        }
        Action::Replay(_) | Action::ResetMenu | Action::ResetBranch(_)
            if matches!(state, Some(State::Active(_) | State::Other)) =>
        {
            Some(i18n::tr!(CommandReplayInProgress))
        }
        _ => None,
    }
}
impl Screen {
    pub(crate) fn replay_state(&self, path: &Path) -> Option<&State> {
        let overview = self
            .overview
            .content
            .snapshot()
            .filter(|s| s.path == path)
            .map(|s| &s.replay);
        let cached = self.replay.0.get(path);
        match (overview, cached) {
            (Some(a), Some(b)) => Some(if a.at > b.at { &a.state } else { &b.state }),
            (Some(a), None) | (None, Some(a)) => Some(&a.state),
            (None, None) => None,
        }
    }
    pub(crate) fn prepare_replay(
        &mut self,
        kind: replay::Kind,
        commits: Vec<Oid>,
        draft: CommandDraft,
        _permit: &crate::command::target::ExecutionPermit,
    ) {
        let Some(path) = draft.path.clone() else {
            self.message = crate::message::Message::error(i18n::tr!(SelectARepositoryFirst).into());
            return;
        };
        self.start_repository(
            path.clone(),
            kind.label(),
            Effect::ReadOnly,
            move |service, cancel, _| {
                if commits.len() > 1 {
                    return Ok(Outcome::MutationPrepared {
                        plan: Box::new(
                            service.prepare_cherry_pick_sequence(&path, &commits, &cancel)?,
                        ),
                        draft,
                    });
                }
                let commit = *commits
                    .first()
                    .ok_or_else(|| anyhow::anyhow!(i18n::tr!(ReplaySelectCommit)))?;
                Ok(Outcome::ReplayPrepared {
                    preparation: Box::new(service.prepare_replay(&path, kind, commit, &cancel)?),
                    draft,
                })
            },
        );
    }
    pub(crate) fn replay_prepared(
        &mut self,
        preparation: replay::Preparation,
        draft: CommandDraft,
        origin: Mode,
    ) {
        let operation = match preparation {
            replay::Preparation::Ready(plan) => Confirmation::Mutation {
                plan: Box::new(mutation::Plan::Replay(plan)),
                origin: Box::new(origin),
                draft,
            },
            replay::Preparation::Parents(candidate) => Confirmation::ReplayParents {
                candidate: Box::new(candidate),
                selected: 0,
                origin: Box::new(origin),
                draft,
            },
        };
        self.mode = Mode::Confirm {
            operation,
            choice: Default::default(),
        };
    }
    pub(crate) fn choose_replay_parent(
        &mut self,
        candidate: replay::Candidate,
        selected: usize,
        origin: Mode,
        draft: CommandDraft,
    ) {
        let parent = candidate.parents[selected].number;
        let path = candidate.path.clone();
        self.mode = origin;
        self.sync_bar();
        self.start_repository(
            path,
            candidate.kind.label(),
            Effect::ReadOnly,
            move |service, cancel, _| {
                Ok(Outcome::MutationPrepared {
                    plan: Box::new(service.choose_replay_parent(candidate, parent, &cancel)?),
                    draft,
                })
            },
        );
    }
}
pub(crate) fn review(plan: &replay::Plan) -> String {
    match plan {
        replay::Plan::Batch(batch) => {
            let mut text = format!(
                "{}: {}\n\n{} · {}\n",
                i18n::tr!(ReplayTargetBranch),
                batch.first.candidate.branch,
                i18n::tr!(ReplayOrder),
                batch.steps().count()
            );
            for (index, step) in batch.steps().enumerate() {
                text.push_str(&format!(
                    "\n{}  {}  {}\n{}\n",
                    index + 1,
                    step.candidate.commit,
                    step.candidate.summary,
                    step.summary
                ));
                for path in &step.files {
                    text.push_str(&format!("  {}\n", path.display()));
                }
            }
            text
        }
        replay::Plan::Start(p) => {
            let mut text = format!(
                "{}: {}\n{}: {}\n{}\n",
                i18n::tr!(ReplayTargetBranch),
                p.candidate.branch,
                i18n::tr!(ReplaySource),
                p.candidate.commit,
                p.candidate.summary
            );
            if let replay::Mainline::Merge(parent) = &p.mainline {
                text.push_str(&format!(
                    "\n{}: {} · {}\n{}\n",
                    i18n::tr!(ReplayMainline),
                    parent.number,
                    parent.oid,
                    parent.summary
                ));
            }
            text.push_str(&format!("\n{}\n", p.summary));
            for path in &p.files {
                text.push_str(&format!("\n{}", path.display()));
            }
            text
        }
        replay::Plan::Control(p) => {
            let mut text = format!(
                "{}: {}\n{}: {}\n{}: {}\n\n{}",
                i18n::tr!(ReplayTargetBranch),
                p.branch,
                i18n::tr!(ReplayActive),
                p.active.kind.label(),
                i18n::tr!(ReplaySource),
                p.active.commit,
                match p.control {
                    replay::Control::Continue => i18n::tr!(ReplayPendingHint),
                    replay::Control::Skip => i18n::tr!(ReplaySkipHint),
                    replay::Control::Abort => i18n::tr!(ReplayAbortHint),
                }
            );
            text.push_str(&format!(
                "\n\n{}: {}",
                i18n::tr!(ReplayQueueRemaining),
                p.active.remaining().len()
            ));
            for oid in p.active.remaining() {
                text.push_str(&format!("\n{oid}"));
            }
            if !p.active.conflicts.is_empty() {
                text.push_str(&format!("\n\n{}", i18n::tr!(ReplayConflictFiles)));
                for path in &p.active.conflicts {
                    text.push_str(&format!("\n{}", path.display()));
                }
            }
            text
        }
    }
}
