//! Locked single-repository operations and manual journals.
use super::*;
impl Service {
    fn repository_lock(&self, path: &Path, cancel: &Cancel) -> Result<Lock> {
        cancel.check()?;
        let lock = self.store.lock()?;
        ensure!(
            self.store.config()?.repositories.iter().any(|p| p == path),
            i18n::tf!(RepositoryIsNotCurrentlyRegistered, path.display())
        );
        local::open(path)?;
        Ok(lock)
    }
    pub fn files(&self, path: &Path, cancel: &Cancel) -> Result<Files> {
        let _lock = self.repository_lock(path, cancel)?;
        files::read(path)
    }
    pub fn diff(&self, snapshot: &Files, change: &Change, cancel: &Cancel) -> Result<String> {
        let _lock = self.repository_lock(&snapshot.path, cancel)?;
        files::diff(snapshot, change)
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
