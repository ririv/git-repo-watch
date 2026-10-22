//! Background execution, progress, cancellation, and result transitions.
use super::*;
impl Screen {
    pub(super) fn start(
        &mut self,
        title: &str,
        effect: Effect,
        task: impl FnOnce(Arc<Service>, Cancel, mpsc::Sender<WorkerEvent>) -> Result<Outcome>
        + Send
        + 'static,
    ) {
        let origin = Box::new(self.take_mode());
        let cancel = Cancel::default();
        let task_cancel = cancel.clone();
        let service = self.service.clone();
        let (tx, rx) = mpsc::channel();
        let language = i18n::language();
        let handle = thread::spawn(move || {
            let _language = i18n::using(language);
            let result = task(service, task_cancel, tx.clone());
            let _ = tx.send(WorkerEvent::Done(result));
        });
        self.mode = Mode::Busy(Pending {
            title: title.into(),
            effect,
            cancel,
            state: TaskState::Running,
            rx,
            handle,
            origin,
            animation: spinner::Spinner::new(std::time::Instant::now()),
        });
        self.message.clear();
    }
    pub(super) fn animate_task(&mut self) -> bool {
        match &mut self.mode {
            Mode::Busy(pending) => pending.animation.tick(std::time::Instant::now()),
            _ => self
                .repository_tasks
                .values_mut()
                .fold(false, |changed, task| {
                    task.animation.tick(std::time::Instant::now()) | changed
                }),
        }
    }
    pub(super) fn poll(&mut self, root: &Cancel) -> bool {
        let mut changed = false;
        let mut result = None;
        if let Mode::Busy(pending) = &mut self.mode {
            match pending.rx.try_recv() {
                Ok(WorkerEvent::Done(value)) => result = Some(value),
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    result = Some(Err(anyhow::anyhow!(i18n::tr!(
                        BackgroundWorkerTerminatedUnexpectedly
                    ))))
                }
            }
        }

        if let Some(result) = result {
            changed = true;
            let Mode::Busy(pending) = self.take_mode() else {
                unreachable!()
            };
            let joined = pending.handle.join();
            let result = if joined.is_err() {
                Err(anyhow::anyhow!(i18n::tr!(BackgroundWorkerPanicked)))
            } else {
                result
            };
            match result {
                Ok(outcome) => self.finish(outcome, *pending.origin, root),
                Err(e) => {
                    let detail =
                        if pending.effect != Effect::ReadOnly && pending.cancel.is_cancelled() {
                            i18n::tf!(OperationInterruptedSomeWritesMayHaveCompleted, e = e)
                        } else {
                            format!("{e:#}")
                        };
                    self.mode = Mode::Error {
                        origin: pending.origin,
                        title: pending.title,
                        detail,
                        scroll: 0,
                    };
                    self.message.clear();
                }
            }
        }
        changed | self.poll_repository_tasks(root)
    }
    pub(super) fn finish(&mut self, outcome: Outcome, origin: Mode, root: &Cancel) {
        self.message.clear();
        self.scroll = 0;
        match outcome {
            Outcome::Checked(report) => {
                self.mode = origin;
                self.message = i18n::tf!(CheckOneComplete, report.repositories[0].path().display());
            }
            Outcome::Branches(view) => {
                self.workspace.save(origin);
                self.mode = Mode::Branches(view);
            }
            Outcome::History(view) => {
                self.workspace.save(origin);
                self.mode = Mode::History(view);
            }
            Outcome::TerminalOpened(path) => {
                self.mode = origin;
                self.message = i18n::tf!(RepositoryTerminalOpened, path.display());
            }
            Outcome::Files(snapshot, area) => {
                let next = match origin.file_session() {
                    Some(current)
                        if current.snapshot.path == snapshot.path && current.area == area =>
                    {
                        current.refreshed(snapshot)
                    }
                    _ => FileSession::new(snapshot, area),
                };
                self.workspace.save(origin);
                self.mode = Mode::Files(next)
            }
            Outcome::Diff(session, content) => {
                self.mode = Mode::Diff {
                    session,
                    content,
                    scroll: 0,
                }
            }
            Outcome::CommitPrepared(plan) => {
                self.workspace.save(origin);
                self.mode = Mode::CommitMessage {
                    plan,
                    editor: Editor::default(),
                    scroll: 0,
                }
            }
            Outcome::PushPrepared(plan) => {
                self.workspace.save(origin);
                self.mode = Mode::PushConfirm {
                    plan,
                    scroll: 0,
                    choice: Default::default(),
                }
            }
            Outcome::Committed(result) => {
                self.mode = Mode::Browsing;
                self.message = i18n::tf!(
                    CommittedRepositoryUpdated,
                    &result.oid[..12],
                    if result.review == engine::operations::commit::Review::Changed {
                        i18n::tr!(ContentsChangedVerifyTheCommit)
                    } else {
                        ""
                    }
                );
            }
            Outcome::Published(result) => match result {
                Publication::Pushed { target } => {
                    self.mode = Mode::Browsing;
                    self.message = i18n::tf!(
                        PushedRepositoryUpdated,
                        target.branch,
                        target.remote,
                        target.remote_ref
                    )
                }
                Publication::Failed { phase, error, .. } => {
                    self.mode = Mode::Error {
                        origin: Box::new(Mode::Browsing),
                        title: i18n::tr!(PushFailed).into(),
                        detail: i18n::tf!(PhasePhaseError, error = error, phase = phase),
                        scroll: 0,
                    }
                }
            },
            Outcome::Text(message) => {
                self.workspace.save(origin);
                self.mode = Mode::Browsing;
                self.message = message;
                if let Err(e) = self.reload(root) {
                    self.message.push_str(&i18n::tf!(RefreshFailedE, e = e))
                }
            }
        }
        if matches!(self.mode, Mode::Browsing) {
            self.refresh_overview();
        }
        self.sync_bar();
    }
    pub(super) fn error(&mut self, title: &str, e: anyhow::Error) {
        let origin = Box::new(self.take_mode());
        self.mode = Mode::Error {
            origin,
            title: title.into(),
            detail: format!("{e:#}"),
            scroll: 0,
        };
    }
    pub(super) fn shutdown(&mut self) {
        for task in self.repository_tasks.values() {
            task.cancel.cancel();
        }
        for (_, task) in std::mem::take(&mut self.repository_tasks) {
            let _ = task.handle.join();
        }
        self.shutdown_check();
        self.overview.shutdown();
        self.workspace.shutdown();
        if let Mode::Busy(pending) = self.take_mode() {
            pending.cancel.cancel();
            let _ = pending.handle.join();
        }
    }
}
