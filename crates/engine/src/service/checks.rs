//! Bounded Git operations and per-repository persistence; batch metadata is independent.
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
        let _batch = self.store.batch_lock()?;
        let mut config = self.store.config()?;
        if matches!(invocation, CheckInvocation::Manual) {
            config.push_policy = PushPolicy::Disabled;
        }
        // 提前校验旧数据；执行期间只在写入单个结果时持有短事务锁。
        self.store.repository_reports()?;
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
        self.store.repository_reports()?;
        emit(Event::Checking(path.to_owned()));
        let report = self.check_one(path, PushPolicy::Disabled, cancel)?;
        emit(Event::Checked {
            result: report.repositories[0].clone(),
            finished_at: report.finished_at.clone(),
            revision: report.repository_revisions[path],
        });
        Ok(report)
    }
    fn check_one(&self, path: &Path, policy: PushPolicy, cancel: &Cancel) -> Result<Report> {
        let _repository = self.repository_lock(path, cancel)?;
        let started_at = state::now();
        let (result, timing) = check::repository(&self.git, path, policy, cancel);
        cancel.check()?;
        let mut report = single_report(result, timing, started_at, policy);
        self.store.save_repository_report(&mut report, cancel)?;
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
        let mut results = BTreeMap::new();
        let mut timings = BTreeMap::new();
        let mut finished = BTreeMap::new();
        let mut revisions = BTreeMap::new();
        let mut notices = Vec::new();
        let mut error = None;
        let mut receive = |event: executor::Event<Result<Report>>| {
            let (index, report) = match event {
                executor::Event::Started { path, .. } => {
                    emit(Event::Checking(path));
                    return;
                }
                executor::Event::Finished { index, value } => (index, value),
            };
            match report {
                Err(failure) => {
                    if error.is_none() {
                        error = Some(failure);
                    }
                }
                Ok(report) => {
                    let result = report
                        .repositories
                        .into_iter()
                        .next()
                        .expect("single repository report");
                    for pushed in result.pushes() {
                        if let Publication::Failed { target, error, .. } = pushed {
                            let notice = notify::failure(result.path(), target, error, cancel);
                            if matches!(notice, Notice::Failed { .. }) {
                                emit(Event::NotificationProblem(notice.clone()));
                            }
                            notices.push(notice);
                        }
                    }
                    let revision = report.repository_revisions[result.path()];
                    revisions.insert(result.path().to_owned(), revision);
                    finished.insert(result.path().to_owned(), report.finished_at.clone());
                    emit(Event::Checked {
                        result: result.clone(),
                        finished_at: report.finished_at,
                        revision,
                    });
                    results.insert(index, result);
                    if let Some(execution) = report.execution {
                        timings.insert(
                            index,
                            execution
                                .repositories
                                .into_iter()
                                .next()
                                .expect("single repository timing"),
                        );
                    }
                }
            }
        };
        for (index, result, timing) in groups.failures {
            let mut report = single_report(result, timing, state::now(), config.push_policy);
            let saved = self
                .store
                .save_repository_report(&mut report, cancel)
                .map(|()| report);
            receive(executor::Event::Finished {
                index,
                value: saved,
            });
        }
        // 由执行器独立发送开始事件，写入完成事件必须在仓库锁内持久化之后才能发出。
        executor::run(
            groups.jobs,
            config.max_concurrency,
            cancel,
            |path| self.check_one(path, config.push_policy, cancel),
            &mut receive,
        )?;
        if let Some(error) = error {
            return Err(error);
        }
        let repositories = results.into_values().collect::<Vec<_>>();
        ensure!(
            repositories.len() == config.repositories.len(),
            i18n::tr!(CheckResultsAreIncompleteHistoricalReportWas)
        );
        Ok(Report {
            schema_version: 3,
            repository_finished_at: finished,
            repository_revisions: revisions,
            push_policy: config.push_policy,
            notifications: notices,
            started_at,
            finished_at: state::now(),
            status: status(&repositories),
            repositories,
            classification_updated_at: None,
            execution: Some(Execution {
                max_concurrency: config.max_concurrency,
                repositories: timings.into_values().collect(),
            }),
        })
    }
}
pub(super) fn single_report(
    result: RepositoryResult,
    timing: Timing,
    started_at: String,
    policy: PushPolicy,
) -> Report {
    let finished_at = state::now();
    let repositories = vec![result];
    Report {
        schema_version: 3,
        repository_revisions: BTreeMap::new(),
        repository_finished_at: [(repositories[0].path().to_owned(), finished_at.clone())].into(),
        push_policy: policy,
        notifications: vec![],
        started_at,
        finished_at,
        status: status(&repositories),
        repositories,
        classification_updated_at: None,
        execution: Some(Execution {
            max_concurrency: 1,
            repositories: vec![timing],
        }),
    }
}
fn status(repositories: &[RepositoryResult]) -> ReportStatus {
    if repositories.is_empty() {
        ReportStatus::Unconfigured
    } else if repositories
        .iter()
        .all(|r| matches!(r, RepositoryResult::Clean { .. }))
    {
        ReportStatus::Clean
    } else {
        ReportStatus::Attention
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
