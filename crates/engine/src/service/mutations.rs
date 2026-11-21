//! Reviewed repository mutations share locking, bounded slots and local refresh.
use super::*;
use crate::operations::mutation::{self, Action, Change, Plan, Request};
impl Service {
    pub fn replay_state(
        &self,
        path: &Path,
        cancel: &Cancel,
    ) -> Result<operations::replay::Observation> {
        self.validate_repository(path, cancel)?;
        operations::replay::Observation::read(path)
    }
    pub fn prepare_replay(
        &self,
        path: &Path,
        kind: operations::replay::Kind,
        commit: git2::Oid,
        cancel: &Cancel,
    ) -> Result<operations::replay::Preparation> {
        let _lock = self.repository_lock(path, cancel)?;
        operations::replay::prepare(path, kind, commit, cancel)
    }
    pub fn choose_replay_parent(
        &self,
        candidate: operations::replay::Candidate,
        parent: std::num::NonZeroUsize,
        cancel: &Cancel,
    ) -> Result<Plan> {
        let _lock = self.repository_lock(&candidate.path, cancel)?;
        Ok(Plan::Replay(operations::replay::choose(
            candidate, parent, cancel,
        )?))
    }
    pub fn prepare_cherry_pick_sequence(
        &self,
        path: &Path,
        commits: &[git2::Oid],
        cancel: &Cancel,
    ) -> Result<Plan> {
        let _lock = self.repository_lock(path, cancel)?;
        Ok(Plan::Replay(operations::replay::batch::prepare(
            path, commits, cancel,
        )?))
    }
    pub fn stashes(&self, path: &Path, cancel: &Cancel) -> Result<local::stash::Snapshot> {
        self.validate_repository(path, cancel)?;
        local::stash::read(path, cancel)
    }
    pub fn stash_detail(
        &self,
        path: &Path,
        entry: &local::stash::Entry,
        scope: local::stash::preview::Scope,
        cancel: &Cancel,
    ) -> Result<local::stash::preview::Detail> {
        self.validate_repository(path, cancel)?;
        local::stash::preview::read(path, entry, scope, cancel)
    }
    pub fn stash_diff(
        &self,
        detail: &local::stash::preview::Detail,
        index: usize,
        cancel: &Cancel,
    ) -> Result<local::files::patch::Document> {
        self.validate_repository(&detail.path, cancel)?;
        local::stash::preview::document(detail, index, cancel)
    }
    pub fn prepare_mutation(&self, path: &Path, request: Request, cancel: &Cancel) -> Result<Plan> {
        self.prepare_mutation_observed(path, request, cancel, &mut |_| Ok(()))
    }
    pub fn prepare_mutation_observed(
        &self,
        path: &Path,
        request: Request,
        cancel: &Cancel,
        progress: &mut impl FnMut(crate::tasks::progress::Stage) -> Result<()>,
    ) -> Result<Plan> {
        let _lock = self.repository_lock_observed(path, cancel, progress)?;
        match request {
            Request::Branch { action, name } => Ok(Plan::Branch(local::branches::edit::prepare(
                path, &name, action, cancel,
            )?)),
            Request::Stash(request) => {
                Ok(Plan::Stash(local::stash::prepare(path, request, cancel)?))
            }
            Request::ReplayControl(control) => Ok(Plan::Replay(
                operations::replay::prepare_control(path, control, cancel)?,
            )),
            Request::Reset { target, mode } => Ok(Plan::Reset(operations::reset::prepare(
                path, target, mode, cancel,
            )?)),
            Request::Pull => {
                let result = operations::pull::prepare(&self.git, path, cancel).map(Plan::Pull);
                let completion = state::Completion::capture(&result, cancel, |_| ());
                self.refresh_after_write(
                    path,
                    result,
                    vec![],
                    state::Operation::PullFetch { completion },
                )
            }
            Request::Sync => {
                let result = operations::sync::prepare_observed(&self.git, path, cancel, progress)
                    .map(Plan::Sync);
                let completion = state::Completion::capture(&result, cancel, |_| {
                    Change::SynchronizationPrepared
                });
                let observed = progress(crate::tasks::progress::Stage::RefreshingState);
                let refreshed = self.refresh_after_write(
                    path,
                    result,
                    vec![],
                    state::Operation::Workspace {
                        action: Action::PrepareSync,
                        completion,
                    },
                );
                match (refreshed, observed) {
                    (result, Ok(())) => result,
                    (Ok(_), Err(error)) => Err(error),
                    (Err(operation), Err(progress)) => Err(operation.context(progress)),
                }
            }
        }
    }
    pub fn mutate(&self, plan: &Plan, cancel: &Cancel) -> Result<Change> {
        let _lock = self.repository_lock(plan.path(), cancel)?;
        let result = match plan {
            Plan::Sync(plan) => match operations::sync::execute(&self.git, plan, cancel) {
                operations::sync::Execution::Local(result) => {
                    result.map(|change| Change::Synchronized { result: change })
                }
                operations::sync::Execution::Published(publication) => {
                    let result = operations::sync::published_result(&publication)
                        .map(|change| Change::Synchronized { result: change });
                    return self.refresh_after_write(
                        plan.path(),
                        result,
                        vec![publication.clone()],
                        state::Operation::Push { publication },
                    );
                }
            },
            _ => mutation::execute(&self.git, plan, cancel),
        };
        let completion = state::Completion::capture(&result, cancel, Clone::clone);
        let change = self.refresh_after_write(
            plan.path(),
            result,
            vec![],
            state::Operation::Workspace {
                action: plan.action(),
                completion,
            },
        )?;
        if let Plan::Replay(operations::replay::Plan::Control(resume)) = plan {
            let identity = resume.active.operation();
            if matches!(operations::replay::state::read(&local::open(plan.path())?)?, operations::replay::State::Active(active) if active.operation() == identity)
            {
                return Ok(change);
            }
            for record in self.store.tasks(200)? {
                if record.pending()
                    && record.spec.operation.as_ref() == Some(&identity)
                    && matches!(&record.spec.scope, crate::tasks::Scope::Repository { path } if path == plan.path())
                {
                    self.store.resolve_task(&record.id)?;
                }
            }
        }
        Ok(change)
    }
    pub fn fetch(&self, path: &Path, cancel: &Cancel) -> Result<Change> {
        let _lock = self.repository_lock(path, cancel)?;
        let result = operations::fetch::all(&self.git, path, cancel).map(|outcome| match outcome {
            operations::fetch::Outcome::Fetched { remotes } => Change::Fetched { remotes },
            operations::fetch::Outcome::Skipped { reason } => Change::FetchSkipped { reason },
        });
        let completion = state::Completion::capture(&result, cancel, Clone::clone);
        self.refresh_after_write(
            path,
            result,
            vec![],
            state::Operation::Workspace {
                action: Action::Fetch,
                completion,
            },
        )
    }
}
