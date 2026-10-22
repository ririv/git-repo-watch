//! Background repository checks independent of the current interaction mode.
use super::*;
use crate::spinner::Spinner;
use chrono::{DateTime, FixedOffset};
use std::{collections::BTreeMap, time::Instant};
#[derive(Debug, Clone)]
pub(crate) enum CheckTarget {
    All,
    Repository(PathBuf),
}
#[derive(Debug, Clone)]
pub(crate) struct Update {
    pub result: RepositoryResult,
    pub finished_at: DateTime<FixedOffset>,
    pub revision: u64,
}
impl Update {
    fn order(&self) -> (u64, DateTime<FixedOffset>) {
        (self.revision, self.finished_at)
    }
}
#[derive(Debug)]
enum Progress {
    Queued,
    Checking,
    Finished(Update),
}
#[derive(Debug)]
enum Event {
    Progress(service::Event),
    Done(Result<domain::Report>),
}
#[derive(Debug)]
pub(crate) struct CheckTask {
    progress: BTreeMap<PathBuf, Progress>,
    cancel: Cancel,
    state: TaskState,
    rx: mpsc::Receiver<Event>,
    handle: thread::JoinHandle<()>,
    animation: Spinner,
}
impl CheckTask {
    pub(crate) fn spinner(&self) -> &'static str {
        self.animation.symbol()
    }
    pub(crate) fn checking(&self, path: &std::path::Path) -> bool {
        matches!(self.progress.get(path), Some(Progress::Checking))
    }
    pub(crate) fn update(&self, path: &std::path::Path) -> Option<&Update> {
        match self.progress.get(path) {
            Some(Progress::Finished(update)) => Some(update),
            _ => None,
        }
    }
    pub(crate) fn summary(&self) -> String {
        let done = self
            .progress
            .values()
            .filter(|p| matches!(p, Progress::Finished(_)))
            .count();
        match self.state {
            TaskState::Running => i18n::tf!(
                BackgroundCheckProgress,
                self.spinner(),
                done,
                self.progress.len()
            ),
            TaskState::Cancelling => i18n::tf!(
                BackgroundCheckCancelling,
                self.spinner(),
                done,
                self.progress.len()
            ),
        }
    }
}
impl Screen {
    pub(crate) fn merge_update(&mut self, path: PathBuf, update: Update) {
        if self
            .updates
            .get(&path)
            .is_none_or(|old| update.order() > old.order())
        {
            self.updates.insert(path, update);
        }
    }
    pub(crate) fn latest_update(&self, path: &std::path::Path) -> Option<&Update> {
        [
            self.check_task.as_ref().and_then(|task| task.update(path)),
            self.updates.get(path),
        ]
        .into_iter()
        .flatten()
        .max_by_key(|update| update.order())
    }
    pub(crate) fn result_for(&self, path: &std::path::Path) -> Option<&RepositoryResult> {
        if let Some(update) = self.latest_update(path) {
            return Some(&update.result);
        }
        match &self.snapshot.report {
            service::CachedReport::Available { report, .. } => report
                .repositories
                .iter()
                .find(|result| result.path() == path),
            service::CachedReport::Missing => None,
        }
    }
    pub(crate) fn load_updates(&mut self) -> Result<()> {
        let batch_versions = match &self.snapshot.report {
            service::CachedReport::Missing => BTreeMap::new(),
            service::CachedReport::Available { report, .. } => report
                .repositories
                .iter()
                .map(|result| {
                    let path = result.path();
                    let at = report
                        .repository_finished_at
                        .get(path)
                        .unwrap_or(&report.finished_at);
                    Ok((
                        path.to_owned(),
                        (
                            report.repository_revisions.get(path).copied().unwrap_or(0),
                            DateTime::parse_from_rfc3339(at)?,
                        ),
                    ))
                })
                .collect::<Result<BTreeMap<_, _>>>()?,
        };
        self.updates.retain(|path, update| {
            self.config.repositories.contains(path)
                && batch_versions
                    .get(path)
                    .is_none_or(|version| update.order() > *version)
        });
        for (path, report) in self.service.store.repository_reports()? {
            let revision = report.repository_revisions.get(&path).copied().unwrap_or(0);
            let finished_at = DateTime::parse_from_rfc3339(&report.finished_at)
                .context(i18n::tr!(InvalidRepositoryCheckReport))?;
            let update = Update {
                revision,
                finished_at,
                result: report
                    .repositories
                    .into_iter()
                    .next()
                    .expect("validated single-repository report"),
            };
            if self.config.repositories.contains(&path)
                && batch_versions
                    .get(&path)
                    .is_none_or(|version| update.order() > *version)
            {
                self.merge_update(path, update);
            }
        }
        Ok(())
    }
    pub(crate) fn check_on_startup(&mut self) {
        if !self.config.check_on_startup || self.config.repositories.is_empty() {
            return;
        }
        if self.check_task.is_some() || matches!(self.snapshot.activity, service::Activity::Running)
        {
            self.message = i18n::tr!(StartupCheckAlreadyRunning).into();
            return;
        }
        self.start_check(CheckTarget::All);
    }
    pub(crate) fn start_check(&mut self, target: CheckTarget) {
        if let CheckTarget::Repository(path) = target {
            self.start_repository(
                path.clone(),
                i18n::tr!(CheckingAlt),
                Effect::Check,
                move |service, cancel, _| {
                    Ok(Outcome::Checked(service.check_repository(
                        &path,
                        &cancel,
                        |_| {},
                    )?))
                },
            );
            return;
        }
        if self.check_task.is_some() {
            self.message = i18n::tr!(CheckAlreadyRunning).into();
            return;
        }
        let paths = match &target {
            CheckTarget::All => self.config.repositories.clone(),
            CheckTarget::Repository(path) => vec![path.clone()],
        };
        let progress = paths
            .into_iter()
            .map(|path| (path, Progress::Queued))
            .collect();
        let cancel = Cancel::default();
        let worker_cancel = cancel.clone();
        let service = self.service.clone();
        let (tx, rx) = mpsc::channel();
        let language = i18n::language();
        let handle = thread::spawn(move || {
            let _language = i18n::using(language);
            let emit = |event| {
                let _ = tx.send(Event::Progress(event));
            };
            let result = service.run(&worker_cancel, emit);
            let _ = tx.send(Event::Done(result));
        });
        self.check_task = Some(CheckTask {
            progress,
            cancel,
            state: TaskState::Running,
            rx,
            handle,
            animation: Spinner::new(Instant::now()),
        });
        self.message.clear();
    }
    pub(crate) fn cancel_check(&mut self) {
        match &mut self.check_task {
            Some(task) => {
                task.cancel.cancel();
                task.state = TaskState::Cancelling;
                self.message = i18n::tr!(CheckCancelRequested).into();
            }
            None => self.message = i18n::tr!(NoBackgroundCheck).into(),
        }
    }
    pub(crate) fn animate_check(&mut self) -> bool {
        let Some(task) = &mut self.check_task else {
            return false;
        };
        task.animation.tick(Instant::now())
    }
    pub(crate) fn poll_check(&mut self) -> bool {
        let path = self.selected_path();
        let mut done = None;
        let mut changed = false;
        if let Some(task) = &mut self.check_task {
            loop {
                match task.rx.try_recv() {
                    Ok(Event::Progress(service::Event::Checking(path))) => {
                        task.progress.insert(path, Progress::Checking);
                        changed = true;
                    }
                    Ok(Event::Progress(service::Event::Checked {
                        result,
                        finished_at,
                        revision,
                    })) => {
                        if self.page == Page::Detail
                            && self.overview.content.path() == Some(result.path())
                        {
                            self.overview.open(result.path().to_owned(), &self.service);
                        }
                        task.progress.insert(
                            result.path().to_owned(),
                            Progress::Finished(Update {
                                result,
                                revision,
                                finished_at: DateTime::parse_from_rfc3339(&finished_at)
                                    .expect("service timestamp"),
                            }),
                        );
                        changed = true;
                    }
                    Ok(Event::Progress(service::Event::NotificationProblem(notice))) => {
                        self.message = i18n::tf!(SystemNotificationFailedNotice, notice = notice);
                        changed = true;
                    }
                    Ok(Event::Done(result)) => {
                        done = Some(result);
                        break;
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        done = Some(Err(anyhow::anyhow!(i18n::tr!(
                            BackgroundWorkerTerminatedUnexpectedly
                        ))));
                        break;
                    }
                }
            }
        }
        if let Some(result) = done {
            let task = self.check_task.take().expect("completed check task exists");
            for (path, progress) in task.progress {
                if let Progress::Finished(update) = progress {
                    self.merge_update(path, update);
                }
            }
            let result = if task.handle.join().is_err() {
                Err(anyhow::anyhow!(i18n::tr!(BackgroundWorkerPanicked)))
            } else {
                result
            };
            match result {
                Ok(report) => {
                    let stats = domain::Stats::of(&report);
                    let scope = if report.repositories.len() == self.config.repositories.len()
                        && report.repositories.iter().all(|result| {
                            self.config.repositories.contains(&result.path().to_owned())
                        }) {
                        service::Scope::Current
                    } else {
                        service::Scope::Outdated
                    };
                    self.snapshot.report = service::CachedReport::Available {
                        report: Box::new(report),
                        scope,
                    };
                    self.message = i18n::tf!(
                        CheckCompleteCleanAttentionFailed,
                        stats.clean,
                        stats.attention,
                        stats.failed
                    );
                }
                Err(error) if error.downcast_ref::<engine::cancel::Cancelled>().is_some() => {
                    self.message = i18n::tr!(CheckCancelledRetained).into();
                }
                Err(error) => self.message = i18n::tf!(ErrorE, e = format!("{error:#}")),
            }
            match self.service.store.running() {
                Ok(running) => {
                    self.snapshot.activity = if running {
                        service::Activity::Running
                    } else {
                        service::Activity::Idle
                    }
                }
                Err(error) => self.message.push_str(&i18n::tf!(RefreshFailedE, e = error)),
            }
            changed = true;
        }
        if changed {
            self.focus(path);
        }
        changed
    }
    pub(crate) fn shutdown_check(&mut self) {
        if let Some(task) = self.check_task.take() {
            task.cancel.cancel();
            let _ = task.handle.join();
        }
    }
}
