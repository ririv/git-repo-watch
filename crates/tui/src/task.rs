//! Background execution, progress, cancellation, and result transitions.
use super::*;
impl Pending {
    pub(super) fn tick(&mut self, now: std::time::Instant) -> bool {
        let changed = self.feedback.tick(now);
        if self.feedback.visible() || self.state == TaskState::Cancelling {
            changed | self.animation.tick(now)
        } else {
            changed
        }
    }
}
impl Screen {
    pub(super) fn start(
        &mut self,
        title: &str,
        effect: Effect,
        task: impl FnOnce(Arc<Service>, Cancel, mpsc::Sender<WorkerEvent>) -> Result<Outcome>
        + Send
        + 'static,
    ) {
        let input = view::input::Snapshot::capture(self);
        let origin = Box::new(self.take_mode());
        let now = std::time::Instant::now();
        let mut feedback = loading::Feedback::default();
        feedback.begin(None, now);
        let cancel = Cancel::default();
        let task_cancel = cancel.clone();
        let service = self.service.clone();
        let (tx, rx) = mpsc::channel();
        let language = i18n::language();
        let spec =
            crate::task_center::spec(title, self.scope_path(), engine::tasks::Retry::Unavailable);
        let handle = thread::spawn(move || {
            let _language = i18n::using(language);
            let result = (|| {
                let run = engine::tasks::Run::begin(&service.store, spec.clone(), &task_cancel)?;
                let result = task(service, task_cancel.clone(), tx.clone());
                run.finish(crate::task_center::outcome(
                    &result,
                    &task_cancel,
                    &spec.title,
                    effect,
                ))?;
                result
            })();
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
            animation: spinner::Spinner::new(now),
            input,
            feedback,
        });
        self.message.clear();
    }
    pub(super) fn animate_task(&mut self) -> bool {
        let waiting = match &mut self.mode {
            Mode::CompositionWaiting(waiting) => match &mut waiting.state {
                composition::waiting::State::Running(animation) => {
                    animation.tick(std::time::Instant::now())
                }
                _ => false,
            },
            _ => false,
        };
        self.commit_generator.tick()
            | waiting
            | match &mut self.mode {
                Mode::Busy(pending) => pending.tick(std::time::Instant::now()),
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
                    let detail = if !matches!(pending.effect, Effect::ReadOnly | Effect::Terminal)
                        && pending.cancel.is_cancelled()
                    {
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
        changed
            | self.poll_repository_tasks(root)
            | self.poll_browser()
            | self.poll_commit_message()
            | self.poll_inspection()
    }
    pub(super) fn finish(&mut self, outcome: Outcome, origin: Mode, root: &Cancel) {
        self.message.clear();
        if !matches!(outcome, Outcome::Opened(_) | Outcome::TextWrapConfigured(_)) {
            self.scroll = 0;
        }
        match outcome {
            Outcome::Scanned(report) => {
                self.enter_management(
                    crate::management::Session::scanned(*report, &self.config.repositories),
                    origin,
                );
            }
            Outcome::RegistrationsAdded(count) => {
                self.mode = origin;
                if let Err(error) = self.reload(root) {
                    self.message = format!("{error:#}");
                } else {
                    self.message = i18n::tf!(AddedRepositories, count);
                }
                if let Mode::Management(session) = &mut self.mode {
                    session.refresh(&self.config.repositories);
                }
            }
            Outcome::Opened(content) => self.enter_view(*content, origin),
            Outcome::BrowserDiff { files } => {
                self.enter_view(navigation::Content::Diff(files), origin);
                if let Mode::Changes(session) = &mut self.mode {
                    session.focus = crate::changes::Focus::Preview;
                }
            }
            Outcome::Stashes(snapshot) => {
                let session = match &origin {
                    Mode::Stashes(view) => view.session().refreshed(snapshot),
                    _ => crate::stashes::Session::new(snapshot),
                };
                self.workspace.save(origin);
                self.mode = Mode::Stashes(Box::new(crate::stashes::View::List(session)));
            }
            Outcome::StashView(view) => {
                self.workspace.save(origin);
                self.mode = Mode::Stashes(view);
            }
            Outcome::ReplayPrepared { preparation, draft } => {
                self.replay_prepared(*preparation, draft, origin)
            }
            Outcome::MutationPrepared { plan, draft } => {
                if let engine::operations::mutation::Plan::Stash(
                    engine::local::stash::Plan::Save(plan),
                ) = *plan
                {
                    self.mode = Mode::StashSave(Box::new(crate::stash_save::Session::new(
                        plan, origin, draft,
                    )));
                } else if let Some(notice) = plan.notice() {
                    self.mode = origin;
                    self.message = notice.into();
                } else {
                    self.mode = Mode::Confirm {
                        operation: Confirmation::Mutation {
                            plan,
                            origin: Box::new(origin),
                            draft,
                        },
                        choice: Default::default(),
                    };
                }
            }
            Outcome::MutationCompleted(change) => {
                self.mode = origin;
                self.refresh_mutated_view();
                if let engine::operations::mutation::Change::FetchSkipped { reason } = change {
                    self.message = reason.label().into();
                } else if let engine::operations::mutation::Change::Synchronized {
                    result: change,
                } = change
                {
                    self.message = change.detail();
                }
            }
            Outcome::Checked(report) => {
                self.mode = origin;
                self.message = i18n::tf!(CheckOneComplete, report.repositories[0].path().display());
            }
            Outcome::Branches(mut view) => {
                if let Mode::Branches(current) = &origin
                    && let crate::branches::View::List(session) = view.as_mut()
                {
                    *session = current.session().refreshed((*session.snapshot).clone());
                }
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
            Outcome::Files(snapshot) => match origin {
                Mode::Changes(mut session) => {
                    session.refreshed(snapshot);
                    self.mode = Mode::Changes(session);
                    self.poll_changes();
                }
                origin => {
                    let next = match origin.file_session() {
                        Some(current) if current.snapshot.path == snapshot.path => {
                            current.refreshed(snapshot)
                        }
                        _ => FileSession::new(snapshot),
                    };
                    self.workspace.save(origin);
                    self.mode = Mode::Files(next);
                }
            },
            Outcome::Composed(prepared) => {
                if self.config.ai.enabled {
                    let origin = match origin {
                        Mode::CompositionWaiting(waiting) => *waiting.origin,
                        origin => origin,
                    };
                    self.workspace.save(origin);
                    self.mode = Mode::Composition(Box::new(composition::Session::new(*prepared)));
                } else {
                    self.mode = Mode::Browsing;
                }
            }
            Outcome::CompositionCommitted(progress) => self.finish_composition(*progress),
            Outcome::CommitPrepared(plan) => {
                self.commit_draft_id += 1;
                self.workspace.save(origin);
                self.mode = Mode::CommitMessage {
                    plan,
                    editor: crate::commit_form::Draft::new(String::new()),
                    scroll: 0,
                }
            }
            Outcome::CommitForGeneration(plan) => {
                self.finish(Outcome::CommitPrepared(plan), origin, root);
                self.generate_commit_message();
            }
            Outcome::PushPrepared(preparation) => {
                self.finish_push_preparation(*preparation, origin)
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
                    self.mode = match origin {
                        Mode::PushConfirm { .. } | Mode::PushTarget(_) => Mode::Browsing,
                        origin => origin,
                    };
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
            Outcome::TextWrapConfigured(enabled) => {
                self.config.text_wrap = enabled;
                self.mode = origin;
                self.message = i18n::tr!(PreferenceSaved).into();
            }
            Outcome::Text(message) => {
                if matches!(origin, Mode::Management(_)) {
                    self.mode = origin;
                } else {
                    self.workspace.save(origin);
                    self.mode = Mode::Browsing;
                }
                self.message = message;
                if let Err(e) = self.reload(root) {
                    self.message.push_str(&i18n::tf!(RefreshFailedE, e = e))
                }
                if let Mode::Management(session) = &mut self.mode {
                    session.refresh(&self.config.repositories);
                }
            }
        }
        if matches!(self.mode, Mode::Browsing) {
            if matches!(self.page, Page::View(_)) {
                self.return_to_view();
            }
            self.refresh_overview();
        }
        self.sync_bar();
    }
    pub(super) fn error(&mut self, title: &str, e: anyhow::Error) {
        if let Mode::Composition(session) = &mut self.mode {
            session.restore();
        }
        if let Mode::CommitMessage { editor, .. } = &mut self.mode
            && editor.generating()
        {
            self.commit_generator.cancel();
            editor.restore();
        }
        if let Mode::PushTarget(session) = &mut self.mode {
            session.restore();
        }
        let origin = Box::new(self.take_mode());
        self.mode = Mode::Error {
            origin,
            title: title.into(),
            detail: format!("{e:#}"),
            scroll: 0,
        };
    }
    pub(super) fn shutdown(&mut self) {
        self.command_reader.clear();
        self.clipboard.shutdown();
        self.link_opener.shutdown();
        self.commit_generator.shutdown();
        self.interaction_save.shutdown();
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
        if let Err(error) = self.view_save.finish() {
            eprintln!("{}: {error:#}", i18n::tr!(ViewPreferenceSaveFailed));
        }
    }
}
