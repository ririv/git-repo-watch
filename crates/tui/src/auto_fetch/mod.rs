//! Focus-aware automatic fetches reuse repository services without owning the input UI.
mod clock;
use crate::{Mode, Screen, checks::Update, live::WindowFocus};
use clock::Clock;
use engine::{cancel::Cancel, domain::Report, tasks::Scope};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    thread,
    time::Instant,
};

#[derive(Default)]
pub(crate) struct Controller {
    clock: Clock,
    workers: BTreeMap<PathBuf, Worker>,
}
struct Worker {
    cancel: Cancel,
    handle: thread::JoinHandle<anyhow::Result<Report>>,
    order: Option<(u64, chrono::DateTime<chrono::FixedOffset>)>,
}
impl Controller {
    pub fn running(&self, path: &Path) -> bool {
        self.workers.contains_key(path)
    }
    pub fn cancel_all(&self) {
        for worker in self.workers.values() {
            worker.cancel.cancel();
        }
    }
    pub fn shutdown(&mut self) {
        self.cancel_all();
        for (_, worker) in std::mem::take(&mut self.workers) {
            let _ = worker.handle.join();
        }
    }
}
impl Drop for Controller {
    fn drop(&mut self) {
        self.shutdown();
    }
}

impl Screen {
    fn configure_auto_fetch(&mut self) {
        if !self.config.auto_fetch.enabled {
            self.auto_fetch
                .clock
                .configure(false, self.config.auto_fetch.interval_seconds, &[]);
            return;
        }
        let mut paths = self.config.repositories.clone();
        paths.extend(self.service.session_repository().map(Path::to_owned));
        paths.sort();
        paths.dedup();
        self.auto_fetch.clock.configure(
            self.config.auto_fetch.enabled,
            self.config.auto_fetch.interval_seconds,
            &paths,
        );
        // 重新启用时沿用尚在执行的获取，不另起同仓库任务。
        for (path, worker) in &self.auto_fetch.workers {
            if paths.contains(path) {
                self.auto_fetch.clock.started(path);
            } else {
                worker.cancel.cancel();
            }
        }
    }
    pub(crate) fn note_fetch_started(&mut self, paths: &[PathBuf]) {
        self.configure_auto_fetch();
        for path in paths {
            self.auto_fetch.clock.started(path);
        }
    }
    pub(crate) fn note_fetch_finished(&mut self, path: &Path) {
        self.configure_auto_fetch();
        self.auto_fetch.clock.finished(path, Instant::now());
    }
    pub(crate) fn cancel_auto_fetch(&mut self) -> bool {
        if let Some(path) = self.scope_path()
            && let Some(worker) = self.auto_fetch.workers.get(&path)
        {
            worker.cancel.cancel();
            return true;
        }
        false
    }
    pub(crate) fn poll_auto_fetch(&mut self) -> bool {
        self.poll_auto_fetch_at(Instant::now())
    }
    pub(crate) fn poll_auto_fetch_at(&mut self, now: Instant) -> bool {
        self.configure_auto_fetch();
        if !self.config.auto_fetch.enabled && self.auto_fetch.workers.is_empty() {
            return false;
        }
        let selected = self.selected_path();
        let completed = self
            .auto_fetch
            .workers
            .iter()
            .filter(|(_, worker)| worker.handle.is_finished())
            .map(|(path, _)| path.clone())
            .collect::<Vec<_>>();
        let changed = !completed.is_empty();
        for path in completed {
            let worker = self
                .auto_fetch
                .workers
                .remove(&path)
                .expect("completed automatic fetch");
            let result = worker
                .handle
                .join()
                .unwrap_or_else(|_| Err(anyhow::anyhow!(i18n::tr!(BackgroundWorkerPanicked))));
            self.auto_fetch.clock.finished(&path, now);
            match result {
                Ok(report) => {
                    if let Some(result) = report.repositories.into_iter().next() {
                        self.merge_update(
                            path.clone(),
                            Update {
                                result,
                                finished_at: chrono::DateTime::parse_from_rfc3339(
                                    &report.finished_at,
                                )
                                .expect("service timestamp"),
                                revision: report.repository_revisions[&path],
                            },
                        );
                    }
                }
                Err(_) if worker.cancel.is_cancelled() => {}
                Err(error) => {
                    if worker.order == self.observation_order(&path) {
                        self.repository_errors.insert(path, format!("{error:#}"));
                    }
                }
            }
        }
        if changed {
            self.focus(selected);
        }
        if self.window_focus == WindowFocus::Unfocused || matches!(self.mode, Mode::Busy(_)) {
            return changed;
        }
        let available = self
            .config
            .max_concurrency
            .saturating_sub(self.auto_fetch.workers.len() + self.repository_tasks.len());
        let ready = self
            .auto_fetch
            .clock
            .due(now)
            .into_iter()
            .filter(|path| {
                !self.auto_fetch.workers.contains_key(path)
                    && !self.repository_tasks.contains_key(path)
                    && !self.live.is_reading(path)
                    && !self
                        .check_task
                        .as_ref()
                        .is_some_and(|task| task.fetching(path))
                    && !(self.workspace.path() == Some(path.as_path()) && self.workspace.loading())
                    && !self.tasks.records.iter().any(|task| {
                        task.active()
                            && match &task.spec.scope {
                                Scope::All => true,
                                Scope::Repository { path: active } => active == path,
                                Scope::Application => false,
                            }
                    })
            })
            .take(available)
            .collect::<Vec<_>>();
        for path in ready {
            let cancel = Cancel::default();
            let worker_cancel = cancel.clone();
            let service = self.service.clone();
            let worker_path = path.clone();
            let language = i18n::language();
            let order = self.observation_order(&path);
            let handle = thread::spawn(move || {
                let _language = i18n::using(language);
                service.fetch_automatically(&worker_path, &worker_cancel)
            });
            self.auto_fetch.clock.started(&path);
            self.auto_fetch.workers.insert(
                path,
                Worker {
                    cancel,
                    handle,
                    order,
                },
            );
        }
        changed
    }
}
