//! Repository-scoped background work; completion never takes over a different view.
use super::*;
use chrono::DateTime;
use engine::tasks::progress::{Recorder, Stage};
use std::path::Path;

pub(crate) struct Progress {
    reporter: Option<engine::tasks::Reporter>,
    updates: mpsc::Sender<Stage>,
    measurements: Recorder,
}
impl Progress {
    pub fn enter(&mut self, stage: Stage) -> Result<()> {
        self.measurements.enter(stage);
        let _ = self.updates.send(stage);
        if let Some(reporter) = &self.reporter {
            reporter.progress(stage.label().into())?;
        }
        Ok(())
    }
}

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
    progress: Option<Stage>,
    progress_updates: mpsc::Receiver<Stage>,
    rx: mpsc::Receiver<Completion>,
    pub(super) handle: thread::JoinHandle<()>,
}
impl Task {
    pub(crate) fn label(&self) -> &str {
        match self.progress {
            Some(stage) => stage.label(),
            None => &self.title,
        }
    }
}
#[derive(Debug)]
struct Completion {
    outcome: Result<Outcome>,
    replay: Option<engine::operations::replay::Observation>,
    refresh: Refresh,
    view: Result<crate::mutations::readback::Readback>,
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
        task: impl FnOnce(Arc<Service>, Cancel, &mut Progress) -> Result<Outcome> + Send + 'static,
    ) {
        let spec =
            crate::task_center::spec(title, Some(path.clone()), engine::tasks::Retry::Unavailable);
        self.start_repository_tracked(path, title, effect, spec, task);
    }
    pub(super) fn start_repository_tracked(
        &mut self,
        path: PathBuf,
        title: &str,
        effect: Effect,
        spec: engine::tasks::Spec,
        task: impl FnOnce(Arc<Service>, Cancel, &mut Progress) -> Result<Outcome> + Send + 'static,
    ) {
        if self.repository_tasks.contains_key(&path) {
            self.message = crate::message::Message::error(i18n::tr!(RepositoryTaskBusy).into());
            return;
        }
        if self.repository_tasks.len() >= self.config.max_concurrency {
            self.message = crate::message::Message::error(i18n::tr!(RepositoryTaskLimit).into());
            return;
        }
        let requested = if effect == Effect::RepositoryWrite {
            crate::mutations::readback::Requested::for_mode(&self.mode)
        } else {
            crate::mutations::readback::Requested::None
        };
        let service = self.service.clone();
        let task_path = path.clone();
        let cancel = Cancel::default();
        let task_cancel = cancel.clone();
        let language = i18n::language();
        let (tx, rx) = mpsc::channel();
        let (progress_updates, progress_rx) = mpsc::channel();
        let handle = thread::spawn(move || {
            let _language = i18n::using(language);
            let run = if effect == Effect::Check {
                Ok(None)
            } else {
                engine::tasks::Run::begin(&service.store, spec.clone(), &task_cancel).map(Some)
            };
            let run = match run {
                Ok(run) => run,
                Err(error) => {
                    let _ = tx.send(Completion {
                        outcome: Err(error),
                        replay: None,
                        refresh: Refresh::Unchanged,
                        view: Ok(crate::mutations::readback::Readback::Unchanged),
                    });
                    return;
                }
            };
            let mut progress = Progress {
                reporter: run.as_ref().map(engine::tasks::Run::reporter),
                updates: progress_updates,
                measurements: Recorder::default(),
            };
            let mut outcome =
                task(service.clone(), task_cancel.clone(), &mut progress).and_then(|outcome| {
                    match outcome {
                        Outcome::Published(Publication::Failed { phase, error, .. }) => {
                            Err(anyhow::anyhow!(i18n::tf!(
                                PhasePhaseError,
                                phase = phase,
                                error = error
                            )))
                        }
                        outcome => Ok(outcome),
                    }
                });
            let refresh = match &outcome {
                Ok(Outcome::Checked(report)) => Refresh::Updated(Box::new(report.clone())),
                _ => match effect {
                    Effect::RepositoryWrite
                    | Effect::IndexWrite
                    | Effect::Commit
                    | Effect::Push => match service.store.repository_report(&task_path) {
                        Ok(report) => match report {
                            Some(report) => Refresh::Updated(Box::new(report)),
                            None => Refresh::Unchanged,
                        },
                        Err(error) => Refresh::Failed(error),
                    },
                    Effect::ReadOnly | Effect::Terminal | Effect::Check | Effect::Configuration => {
                        Refresh::Unchanged
                    }
                },
            };
            // 单仓库获取即使失败也会返回状态报告；先保留报告更新，再按错误结束界面任务。
            if let Ok(Outcome::Checked(report)) = &outcome
                && let Some(engine::domain::RepositoryResult::Failed { error, .. }) =
                    report.repositories.first()
            {
                outcome = Err(anyhow::anyhow!(error.clone()));
            }
            let view = if progress.measurements.is_empty()
                || matches!(requested, crate::mutations::readback::Requested::None)
            {
                requested.read(&service, &task_path)
            } else {
                let notified = progress.enter(Stage::ReadingView);
                let read = requested.read(&service, &task_path);
                match (read, notified) {
                    (result, Ok(())) => result,
                    (Ok(_), Err(error)) => Err(error),
                    (Err(read), Err(progress)) => Err(read.context(progress)),
                }
            };
            let replay = if effect == Effect::Terminal {
                None
            } else {
                match service.replay_state(&task_path, &Cancel::default()) {
                    Ok(observation) => Some(observation),
                    Err(error) => {
                        outcome = match outcome {
                            Ok(_) => Err(error),
                            Err(operation) => Err(operation.context(error)),
                        };
                        None
                    }
                }
            };
            let timings = progress.measurements.finish();
            if let Some(run) = run {
                let completion = match (&outcome, &refresh, &view) {
                    (_, Refresh::Failed(error), _) | (_, _, Err(error)) => {
                        engine::tasks::Outcome::Failed {
                            detail: format!("{error:#}"),
                            resolution: engine::tasks::Resolution::Pending,
                        }
                    }
                    _ => crate::task_center::outcome(&outcome, &task_cancel, &spec.title, effect),
                };
                if let Err(error) = run.finish_with_timings(completion, timings) {
                    outcome = Err(error);
                }
            }
            let _ = tx.send(Completion {
                outcome,
                replay,
                refresh,
                view,
            });
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
                input: self.bar.text(),
                progress: None,
                progress_updates: progress_rx,
                rx,
                handle,
            },
        );
        self.message.clear();
    }
    pub(super) fn poll_repository_tasks(&mut self, root: &Cancel) -> bool {
        let mut changed = false;
        for task in self.repository_tasks.values_mut() {
            for stage in task.progress_updates.try_iter() {
                task.progress = Some(stage);
                changed = true;
            }
        }
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
                        replay: None,
                        refresh: Refresh::Unchanged,
                        view: Ok(crate::mutations::readback::Readback::Unchanged),
                    },
                )),
            })
            .collect::<Vec<_>>();
        changed |= !ready.is_empty();
        for (path, mut completion) in ready {
            let task = self
                .repository_tasks
                .remove(&path)
                .expect("completed repository task");
            if task.effect == Effect::Check {
                self.note_fetch_finished(&path);
            }
            if task.handle.join().is_err() {
                completion.outcome = Err(anyhow::anyhow!(i18n::tr!(BackgroundWorkerPanicked)));
            }
            if let Some(observation) = completion.replay {
                self.replay.observe(path.clone(), observation);
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
            match completion.view {
                Ok(view) => self.merge_mutation_view(view),
                Err(error) => {
                    completion.outcome =
                        match completion.outcome {
                            Ok(_) => Err(error),
                            Err(operation) => Err(operation
                                .context(format!("{}: {error:#}", i18n::tr!(RefreshFailed)))),
                        }
                }
            }
            self.focus(selected);
            // 提交中的说明只读；忽略编辑按键或切换操作焦点不应阻止该任务收尾。
            let submitting_here = task.effect == Effect::Commit
                && match &self.mode {
                    Mode::CommitMessage { plan, editor, .. } => {
                        plan.path == path && editor.submitting()
                    }
                    Mode::Composition(session) => {
                        session.prepared.snapshot.path == path
                            && matches!(session.state, composition::State::Submitting)
                    }
                    _ => false,
                };
            let waiting_here = matches!(&self.mode, Mode::CompositionWaiting(waiting) if waiting.path == path)
                && task.effect == Effect::ReadOnly;
            let publishing_here = matches!(&self.mode, Mode::PushTarget(session) if session.request.path == path && !session.editing())
                && task.effect == Effect::Push;
            let same_view = submitting_here
                || waiting_here
                || publishing_here
                || (task.page == self.page
                    && task.revision == self.view_revision
                    && self.scope_path() == task.scope
                    && self.bar.text() == task.input);
            if matches!(
                task.effect,
                Effect::RepositoryWrite
                    | Effect::IndexWrite
                    | Effect::Commit
                    | Effect::Push
                    | Effect::Check
            ) {
                self.refresh_repository_panes(&path);
            }
            if !same_view && let Ok(Outcome::Files(snapshot)) = &completion.outcome {
                self.merge_mutation_view(crate::mutations::readback::Readback::Files(
                    snapshot.clone(),
                ));
            }
            match completion.outcome {
                Ok(outcome) => {
                    self.repository_errors.remove(&path);
                    if !same_view
                        && self.config.ai.enabled
                        && let Outcome::Composed(prepared) = &outcome
                    {
                        if let Some(origin) =
                            composition::waiting::complete(&mut self.mode, &path, prepared)
                        {
                            self.workspace.save(origin);
                        }
                        for visit in &mut self.history {
                            if let Some(origin) = visit.complete_composition(&path, prepared) {
                                self.workspace.save(origin);
                            }
                        }
                    }
                    if same_view {
                        let origin = self.take_mode();
                        self.finish(outcome, origin, root);
                    } else if let Outcome::PushPrepared(preparation) = outcome {
                        self.message = crate::message::Message::information(
                            crate::push::preparation_notice(&preparation).into(),
                        );
                    } else {
                        self.message = crate::message::Message::information(i18n::tf!(
                            RepositoryTaskCompleted,
                            view::name(&path),
                            task.title
                        ));
                    }
                }
                Err(error) => {
                    if task.effect == Effect::Check
                        && error.downcast_ref::<engine::cancel::Cancelled>().is_some()
                    {
                        self.message = crate::message::Message::information(
                            i18n::tr!(CheckCancelledRetained).into(),
                        );
                        continue;
                    }
                    let detail = if matches!(
                        task.effect,
                        Effect::RepositoryWrite
                            | Effect::IndexWrite
                            | Effect::Commit
                            | Effect::Push
                    ) && task.cancel.is_cancelled()
                    {
                        i18n::tf!(OperationInterruptedSomeWritesMayHaveCompleted, e = error)
                    } else {
                        format!("{error:#}")
                    };
                    self.repository_errors.insert(path.clone(), detail.clone());
                    composition::waiting::fail(&mut self.mode, &path, &detail);
                    for visit in &mut self.history {
                        visit.fail_composition(&path, &detail);
                    }
                    if waiting_here {
                        self.message.clear();
                        self.sync_bar();
                        continue;
                    }
                    if same_view {
                        if task.effect == Effect::RepositoryWrite
                            && matches!(
                                self.replay_state(&path),
                                Some(engine::operations::replay::State::Active(_))
                            )
                        {
                            let message = match self.replay_state(&path) {
                                Some(engine::operations::replay::State::Active(active))
                                    if !active.conflicts.is_empty() =>
                                {
                                    i18n::tr!(ReplayResolveFirst)
                                }
                                _ => i18n::tr!(ReplayPaused),
                            };
                            self.open_view(crate::navigation::View::Diff, path.clone());
                            self.message = crate::message::Message::error(message.into());
                            continue;
                        }
                        let mut origin = self.take_mode();
                        if let Mode::Composition(session) = &mut origin {
                            session.restore();
                        }
                        if let Mode::CommitMessage { editor, .. } = &mut origin {
                            editor.restore();
                        }
                        if let Mode::PushTarget(session) = &mut origin {
                            session.restore();
                        }
                        self.mode = Mode::Error {
                            origin: Box::new(origin),
                            title: task.title,
                            detail,
                            scroll: 0,
                        };
                    } else {
                        self.message = crate::message::Message::error(format!(
                            "{} · {}: {}",
                            view::name(&path),
                            task.title,
                            detail
                        ));
                    }
                }
            }
        }
        changed
    }
    pub(super) fn refresh_repository_panes(&mut self, path: &Path) {
        if let Mode::Browser(session) = &mut self.mode
            && session.root == path
        {
            session.refresh();
        }
        for visit in &mut self.history {
            visit.refresh_browser(path);
        }
        if self.workspace.path() != Some(path) {
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
        self.message = crate::message::Message::information(i18n::tr!(Cancelling).into());
        true
    }
}
