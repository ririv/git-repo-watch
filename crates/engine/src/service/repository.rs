//! Locked single-repository operations and manual journals.
use super::*;
pub(super) struct RepositoryLock {
    _repository: Lock,
    _slot: Lock,
}
impl Service {
    pub fn branches(
        &self,
        path: &Path,
        cancel: &Cancel,
    ) -> Result<local::branches::catalog::Snapshot> {
        self.validate_repository(path, cancel)?;
        local::branches::catalog::read(path, cancel)
    }
    pub fn overview(&self, path: &Path, cancel: &Cancel) -> Result<local::overview::Snapshot> {
        self.validate_repository(path, cancel)?;
        local::overview::read(path, cancel)
    }
    pub fn history(
        &self,
        path: &Path,
        request: local::history::Request,
        cancel: &Cancel,
    ) -> Result<local::history::Page> {
        self.validate_repository(path, cancel)?;
        local::history::read(path, request, cancel)
    }
    pub fn history_commit(
        &self,
        path: &Path,
        oid: git2::Oid,
        parent: local::history::Parent,
        cancel: &Cancel,
    ) -> Result<local::history::Detail> {
        self.validate_repository(path, cancel)?;
        local::history::detail(path, oid, parent, cancel)
    }
    pub fn history_diff(
        &self,
        detail: &local::history::Detail,
        change: &local::history::Change,
        cancel: &Cancel,
    ) -> Result<files::patch::Document> {
        self.validate_repository(&detail.path, cancel)?;
        local::history::document(detail, change, cancel)
    }
    pub fn open_terminal(&self, path: &Path, cancel: &Cancel) -> Result<()> {
        cancel.check()?;
        ensure!(
            self.store
                .config()?
                .repositories
                .iter()
                .any(|registered| registered == path),
            i18n::tf!(RepositoryIsNotCurrentlyRegistered, path.display())
        );
        crate::terminal::open(path, cancel)
    }

