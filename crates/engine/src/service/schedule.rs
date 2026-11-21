//! Explicit scheduled-task controls independent of checks and report writes.
use super::*;
use crate::scheduler::{
    Control, Registration, Schedule,
    timing::{Anchor, Basis, Plan},
};
impl Service {
    pub fn control_schedule(&self, operation: Control, cancel: &Cancel) -> Result<()> {
        let _lock = self.store.lock()?;
        cancel.check()?;
        if operation == Control::Disable {
            return self.scheduler.disable(&self.store, cancel);
        }
        self.scheduler.verify_install_target(&self.store)?;
        if operation == Control::Apply {
            self.scheduler.verify_owner(&self.store)?;
            #[cfg(unix)]
            ensure!(
                self.scheduler.definition().try_exists()?,
                i18n::tr!(ScheduleDefinitionMissing)
            );
        }
        let mut config = self.store.config()?;
        if config.schedule_anchor.is_unset() {
            ensure!(
                operation == Control::Enable
                    || self.scheduler.schedule_state(&self.store, cancel)?.0
                        == Registration::Loaded,
                i18n::tr!(ScheduleEnableToSetAnchor)
            );
            config.schedule_anchor = Anchor::Set(Basis::automatic(chrono::Local::now()));
        }
        let basis = config
            .schedule_anchor
            .basis()
            .expect("established anchor")
            .clone();
        let plan = Plan::new(config.interval_seconds, basis)?;
        match operation {
            Control::Enable => {
                self.scheduler
                    .install(&self.store, &std::env::current_exe()?, &plan, cancel)?
            }
            Control::Disable => unreachable!("disabled before preparing a plan"),
            Control::Apply => self.scheduler.apply(&self.store, &plan, cancel)?,
        }
        self.store.save_config(&config)
    }
    /// Wake only for the current minute's occurrence; completed slots are claimed once.
    pub fn scheduled_tick(
        &self,
        cancel: &Cancel,
        emit: impl FnMut(Event),
    ) -> Result<Option<Report>> {
        #[cfg(unix)]
        self.scheduler.verify_task_owner(&self.store)?;
        let (_, schedule) = self.scheduler.schedule_state(&self.store, cancel)?;
        let Schedule::Anchored { plan } = schedule else {
            anyhow::bail!(i18n::tr!(ScheduleEnableToSetAnchor));
        };
        let Some(due) = plan.occurrence_in_minute(chrono::Local::now())? else {
            return Ok(None);
        };
        loop {
            cancel.check()?;
            let remaining = due.signed_duration_since(chrono::Local::now());
            if remaining > chrono::Duration::minutes(1) {
                return Ok(None);
            }
            if remaining <= chrono::Duration::zero() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        if chrono::Local::now().signed_duration_since(due) >= chrono::Duration::minutes(1) {
            return Ok(None);
        }
        self.run_due(&plan, due.naive_local(), cancel, emit)
    }
}
