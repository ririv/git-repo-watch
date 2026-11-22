//! Cached status projection and compatible machine output.
use super::*;
impl Service {
    pub fn snapshot(&self, cancel: &Cancel) -> Result<Snapshot> {
        cancel.check()?;
        let c = self.store.config()?;
        let report = match self.store.report()? {
            None => CachedReport::Missing,
            Some(report) => {
                let registered = c.repositories.iter().collect::<BTreeSet<_>>();
                let reported = report
                    .repositories
                    .iter()
                    .map(|r| r.path().to_path_buf())
                    .collect::<BTreeSet<_>>();
                let scope = if registered.len() == reported.len()
                    && registered.iter().all(|p| reported.contains(*p))
                {
                    Scope::Current
                } else {
                    Scope::Outdated
                };
                CachedReport::Available {
                    report: Box::new(report),
                    scope,
                }
            }
        };
        #[cfg(feature = "developer-scheduler")]
        let (registration, schedule) = self.scheduler.schedule_state(&self.store, cancel)?;
        #[cfg(feature = "developer-scheduler")]
        let next_trigger = schedule.next_trigger(registration, chrono::Local::now())?;
        Ok(Snapshot {
            #[cfg(feature = "developer-scheduler")]
            registration,
            activity: if self.store.running()? {
                Activity::Running
            } else {
                Activity::Idle
            },
            #[cfg(feature = "developer-scheduler")]
            configured_interval_seconds: c.interval_seconds,
            #[cfg(feature = "developer-scheduler")]
            configured_anchor: c.schedule_anchor,
            #[cfg(feature = "developer-scheduler")]
            next_trigger,
            #[cfg(feature = "developer-scheduler")]
            push_policy: c.scheduled_actions.push_policy(),
            #[cfg(feature = "developer-scheduler")]
            scheduled_actions: c.scheduled_actions,
            repository_count: c.repositories.len(),
            #[cfg(feature = "developer-scheduler")]
            schedule,
            report,
            max_concurrency: c.max_concurrency,
            fetch_on_startup: c.fetch_on_startup,
        })
    }
}
impl Serialize for CachedReport {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        let value = match self {
            Self::Missing => serde_json::json!({"status":"missing"}),
            Self::Available { report, scope } => {
                let stats = Stats::of(report);
                let mut counts = serde_json::Map::new();
                for (key, value) in [
                    ("clean", stats.clean),
                    ("attention", stats.attention),
                    ("failed", stats.failed),
                ] {
                    if value > 0 {
                        counts.insert(key.into(), value.into());
                    }
                }
                let mut push_counts = serde_json::Map::new();
                for (key, value) in [("pushed", stats.pushed), ("failed", stats.push_failed)] {
                    if value > 0 {
                        push_counts.insert(key.into(), value.into());
                    }
                }
                serde_json::json!({"status":"available","outcome":report.status,"scope":scope,"started_at":report.started_at,"finished_at":report.finished_at,"counts":counts,"branches_without_upstream":{"branches":stats.no_upstream,"repositories":stats.no_upstream_repos,"unavailable_repositories":stats.failed},"push_policy":report.push_policy,"push_counts":push_counts,"notification_failures":stats.notice_failed})
            }
        };
        value.serialize(serializer)
    }
}
