//! Explicit scheduled-task controls independent of checks and report writes.
use super::*;
use crate::scheduler::Control;
impl Service {
    pub fn control_schedule(&self, operation: Control, cancel: &Cancel) -> Result<()> {
        let _lock = self.store.lock()?;
        cancel.check()?;
        let config = self.store.config()?;
        match operation {
            Control::Enable => self.scheduler.install(
                &self.store,
                &std::env::current_exe()?,
                config.interval_seconds,
                cancel,
            ),
            Control::Disable => self.scheduler.disable(&self.store, cancel),
            Control::Apply => self
                .scheduler
                .apply(&self.store, config.interval_seconds, cancel),
        }
    }
}