    pub(super) fn validate_repository(&self, path: &Path, cancel: &Cancel) -> Result<()> {
        cancel.check()?;
        ensure!(
            self.store.config()?.repositories.iter().any(|p| p == path),
            i18n::tf!(RepositoryIsNotCurrentlyRegistered, path.display())
        );
        local::open(path)?;
        Ok(())
    }
    pub(super) fn repository_lock(&self, path: &Path, cancel: &Cancel) -> Result<RepositoryLock> {
        cancel.check()?;
        self.validate_repository(path, cancel)?;
        let common = local::common_dir(&local::open(path)?)?;
        let lock = self.store.repository_lock(&common, cancel)?;
        self.validate_repository(path, cancel)?;
        let slot = self
            .store
            .operation_slot(self.store.config()?.max_concurrency, cancel)?;
        Ok(RepositoryLock {
            _repository: lock,
            _slot: slot,
        })
    }
    pub fn files(&self, path: &Path, cancel: &Cancel) -> Result<Files> {
        self.validate_repository(path, cancel)?;
        files::read(path)
    }
    pub fn diff(
        &self,
        snapshot: &Files,
        change: &Change,
        cancel: &Cancel,
    ) -> Result<files::patch::Document> {
        self.validate_repository(&snapshot.path, cancel)?;
        files::document(snapshot, change, cancel)
    }
    pub fn edit_index(
        &self,
        snapshot: &Files,
        operation: IndexAction,
        selected: &[Change],
        cancel: &Cancel,
    ) -> Result<Files> {
        let _lock = self.repository_lock(&snapshot.path, cancel)?;
        let result = files::edit(snapshot, operation, selected);
        let completion = state::Completion::capture(&result, cancel, |_| ());
        let operation = match operation {
            IndexAction::Stage => state::Operation::Stage { completion },
            IndexAction::Unstage => state::Operation::Unstage { completion },
        };
        self.refresh_after_write(&snapshot.path, result, vec![], operation)
    }
    pub fn prepare_commit(&self, path: &Path, cancel: &Cancel) -> Result<CommitPlan> {
        let _lock = self.repository_lock(path, cancel)?;
        local::review::prepare(path)
    }
    pub fn commit(
        &self,
        plan: &CommitPlan,
        message: &str,
        cancel: &Cancel,
    ) -> Result<CommitResult> {
        let _lock = self.repository_lock(&plan.path, cancel)?;
        let result = operations::commit::execute(&self.git, plan, message, cancel);
        let operation = state::Operation::Commit {
            branch: plan.head.branch().to_owned(),
            completion: state::Completion::capture(&result, cancel, Clone::clone),
        };
        self.refresh_after_write(&plan.path, result, vec![], operation)
    }
    pub fn prepare_push(&self, path: &Path, cancel: &Cancel) -> Result<PushPlan> {
        let _lock = self.repository_lock(path, cancel)?;
        operations::push::prepare(&self.git, path, cancel)
    }
    pub fn push(&self, plan: &PushPlan, cancel: &Cancel) -> Result<Publication> {
        let _lock = self.repository_lock(&plan.path, cancel)?;
        let result = operations::push::execute(&self.git, plan, cancel);
        let operation = state::Operation::Push {
            publication: result.clone(),
        };
        self.refresh_after_write(&plan.path, Ok(result.clone()), vec![result], operation)
    }
    pub(super) fn refresh_after_write<T>(
        &self,
        path: &Path,
        operation: Result<T>,
        publications: Vec<Publication>,
        operation_record: state::Operation,
    ) -> Result<T> {
        // 即使 Git 子进程失败或取消，也可能已经修改索引/引用，必须重新读取实际状态。
        let refresh_cancel = Cancel::default();
        let started_at = state::now();
        let (result, timing) = check::local_repository(path, &refresh_cancel);
        let result = with_publications(result, publications);
        let mut report =
            super::checks::single_report(result, timing, started_at, PushPolicy::Disabled);
        let record = state::OperationRecord {
            path: path.to_owned(),
            at: state::now(),
            operation: operation_record,
        };
        let saved = self
            .store
            .save_operation_report(&mut report, &record, &refresh_cancel);
        match (operation, saved) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(error), Ok(())) => Err(error),
            (Ok(_), Err(error)) => Err(error.context(i18n::tr!(RepositoryStateSaveFailed))),
            (Err(error), Err(save)) => Err(error.context(format!(
                "{}: {save:#}",
                i18n::tr!(RepositoryStateSaveFailed)
            ))),
        }
    }
    pub fn install(&self, cancel: &Cancel) -> Result<()> {
        self.control_schedule(crate::scheduler::Control::Enable, cancel)
    }
}

fn with_publications(result: RepositoryResult, publications: Vec<Publication>) -> RepositoryResult {
    let failures = publications
        .iter()
        .filter_map(|publication| match publication {
            Publication::Failed { target, error, .. } => Some(Issue {
                kind: IssueKind::PushFailed,
                detail: format!("{} · {}: {}", i18n::tr!(PushFailed), target.branch, error),
            }),
            Publication::Pushed { .. } => None,
        })
        .collect::<Vec<_>>();
    match result {
        RepositoryResult::Clean {
            path,
            branches_without_upstream,
            ..
        } if !failures.is_empty() => {
            let mut issues = failures.into_iter();
            RepositoryResult::Attention {
                path,
                branches_without_upstream,
                pushes: publications,
                issues: Issues::new(issues.next().expect("push failure"), issues.collect()),
            }
        }
        RepositoryResult::Clean {
            path,
            branches_without_upstream,
            ..
        } => RepositoryResult::Clean {
            path,
            branches_without_upstream,
            pushes: publications,
        },
        RepositoryResult::Attention {
            path,
            issues,
            branches_without_upstream,
            ..
        } => {
            let mut all = issues.iter().cloned().chain(failures);
            RepositoryResult::Attention {
                path,
                branches_without_upstream,
                pushes: publications,
                issues: Issues::new(
                    all.next().expect("nonempty repository issues"),
                    all.collect(),
                ),
            }
        }
        RepositoryResult::Failed { path, error, .. } => RepositoryResult::Failed {
            path,
            error,
            pushes: publications,
        },
    }
}
