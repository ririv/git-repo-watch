//! Locked single-repository operations and manual journals.
use super::*;
impl Service {
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

    fn validate_repository(&self, path: &Path, cancel: &Cancel) -> Result<()> {
        cancel.check()?;
        ensure!(
            self.store.config()?.repositories.iter().any(|p| p == path),
            i18n::tf!(RepositoryIsNotCurrentlyRegistered, path.display())
        );
        local::open(path)?;
        Ok(())
    }
    fn repository_lock(&self, path: &Path, cancel: &Cancel) -> Result<Lock> {
        cancel.check()?;
        let lock = self.store.lock()?;
        self.validate_repository(path, cancel)?;
        Ok(lock)
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
        files::edit(snapshot, operation, selected)
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
        let result = operations::commit::execute(&self.git, plan, message, cancel)?;
        let record = serde_json::json!({"at":state::now(),"path":plan.path,"branch":plan.head.branch(),"result":result});
        state::write_json(&self.store.state_dir().join("manual-commit.json"), &record)
            .context(i18n::tr!(CommitCreatedButTheOperationRecordCould))?;
        Ok(result)
    }
    pub fn prepare_push(&self, path: &Path, cancel: &Cancel) -> Result<PushPlan> {
        let _lock = self.repository_lock(path, cancel)?;
        operations::push::prepare(&self.git, path, cancel)
    }
    pub fn push(&self, plan: &PushPlan, cancel: &Cancel) -> Result<Publication> {
        let _lock = self.repository_lock(&plan.path, cancel)?;
        let result = operations::push::execute(&self.git, plan, cancel);
        let record = serde_json::json!({"at":state::now(),"path":plan.path,"push":result});
        state::write_json(&self.store.state_dir().join("manual-push.json"), &record)
            .context(i18n::tr!(PushCommandExecutedButTheOperationRecord))?;
        Ok(result)
    }
    pub fn install(&self, cancel: &Cancel) -> Result<()> {
        let _lock = self.store.lock()?;
        let c = self.store.config()?;
        self.scheduler.install(
            &self.store.home,
            &std::env::current_exe()?,
            c.interval_seconds,
            cancel,
        )
    }
}
