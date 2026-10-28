//! Bounded Git operations and per-repository persistence; batch metadata is independent.
use super::*;
enum CheckInvocation {
    Manual,
    Startup,
    Scheduled,
}
#[derive(Clone, Copy)]
enum RunPlan<'a> {
    Manual,
    Workflow(&'a crate::workflow::Plan),
}
impl RunPlan<'_> {
    fn policy(self) -> PushPolicy {
        match self {
            Self::Manual => PushPolicy::Disabled,
            Self::Workflow(plan) => plan.push_policy(),
        }
    }
}
impl Service {
    /// Check all registered repositories without publishing commits.
    pub fn run(&self, cancel: &Cancel, emit: impl FnMut(Event)) -> Result<Report> {
        self.run_batch(CheckInvocation::Manual, cancel, emit, None)
    }
    /// Execute the configured scheduled workflow.
    pub fn run_scheduled(&self, cancel: &Cancel, emit: impl FnMut(Event)) -> Result<Report> {
        self.run_batch(CheckInvocation::Scheduled, cancel, emit, None)
    }
    /// Execute the explicitly configured startup workflow.
    pub fn run_startup(&self, cancel: &Cancel, emit: impl FnMut(Event)) -> Result<Report> {
        self.run_batch(CheckInvocation::Startup, cancel, emit, None)
    }
    pub fn retry_check_all(
        &self,
        cancel: &Cancel,
        emit: impl FnMut(Event),
        parent: String,
    ) -> Result<Report> {
        self.run_batch(CheckInvocation::Manual, cancel, emit, Some(parent))
    }
    fn run_batch(
        &self,
        invocation: CheckInvocation,
        cancel: &Cancel,
        emit: impl FnMut(Event),
        retry_of: Option<String>,
    ) -> Result<Report> {
        let _batch = self.store.batch_lock()?;
        self.execute_batch(invocation, cancel, emit, retry_of)
    }
    pub(super) fn run_due(
        &self,
        plan: &crate::scheduler::timing::Plan,
        due: chrono::NaiveDateTime,
        cancel: &Cancel,
        emit: impl FnMut(Event),
    ) -> Result<Option<Report>> {
        let _batch = self.store.batch_lock()?;
        {
            let _configuration = self.store.transaction(cancel)?;
            if self.scheduler.registration(cancel)? != crate::scheduler::Registration::Loaded {
                return Ok(None);
            }
            if !matches!(self.scheduler.schedule(cancel)?, crate::scheduler::Schedule::Anchored { plan: current } if current == *plan)
            {
                return Ok(None);
            }
            if !self.store.claim_schedule_occurrence(
                &plan.fingerprint()?,
                &due.to_string(),
                cancel,
            )? {
                return Ok(None);
            }
        }
        self.execute_batch(CheckInvocation::Scheduled, cancel, emit, None)
            .map(Some)
    }
    fn execute_batch(
        &self,
        invocation: CheckInvocation,
        cancel: &Cancel,
        mut emit: impl FnMut(Event),
        retry_of: Option<String>,
    ) -> Result<Report> {
        let config = self.store.config()?;
        let plan = match invocation {
            CheckInvocation::Manual => RunPlan::Manual,
            CheckInvocation::Startup => RunPlan::Workflow(&config.startup_actions),
            CheckInvocation::Scheduled => RunPlan::Workflow(&config.scheduled_actions),
        };
        let source = match invocation {
            CheckInvocation::Manual => crate::tasks::Source::Interactive,
            CheckInvocation::Startup => crate::tasks::Source::Startup,
            CheckInvocation::Scheduled => crate::tasks::Source::Scheduled,
        };
        let title = match invocation {
            CheckInvocation::Manual => i18n::tr!(TasksCheck).to_owned(),
            CheckInvocation::Startup => format!(
                "{} · {}",
                i18n::tr!(WorkflowStartup),
                config.startup_actions.summary()
            ),
            CheckInvocation::Scheduled => format!(
                "{} · {}",
                i18n::tr!(WorkflowScheduled),
                config.scheduled_actions.summary()
            ),
        };
        let mut run = crate::tasks::Run::begin(
            &self.store,
            crate::tasks::Spec {
                title,
                scope: crate::tasks::Scope::All,
                source,
                retry: match invocation {
                    CheckInvocation::Manual => crate::tasks::Retry::CheckAll,
                    CheckInvocation::Startup | CheckInvocation::Scheduled => {
                        crate::tasks::Retry::Unavailable
                    }
                },
                retry_of,
                operation: None,
            },
            cancel,
        )?;
        let mut done = 0;
        let reporter = run.reporter();
        let mut progress_error = None;
        let result = (|| {
            // 提前校验旧数据；执行期间只在写入单个结果时持有短事务锁。
            self.store.repository_reports()?;
            let total = config.repositories.len();
            let report = self.execute_checks(&config, plan, &reporter, cancel, &mut |event| {
                if let Event::Checked { .. } = &event {
                    done += 1;
                    if let Err(error) = run.progress(format!("{done}/{total}")) {
                        progress_error = Some(error);
                    }
                }
                emit(event);
            })?;
            if let Some(error) = progress_error {
                return Err(error);
            }
            cancel.check()?;
            self.store.save_report(&report)?;
            Ok(report)
        })();
        let mut outcome = check_outcome(&result, cancel);
        if result.is_err() && matches!(plan, RunPlan::Workflow(_)) {
            // 执行器取消或批次保存失败时，单仓库步骤已独立落盘，仍写入任务结果供追溯。
            let records = self.store.repository_reports()?;
            let details = records
                .values()
                .filter(|report| report.started_at >= run.record.started_at)
                .filter(|report| {
                    report
                        .execution
                        .as_ref()
                        .and_then(|execution| execution.workflow.as_ref())
                        == match plan {
                            RunPlan::Workflow(plan) => Some(plan),
                            RunPlan::Manual => None,
                        }
                })
                .map(workflow_detail)
                .collect::<Vec<_>>()
                .join("\n");
            match &mut outcome {
                crate::tasks::Outcome::Cancelled { detail }
                | crate::tasks::Outcome::Failed { detail, .. } => detail.push_str(&details),
                crate::tasks::Outcome::Succeeded { .. } => unreachable!("failed batch"),
            }
        }
        run.finish(outcome)?;
        result
    }
    pub fn check_repository(
        &self,
        path: &Path,
        cancel: &Cancel,
        emit: impl FnMut(Event),
    ) -> Result<Report> {
        self.check_repository_task(path, cancel, emit, None)
    }
    pub fn retry_check_repository(
        &self,
        path: &Path,
        cancel: &Cancel,
        emit: impl FnMut(Event),
        parent: String,
    ) -> Result<Report> {
        self.check_repository_task(path, cancel, emit, Some(parent))
    }
    fn check_repository_task(
        &self,
        path: &Path,
        cancel: &Cancel,
        mut emit: impl FnMut(Event),
        retry_of: Option<String>,
    ) -> Result<Report> {
        let run = crate::tasks::Run::begin(
            &self.store,
            crate::tasks::Spec {
                title: i18n::tr!(TasksCheck).into(),
                scope: crate::tasks::Scope::Repository {
                    path: path.to_owned(),
                },
                source: crate::tasks::Source::Interactive,
                retry: crate::tasks::Retry::CheckRepository {
                    path: path.to_owned(),
                },
                retry_of,
                operation: None,
            },
            cancel,
        )?;
        let result = (|| {
            self.store.repository_reports()?;
            emit(Event::Checking(path.to_owned()));
            let report = self.check_one(path, RunPlan::Manual, &run.reporter(), cancel)?;
            emit(Event::Checked {
                result: report.repositories[0].clone(),
                finished_at: report.finished_at.clone(),
                revision: report.repository_revisions[path],
            });
            Ok(report)
        })();
        run.finish(check_outcome(&result, cancel))?;
        result
    }

