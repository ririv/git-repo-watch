//! Locked single-repository operations and manual journals.
use super::*;
pub(crate) struct RepositoryLock {
    _repository: Lock,
    _slot: Lock,
}
impl Service {
    pub fn command_head(&self, path: &Path, cancel: &Cancel) -> Result<local::head::Head> {
        self.validate_repository_access(path, cancel)?;
        local::head::read(path, cancel)
    }
    pub fn branch_names(
        &self,
        path: &Path,
        cancel: &Cancel,
    ) -> Result<Vec<local::branches::names::Entry>> {
        self.validate_repository(path, cancel)?;
        local::branches::names::read(path, cancel)
    }
    pub fn find_files(
        &self,
        root: &Path,
        query: &str,
        cancel: &Cancel,
    ) -> Result<local::browser::Search> {
        self.validate_repository(root, cancel)?;
        local::browser::find(root, query, cancel)
    }
    pub fn browse_directory(
        &self,
        root: &Path,
        relative: &Path,
        cancel: &Cancel,
    ) -> Result<local::browser::Directory> {
        self.validate_repository(root, cancel)?;
        local::browser::directory(root, relative, cancel)
    }
    pub fn browse_file(
        &self,
        root: &Path,
        relative: &Path,
        cancel: &Cancel,
    ) -> Result<local::browser::Content> {
        self.validate_repository(root, cancel)?;
        local::browser::read(root, relative, cancel)
    }
    pub fn branches(
        &self,
        path: &Path,
        cancel: &Cancel,
    ) -> Result<local::branches::catalog::Snapshot> {
        self.validate_repository(path, cancel)?;
        local::branches::catalog::read(path, cancel)
    }
    pub fn worktrees(&self, root: &Path, cancel: &Cancel) -> Result<local::worktrees::Snapshot> {
        self.validate_repository(root, cancel)?;
        local::worktrees::read(root, cancel)
    }

    pub fn worktree_add_candidates(
        &self,
        root: &Path,
        cancel: &Cancel,
    ) -> Result<local::worktrees::AddCandidates> {
        self.validate_repository(root, cancel)?;
        local::worktrees::add_candidates(root, cancel)
    }

    pub fn authorize_worktree(
        &mut self,
        root: &Path,
        entry: &local::worktrees::Entry,
        cancel: &Cancel,
    ) -> Result<()> {
        self.validate_repository(root, cancel)?;
        let local::worktrees::State::Available { git_dir, .. } = &entry.state else {
            anyhow::bail!(i18n::tr!(WorktreeUnavailable));
        };
        let repository = local::open(&entry.path)?;
        let common = local::common_dir(&repository)?;
        ensure!(
            common == local::common_dir(&local::open(root)?)?
                && &std::fs::canonicalize(repository.path())? == git_dir,
            i18n::tr!(MutationChanged)
        );
        self.session_worktrees.insert(entry.path.clone(), common);
        Ok(())
    }

