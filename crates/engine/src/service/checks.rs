//! Parallel checks grouped by shared Git storage.
use super::*;
enum CheckInvocation {
    Manual,
    Scheduled,
}
impl Service {
    /// Check all registered repositories without publishing commits.
    pub fn run(&self, cancel: &Cancel, emit: impl FnMut(Event)) -> Result<Report> {
        self.run_batch(CheckInvocation::Manual, cancel, emit)
    }
    /// Apply the saved automatic push policy for a scheduled check.
    pub fn run_scheduled(&self, cancel: &Cancel, emit: impl FnMut(Event)) -> Result<Report> {
        self.run_batch(CheckInvocation::Scheduled, cancel, emit)
    }
    fn run_batch(
        &self,
        invocation: CheckInvocation,
        cancel: &Cancel,
        mut emit: impl FnMut(Event),
    ) -> Result<Report> {
        let _lock = self.store.lock()?;
        let mut config = self.store.config()?;
        match invocation {
            CheckInvocation::Manual => config.push_policy = PushPolicy::Disabled,
            CheckInvocation::Scheduled => {}
        }
        let report = self.execute_checks(&config, cancel, &mut emit)?;
        cancel.check()?;
        self.store.save_report(&report)?;
        Ok(report)
    }
    pub fn check_repository(
        &self,
        path: &Path,
        cancel: &Cancel,
        mut emit: impl FnMut(Event),
    ) -> Result<Report> {
        let _lock = self.store.lock()?;
        let mut config = self.store.config()?;
        ensure!(
            config
                .repositories
                .iter()
                .any(|registered| registered == path),
            i18n::tf!(RepositoryIsNotCurrentlyRegistered, path.display())
        );
        config.repositories = vec![path.to_owned()];
        config.max_concurrency = 1;
        config.push_policy = PushPolicy::Disabled;
        // 在可能发生 fetch 之前校验已有报告，避免执行成功后才发现缓存损坏。
        let mut reports = self.store.repository_reports()?;
        let report = self.execute_checks(&config, cancel, &mut emit)?;
        cancel.check()?;
        reports.insert(path.to_owned(), report.clone());
        state::write_json(
            &self.store.state_dir().join("repository-checks.json"),
            &reports,
        )
        .context(i18n::tr!(SingleReportSaveFailed))?;
        Ok(report)
    }
    fn execute_checks(
        &self,
        config: &state::Config,
        cancel: &Cancel,
        emit: &mut impl FnMut(Event),
    ) -> Result<Report> {
        cancel.check()?;
        let started_at = state::now();
        let groups = group(&config.repositories);
        cancel.check()?;
        let mut results = BTreeMap::new();
        let mut timings = BTreeMap::new();
        let mut notices = Vec::new();
        for (index, result, timing) in groups.failures {
            emit(Event::Checking(result.path().to_owned()));
            emit(Event::Checked(result.clone()));
            results.insert(index, result);
            timings.insert(index, timing);
        }
        executor::run(
            groups.jobs,
            config.max_concurrency,
            cancel,
            |path| check::repository(&self.git, path, config.push_policy, cancel),
            |event| match event {
                executor::Event::Started { path, .. } => emit(Event::Checking(path)),
                executor::Event::Finished {
                    index,
                    value: (result, timing),
                } => {
                    // 取消中的仓库并未完成检查，不能用取消错误覆盖上一轮有效状态。
                    if cancel.is_cancelled() {
                        return;
                    }
                    for pushed in result.pushes() {
                        if let Publication::Failed { target, error, .. } = pushed {
                            let notice = notify::failure(result.path(), target, error, cancel);
                            if matches!(notice, Notice::Failed { .. }) {
                                emit(Event::NotificationProblem(notice.clone()))
                            }
                            notices.push(notice)
                        }
                    }
                    emit(Event::Checked(result.clone()));
                    results.insert(index, result);
                    timings.insert(index, timing);
                }
            },
        )?;
        let repositories = results.into_values().collect::<Vec<_>>();
        ensure!(
            repositories.len() == config.repositories.len(),
            i18n::tr!(CheckResultsAreIncompleteHistoricalReportWas)
        );
        let status = if repositories.is_empty() {
            ReportStatus::Unconfigured
        } else if repositories
            .iter()
            .all(|r| matches!(r, RepositoryResult::Clean { .. }))
        {
            ReportStatus::Clean
        } else {
            ReportStatus::Attention
        };
        let report = Report {
            schema_version: 3,
            push_policy: config.push_policy,
            notifications: notices,
            started_at,
            finished_at: state::now(),
            status,
            repositories,
            classification_updated_at: None,
            execution: Some(Execution {
                max_concurrency: config.max_concurrency,
                repositories: timings.into_values().collect(),
            }),
        };
        cancel.check()?;
        Ok(report)
    }
}
pub struct Groups {
    pub jobs: Vec<Vec<(usize, PathBuf)>>,
    pub failures: Vec<(usize, RepositoryResult, Timing)>,
}
pub fn group(paths: &[PathBuf]) -> Groups {
    let mut jobs: BTreeMap<PathBuf, Vec<(usize, PathBuf)>> = BTreeMap::new();
    let mut failures = Vec::new();
    for (index, path) in paths.iter().enumerate() {
        let started = std::time::Instant::now();
        match local::open(path).and_then(|r| local::common_dir(&r)) {
            Ok(common) => jobs.entry(common).or_default().push((index, path.clone())),
            Err(e) => {
                let elapsed = started.elapsed().as_millis();
                failures.push((
                    index,
                    RepositoryResult::Failed {
                        path: path.clone(),
                        error: format!("{e:#}"),
                        pushes: Vec::new(),
                    },
                    Timing {
                        path: path.clone(),
                        elapsed_ms: elapsed,
                        local_ms: elapsed,
                        fetch_ms: 0,
                        verify_ms: 0,
                        push_ms: 0,
                    },
                ));
            }
        }
    }
    Groups {
        jobs: jobs.into_values().collect(),
        failures,
    }
}
