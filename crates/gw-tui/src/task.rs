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
        let handle = thread::spawn(move || {
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
        });
        self.message.clear();
    }
    pub(super) fn poll(&mut self, root: &Cancel) -> bool {
        let mut changed = false;
        let mut result = None;
        if let Mode::Busy(pending) = &mut self.mode {
            loop {
                match pending.rx.try_recv() {
                    Ok(WorkerEvent::Progress(event)) => {
                        changed = true;
                        self.message = match event {
                            service::Event::Checking(path) => {
                                format!("正在检查：{}", path.display())
                            }
                            service::Event::Checked(result) => {
                                format!("{}：{}", result.label(), result.path().display())
                            }
                            service::Event::NotificationProblem(notice) => {
                                format!("系统通知发送失败：{notice:?}")
                            }
                        }
                    }
                    Ok(WorkerEvent::Done(value)) => {
                        result = Some(value);
                        break;
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        result = Some(Err(anyhow::anyhow!("后台工作线程异常退出")));
                        break;
                    }
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
                Err(anyhow::anyhow!("后台工作线程发生异常"))
            } else {
                result
            };
            match result {
                Ok(outcome) => self.finish(outcome, root),
                Err(e) => {
                    let detail =
                        if pending.effect != Effect::ReadOnly && pending.cancel.is_cancelled() {
                            format!("操作被中断，可能已完成部分写入，请刷新并核对实际状态。\n{e:#}")
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
    }
    pub(super) fn finish(&mut self, outcome: Outcome, root: &Cancel) {
        self.message.clear();
        self.scroll = 0;
        match outcome {
            Outcome::Files(snapshot, area) => {
                self.mode = Mode::Files(FileSession {
                    snapshot,
                    area,
                    cursor: 0,
                    selected: BTreeSet::new(),
                    query: String::new(),
                })
            }
            Outcome::Diff(session, content) => {
                self.mode = Mode::Diff {
                    session,
                    content,
                    scroll: 0,
                }
            }
            Outcome::CommitPrepared(plan) => {
                self.mode = Mode::CommitMessage {
                    plan,
                    editor: Editor::default(),
                    scroll: 0,
                }
            }
            Outcome::PushPrepared(plan) => self.mode = Mode::PushConfirm { plan, scroll: 0 },
            Outcome::Committed(result) => {
                self.mode = Mode::Browsing;
                self.message = format!(
                    "已提交 {}{}；列表仍为上次检查结果",
                    &result.oid[..12],
                    if result.review == gw_core::operations::commit::Review::Changed {
                        "（实际内容发生变化，请核对提交）"
                    } else {
                        ""
                    }
                );
            }
            Outcome::Published(result) => match result {
                Publication::Pushed { target } => {
                    self.mode = Mode::Browsing;
                    self.message = format!(
                        "已推送 {} → {}/{}；列表仍为上次检查结果",
                        target.branch, target.remote, target.remote_ref
                    )
                }
                Publication::Failed { phase, error, .. } => {
                    self.mode = Mode::Error {
                        origin: Box::new(Mode::Browsing),
                        title: "推送失败".into(),
                        detail: format!("{phase:?} 阶段：{error}"),
                        scroll: 0,
                    }
                }
            },
            Outcome::Text(message) => {
                self.mode = Mode::Browsing;
                self.message = message;
                if let Err(e) = self.reload(root) {
                    self.message.push_str(&format!("；刷新失败：{e:#}"))
                }
            }
            Outcome::Checked(report) => {
                self.mode = Mode::Browsing;
                let stats = domain::Stats::of(&report);
                self.message = format!(
                    "检查完成：{} 正常 · {} 需关注 · {} 失败",
                    stats.clean, stats.attention, stats.failed
                );
                if let Err(e) = self.reload(root) {
                    self.message.push_str(&format!("；刷新失败：{e:#}"))
                }
            }
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
        if let Mode::Busy(pending) = self.take_mode() {
            pending.cancel.cancel();
            let _ = pending.handle.join();
        }
    }
}
