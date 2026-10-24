//! Composition uses the same repository lock and operation journal as ordinary commits.
use super::*;
use crate::composition::{self, Prepared, Progress};
impl Service {
    pub fn compose(
        &self,
        path: &Path,
        config: &crate::ai::Config,
        cancel: &Cancel,
    ) -> Result<Prepared> {
        let snapshot = {
            let _lock = self.repository_lock(path, cancel)?;
            composition::capture(path, cancel)?
        };
        composition::generate(snapshot, config, cancel)
    }
    pub fn compose_commit(&self, prepared: Prepared, cancel: &Cancel) -> Result<Progress> {
        let path = prepared.snapshot.path.clone();
        let branch = prepared.snapshot.head.branch().to_owned();
        let _lock = self.repository_lock(&path, cancel)?;
        let result = composition::execute(&self.git, prepared, cancel);
        let completion = state::Completion::capture(&result, cancel, |progress| match progress {
            Progress::More { result, .. } => result.clone(),
            Progress::Complete { completed } => {
                completed.last().expect("a confirmed commit").clone()
            }
        });
        self.refresh_after_write(
            &path,
            result,
            vec![],
            state::Operation::Commit { branch, completion },
        )
    }
}