    pub fn remotes(&self, path: &Path, cancel: &Cancel) -> Result<Vec<local::remotes::Endpoint>> {
        self.validate_repository_access(path, cancel)?;
        local::remotes::read(path, cancel)
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
    pub fn open_terminal(
        &self,
        repository: &Path,
        directory: &Path,
        cancel: &Cancel,
    ) -> Result<()> {
        self.validate_repository_access(repository, cancel)?;
        ensure!(
            directory.starts_with(repository),
            i18n::tr!(BrowserInvalidPath)
        );
        crate::terminal::open(directory, cancel)
    }

    fn validate_repository_access(&self, path: &Path, cancel: &Cancel) -> Result<()> {
        cancel.check()?;
        if self.session_repository() == Some(path)
            || self.store.config()?.repositories.iter().any(|p| p == path)
        {
            return Ok(());
        }
        let linked = self
            .session_worktrees
            .get(path)
            .map(|common| -> Result<bool> {
                Ok(local::common_dir(&local::open(path)?)? == *common)
            })
            .transpose()?
            .unwrap_or(false);
        ensure!(
            linked,
            i18n::tf!(RepositoryIsNotCurrentlyRegistered, path.display())
        );
        Ok(())
    }
    pub(super) fn validate_repository(&self, path: &Path, cancel: &Cancel) -> Result<()> {
        self.validate_repository_access(path, cancel)?;
        local::open(path)?;
        Ok(())
    }
    pub(crate) fn repository_lock(&self, path: &Path, cancel: &Cancel) -> Result<RepositoryLock> {
        self.repository_lock_observed(path, cancel, &mut |_| Ok(()))
    }
    pub(super) fn repository_lock_observed(
        &self,
        path: &Path,
        cancel: &Cancel,
        progress: &mut impl FnMut(crate::tasks::progress::Stage) -> Result<()>,
    ) -> Result<RepositoryLock> {
        use crate::tasks::progress::Stage;
        progress(Stage::WaitingRepository)?;
        cancel.check()?;
        self.validate_repository(path, cancel)?;
        let common = local::common_dir(&local::open(path)?)?;
        let lock = self.store.repository_lock(&common, cancel)?;
        progress(Stage::WaitingCapacity)?;
        self.validate_repository(path, cancel)?;
        let slot = self
            .store
            .operation_slot(self.store.config()?.max_concurrency, cancel)?;
        progress(Stage::InspectingRepository)?;
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
    pub fn conflict(
        &self,
        snapshot: &Files,
        change: &Change,
        cancel: &Cancel,
    ) -> Result<files::conflict::Snapshot> {
        self.validate_repository(&snapshot.path, cancel)?;
        files::conflict::read(snapshot, change, cancel)
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
    pub fn prepare_discard_file(
        &self,
        snapshot: Files,
        changes: &[Change],
        cancel: &Cancel,
    ) -> Result<files::discard::Plan> {
        let _lock = self.repository_lock(&snapshot.path, cancel)?;
        files::discard::Plan::prepare(snapshot, changes, cancel)
    }
    pub fn discard_file(&self, plan: &files::discard::Plan, cancel: &Cancel) -> Result<Files> {
        let path = plan.repository();
        let _lock = self.repository_lock(path, cancel)?;
        let result = plan.execute(cancel);
        let completion = state::Completion::capture(&result, cancel, |_| ());
        self.refresh_after_write(
            path,
            result,
            vec![],
            state::Operation::Discard { completion },
        )
    }
    pub fn edit_partial(&self, plan: &files::partial::Plan, cancel: &Cancel) -> Result<Files> {
        let path = &plan.snapshot().files().path;
        let _lock = self.repository_lock(path, cancel)?;
        let result = files::partial::execute(plan, cancel);
        let completion = state::Completion::capture(&result, cancel, |_| ());
        let operation = match plan.action() {
            files::partial::Action::Stage => state::Operation::Stage { completion },
            files::partial::Action::Unstage => state::Operation::Unstage { completion },
            files::partial::Action::Discard => state::Operation::Discard { completion },
        };
        self.refresh_after_write(path, result, vec![], operation)
    }
    pub fn resolve_conflict(
        &self,
        plan: &files::conflict::resolve::Plan,
        cancel: &Cancel,
    ) -> Result<Files> {
        let _lock = self.repository_lock(plan.path(), cancel)?;
        let result = files::conflict::resolve::execute(plan, cancel);
        let file = plan.file().to_owned();
        let intent = plan.intent();
        let completion = state::Completion::capture(&result, cancel, |_| {
            operations::mutation::Change::ResolvedConflict {
                file: file.clone(),
                intent,
            }
        });
        self.refresh_after_write(
            plan.path(),
            result,
            vec![],
            state::Operation::Workspace {
                action: operations::mutation::Action::ResolveConflict { file, intent },
                completion,
            },
        )
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
    pub fn prepare_interactive_push(
        &self,
        path: &Path,
        cancel: &Cancel,
    ) -> Result<operations::push::Preparation> {
        let _lock = self.repository_lock(path, cancel)?;
        operations::push::prepare_interactive(&self.git, path, cancel)
    }
    pub fn prepare_push_target(
        &self,
        request: &operations::push::TargetRequest,
        remote: &str,
        branch: &str,
        cancel: &Cancel,
    ) -> Result<PushPlan> {
        let _lock = self.repository_lock(&request.path, cancel)?;
        operations::push::prepare_target(&self.git, request, remote, branch, cancel)
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
        let result = with_publications(result, publications, operation_error(&operation_record));
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
    #[cfg(feature = "developer-scheduler")]
    pub fn install(&self, cancel: &Cancel) -> Result<()> {
        self.control_schedule(crate::scheduler::Control::Enable, cancel)
    }
}

fn operation_error(operation: &state::Operation) -> Option<&str> {
    fn failed<T>(completion: &state::Completion<T>) -> Option<&str> {
        match completion {
            state::Completion::Failed { error } => Some(error),
            _ => None,
        }
    }
    match operation {
        state::Operation::Discard { completion }
        | state::Operation::Stage { completion }
        | state::Operation::Unstage { completion }
        | state::Operation::PullFetch { completion } => failed(completion),
        state::Operation::Commit { completion, .. } => failed(completion),
        state::Operation::Workspace { completion, .. } => failed(completion),
        state::Operation::Push { .. } => None,
    }
}
fn with_publications(
    result: RepositoryResult,
    publications: Vec<Publication>,
    operation_error: Option<&str>,
) -> RepositoryResult {
    let failures = publications
        .iter()
        .filter_map(|publication| match publication {
            Publication::Failed { target, error, .. } => Some(Issue {
                kind: IssueKind::PushFailed,
                detail: format!("{} · {}: {}", i18n::tr!(PushFailed), target.branch, error),
            }),
            Publication::Pushed { .. } => None,
        })
        .chain(operation_error.map(|error| Issue {
            kind: IssueKind::OperationFailed,
            detail: error.to_owned(),
        }))
        .collect::<Vec<_>>();
    match result {
        RepositoryResult::Clean {
            path,
            synchronization,
            branches_without_upstream,
            ..
        } if !failures.is_empty() => {
            let mut issues = failures.into_iter();
            RepositoryResult::Attention {
                path,
                synchronization,
                branches_without_upstream,
                pushes: publications,
                issues: Issues::new(issues.next().expect("push failure"), issues.collect()),
            }
        }
        RepositoryResult::Clean {
            path,
            synchronization,
            branches_without_upstream,
            ..
        } => RepositoryResult::Clean {
            path,
            synchronization,
            branches_without_upstream,
            pushes: publications,
        },
        RepositoryResult::Attention {
            path,
            issues,
            synchronization,
            branches_without_upstream,
            ..
        } => {
            let mut all = issues.iter().cloned().chain(failures);
            RepositoryResult::Attention {
                path,
                synchronization,
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
