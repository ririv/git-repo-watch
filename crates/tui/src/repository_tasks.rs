//! Repository-scoped background work; completion never takes over a different view.
use super::*;
use chrono::DateTime;
use std::path::Path;

#[derive(Debug)]
pub(super) struct Task {
    pub title: String,
    pub animation: spinner::Spinner,
    effect: Effect,
    pub(super) cancel: Cancel,
    revision: u64,
    page: Page,
    scope: Option<PathBuf>,
    input: String,
    rx: mpsc::Receiver<Completion>,
    pub(super) handle: thread::JoinHandle<()>,
}
#[derive(Debug)]
struct Completion {
    outcome: Result<Outcome>,
    refresh: Refresh,
}
#[derive(Debug)]
enum Refresh {
    Unchanged,
    Updated(Box<domain::Report>),
    Failed(anyhow::Error),
}
impl Screen {
    pub(super) fn start_repository(
        &mut self,
        path: PathBuf,
        title: &str,
        effect: Effect,
        task: impl FnOnce(Arc<Service>, Cancel, mpsc::Sender<WorkerEvent>) -> Result<Outcome>
        + Send
        + 'static,
    ) {
        if self.repository_tasks.contains_key(&path) {
            self.message = i18n::tr!(RepositoryTaskBusy).into();
            return;
        }
        if self.repository_tasks.len() >= self.config.max_concurrency {
            self.message = i18n::tr!(RepositoryTaskLimit).into();
            return;
        }
        let service = self.service.clone();
        let task_path = path.clone();
        let cancel = Cancel::default();
        let task_cancel = cancel.clone();
        let language = i18n::language();
        let (tx, rx) = mpsc::channel();
        let handle = thread::spawn(move || {
            let _language = i18n::using(language);
            let (progress, _) = mpsc::channel();
            let outcome =
                task(service.clone(), task_cancel, progress).and_then(|outcome| match outcome {
                    Outcome::Published(Publication::Failed { phase, error, .. }) => Err(
                        anyhow::anyhow!(i18n::tf!(PhasePhaseError, phase = phase, error = error)),
                    ),
                    outcome => Ok(outcome),
                });
            let refresh = match &outcome {
                Ok(Outcome::Checked(report)) => Refresh::Updated(Box::new(report.clone())),
                _ => match effect {
                    Effect::IndexWrite | Effect::Commit | Effect::Push => {
                        match service.store.repository_report(&task_path) {
                            Ok(report) => match report {
                                Some(report) => Refresh::Updated(Box::new(report)),
                                None => Refresh::Unchanged,
                            },
                            Err(error) => Refresh::Failed(error),
                        }
                    }
                    Effect::ReadOnly | Effect::Check | Effect::Configuration => Refresh::Unchanged,
                },
            };
            let _ = tx.send(Completion { outcome, refresh });
        });
        self.repository_tasks.insert(
            path,
            Task {
                title: title.into(),
                animation: spinner::Spinner::new(std::time::Instant::now()),
                effect,
                cancel,
                revision: self.view_revision,
                page: self.page,
                scope: self.scope_path(),
                input: self.bar.editor().text.clone(),
                rx,
                handle,
            },
        );
        self.message.clear();
    }
    pub(super) fn poll_repository_tasks(&mut self, root: &Cancel) -> bool {
        let ready = self
            .repository_tasks
            .iter()
            .filter_map(|(path, task)| match task.rx.try_recv() {
                Ok(completion) => Some((path.clone(), completion)),
                Err(mpsc::TryRecvError::Empty) => None,
                Err(mpsc::TryRecvError::Disconnected) => Some((
                    path.clone(),
                    Completion {
                        outcome: Err(anyhow::anyhow!(i18n::tr!(
                            BackgroundWorkerTerminatedUnexpectedly
                        ))),
                        refresh: Refresh::Unchanged,
                    },
                )),
            })
            .collect::<Vec<_>>();
        let changed = !ready.is_empty();
        for (path, mut completion) in ready {
            let task = self
                .repository_tasks
                .remove(&path)
                .expect("completed repository task");
            if task.handle.join().is_err() {
                completion.outcome = Err(anyhow::anyhow!(i18n::tr!(BackgroundWorkerPanicked)));
            }
            let selected = self.selected_path();
            match completion.refresh {
                Refresh::Updated(report) => {
                    let finished_at = DateTime::parse_from_rfc3339(&report.finished_at)
                        .expect("validated service timestamp");
                    if let Some(result) = report.repositories.into_iter().next() {
                        self.merge_update(
                            path.clone(),
                            checks::Update {
                                result,
                                finished_at,
                                revision: report.repository_revisions[&path],
                            },
                        );
                    }
                }
                Refresh::Failed(error) => {
                    completion.outcome = match completion.outcome {
                        Ok(_) => Err(error),
                        Err(operation) => Err(operation.context(format!(
                            "{}: {error:#}",
                            i18n::tr!(RepositoryStateSaveFailed)
                        ))),
                    };
                }
                Refresh::Unchanged => {}
            }
            self.focus(selected);
            let same_view = task.page == self.page
                && task.revision == self.view_revision
                && self.scope_path() == task.scope
                && self.bar.editor().text == task.input;
            if matches!(
                task.effect,
                Effect::IndexWrite | Effect::Commit | Effect::Push | Effect::Check
            ) {
                self.refresh_repository_panes(&path);
            }
            if !same_view && let Ok(Outcome::Files(snapshot)) = &completion.outcome {
                match &mut self.mode {
                    Mode::Changes(session) if session.files.snapshot.path == snapshot.path => {
                        session.refreshed(snapshot.clone())
                    }
                    Mode::Files(session) if session.snapshot.path == snapshot.path => {
                        *session = session.refreshed(snapshot.clone())
                    }
                    _ => {}
                }
            }
            match completion.outcome {
                Ok(outcome) => {
                    self.repository_errors.remove(&path);
                    if same_view {
                        let origin = self.take_mode();
                        self.finish(outcome, origin, root);
                    } else {
                        self.message =
                            i18n::tf!(RepositoryTaskCompleted, view::name(&path), task.title);
                    }
                }
                Err(error) => {
                    if task.effect == Effect::Check
                        && error.downcast_ref::<engine::cancel::Cancelled>().is_some()
                    {
                        self.message = i18n::tr!(CheckCancelledRetained).into();
                        continue;
                    }
                    let detail = if matches!(
                        task.effect,
                        Effect::IndexWrite | Effect::Commit | Effect::Push
                    ) && task.cancel.is_cancelled()
                    {
                        i18n::tf!(OperationInterruptedSomeWritesMayHaveCompleted, e = error)
                    } else {
                        format!("{error:#}")
                    };
                    self.repository_errors.insert(path.clone(), detail.clone());
                    if same_view {
                        let origin = Box::new(self.take_mode());
                        self.mode = Mode::Error {
                            origin,
                            title: task.title,
                            detail,
                            scroll: 0,
                        };
                    } else {
                        self.message =
                            format!("{} · {}: {}", view::name(&path), task.title, detail);
                    }
                }
            }
        }
        changed
    }
    pub(super) fn refresh_repository_panes(&mut self, path: &Path) {
        if self.page != Page::Detail || self.workspace.path() != Some(path) {
            return;
        }
        // 只刷新发生变更的仓库；不跳转其他仓库，也不覆盖正在编辑的提交消息。
        self.refresh_workspace();
    }
    pub(super) fn cancel_repository_task(&mut self) -> bool {
        let Some(path) = self.scope_path() else {
            return false;
        };
        let Some(task) = self.repository_tasks.get(&path) else {
            return false;
        };
        task.cancel.cancel();
        self.message = i18n::tr!(Cancelling).into();
        true
    }
}
