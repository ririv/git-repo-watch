//! Reviewed repository mutations share locking, bounded slots and local refresh.
use super::*;
use crate::operations::mutation::{self, Action, Change, Plan, Request};
impl Service {
    pub fn stashes(&self, path: &Path, cancel: &Cancel) -> Result<local::stash::Snapshot> {
        self.validate_repository(path, cancel)?;
        local::stash::read(path, cancel)
    }
    pub fn prepare_mutation(&self, path: &Path, request: Request, cancel: &Cancel) -> Result<Plan> {
        let _lock = self.repository_lock(path, cancel)?;
        match request {
            Request::Branch { action, name } => Ok(Plan::Branch(local::branches::edit::prepare(
                path, &name, action, cancel,
            )?)),
            Request::Stash(request) => {
                Ok(Plan::Stash(local::stash::prepare(path, request, cancel)?))
            }
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
        }
    }
    pub fn mutate(&self, plan: &Plan, cancel: &Cancel) -> Result<Change> {
        let _lock = self.repository_lock(plan.path(), cancel)?;
        let result = mutation::execute(&self.git, plan, cancel);
        let completion = state::Completion::capture(&result, cancel, Clone::clone);
        self.refresh_after_write(
            plan.path(),
            result,
            vec![],
            state::Operation::Workspace {
                action: plan.action(),
                completion,
            },
        )
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