    fn check_one(
        &self,
        path: &Path,
        plan: RunPlan<'_>,
        reporter: &crate::tasks::Reporter,
        cancel: &Cancel,
    ) -> Result<Report> {
        let _repository = self.repository_lock(path, cancel)?;
        let started_at = state::now();
        let (result, timing) = match plan {
            RunPlan::Manual => check::repository(&self.git, path, PushPolicy::Disabled, cancel),
            RunPlan::Workflow(plan) => {
                crate::workflow::execute(&self.git, path, plan, cancel, |step| {
                    reporter.progress(format!("{}\n{}", path.display(), step.label()))
                })
            }
        };
        let refresh_cancel = Cancel::default();
        let save_cancel = match plan {
            RunPlan::Manual => {
                cancel.check()?;
                cancel
            }
            RunPlan::Workflow(_) => &refresh_cancel,
        };
        let mut report = single_report(result, timing, started_at, plan.policy());
        if let RunPlan::Workflow(plan) = plan {
            report
                .execution
                .as_mut()
                .expect("execution timing")
                .workflow = Some(plan.clone());
        }
        self.store
            .save_repository_report(&mut report, save_cancel)?;
        Ok(report)
    }
    fn execute_checks(
        &self,
        config: &state::Config,
        plan: RunPlan<'_>,
        reporter: &crate::tasks::Reporter,
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
            let mut report = single_report(result, timing, state::now(), plan.policy());
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
            |path| self.check_one(path, plan, reporter, cancel),
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
            push_policy: plan.policy(),
            notifications: notices,
            started_at,
            finished_at: state::now(),
            status: status(&repositories),
            repositories,
            classification_updated_at: None,
            execution: Some(Execution {
                workflow: match plan {
                    RunPlan::Manual => None,
                    RunPlan::Workflow(plan) => Some(plan.clone()),
                },
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
            workflow: None,
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
                        steps: vec![],
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

fn check_outcome(result: &Result<Report>, cancel: &Cancel) -> crate::tasks::Outcome {
    use crate::tasks::{Outcome, Resolution};
    let Ok(report) = result else {
        return Outcome::capture(result, cancel, |_| unreachable!());
    };
    let mut failures = Vec::new();
    for repository in &report.repositories {
        if let RepositoryResult::Failed { path, error, .. } = repository {
            failures.push(format!("{}: {error}", path.display()));
        }
        for publication in repository.pushes() {
            if let Publication::Failed { error, .. } = publication {
                failures.push(format!("{}: {error}", repository.path().display()));
            }
        }
    }
    if failures.is_empty() {
        Outcome::Succeeded {
            detail: {
                let stats = crate::domain::Stats::of(report);
                let summary = i18n::tf!(
                    CheckCompleteCleanAttentionFailed,
                    stats.clean,
                    stats.attention,
                    stats.failed
                );
                std::iter::once(summary)
                    .chain(report.repositories.iter().map(|repository| {
                        format!("{} · {}", repository.path().display(), repository.label())
                    }))
                    .collect::<Vec<_>>()
                    .join("\n")
                    + &workflow_detail(report)
            },
        }
    } else {
        Outcome::Failed {
            detail: failures.join("\n\n") + &workflow_detail(report),
            resolution: Resolution::Pending,
        }
    }
}

fn workflow_detail(report: &Report) -> String {
    use crate::workflow::StepOutcome;
    let Some(execution) = &report.execution else {
        return String::new();
    };
    let Some(plan) = &execution.workflow else {
        return String::new();
    };
    let mut lines = vec![String::new(), plan.summary()];
    for repository in &execution.repositories {
        lines.push(repository.path.display().to_string());
        for step in plan.steps() {
            let label = step.label();
            match repository.steps.iter().find(|record| record.step == *step) {
                Some(record) => {
                    let at = chrono::DateTime::parse_from_rfc3339(&record.finished_at)
                        .expect("recorded timestamp")
                        .with_timezone(&chrono::Local)
                        .format("%H:%M:%S")
                        .to_string();
                    lines.push(match &record.outcome {
                        StepOutcome::Succeeded { detail } => format!("✓ {at} {label} · {detail}"),
                        StepOutcome::Skipped { reason } => {
                            format!("– {at} {label} · {}", reason.label())
                        }
                        StepOutcome::Failed { error } => format!("✗ {at} {label}\n  {error}"),
                    });
                }
                None => lines.push(format!("– {label} · {}", i18n::tr!(WorkflowStopped))),
            }
        }
    }
    format!("\n{}", lines.join("\n"))
}
