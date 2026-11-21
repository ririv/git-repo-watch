//! A task screen and asynchronous journal cache do not take ownership of Git workers.
pub(crate) mod push_detail;
use super::*;
use engine::tasks::{Journal, Outcome as Completion, Record, Retry, Scope, State};
use std::{
    collections::BTreeSet,
    time::{Duration, Instant},
};
#[derive(Debug, Default)]
pub(crate) enum Session {
    #[default]
    Closed,
    List,
    Detail {
        id: String,
        scroll: usize,
    },
}
impl Session {
    pub fn open(&self) -> bool {
        !matches!(self, Self::Closed)
    }
}
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Filter {
    #[default]
    All,
    Unread,
    Failed,
    Active,
    Succeeded,
    Cancelled,
}
impl Filter {
    pub fn label(self) -> &'static str {
        match self {
            Self::All => i18n::tr!(TasksFilterAll),
            Self::Unread => i18n::tr!(TasksFilterUnread),
            Self::Failed => i18n::tr!(TasksFailed),
            Self::Active => i18n::tr!(TasksRunning),
            Self::Succeeded => i18n::tr!(TasksSuccess),
            Self::Cancelled => i18n::tr!(TasksCancelled),
        }
    }

    fn includes(self, record: &Record) -> bool {
        match self {
            Self::All => true,
            Self::Unread => record.unread_failure(),
            Self::Active => record.active(),
            Self::Failed => matches!(
                record.state,
                State::Finished {
                    outcome: Completion::Failed { .. },
                    ..
                }
            ),
            Self::Succeeded => matches!(
                record.state,
                State::Finished {
                    outcome: Completion::Succeeded { .. },
                    ..
                }
            ),
            Self::Cancelled => matches!(
                record.state,
                State::Finished {
                    outcome: Completion::Cancelled { .. },
                    ..
                }
            ),
        }
    }
}

/// A task command owns the validated operation and its original task identity.
pub(crate) enum Operation {
    FilterMenu,
    Filter(Filter),
    Cancel(String),
    FetchAll {
        parent: Option<String>,
    },
    Fetch {
        path: PathBuf,
        parent: Option<String>,
    },
}
impl Operation {
    pub fn prepare(action: Action, record: &Record) -> std::result::Result<Self, &'static str> {
        match action {
            Action::TaskCancel if record.active() => Ok(Self::Cancel(record.id.clone())),
            Action::TaskCancel => Err(i18n::tr!(TaskCancelUnavailable)),
            Action::TaskRetry if record.active() => Err(i18n::tr!(TasksUnavailable)),
            Action::TaskRetry => {
                let parent = record.pending().then(|| record.id.clone());
                match &record.spec.retry {
                    Retry::Unavailable => Err(i18n::tr!(TasksUnavailable)),
                    Retry::FetchAll => Ok(Self::FetchAll { parent }),
                    Retry::Fetch { path } => Ok(Self::Fetch {
                        path: path.clone(),
                        parent,
                    }),
                }
            }
            _ => unreachable!("task operation requires a task command"),
        }
    }
}
enum Request {
    Refresh(Option<String>),
    Cancel(String),
    ReadFailure(String),
    Stop,
}
struct Update {
    read_failure: Option<String>,
    journal: Result<Journal>,
}
struct Worker {
    tx: mpsc::Sender<Request>,
    rx: mpsc::Receiver<Update>,
    handle: thread::JoinHandle<()>,
}
#[derive(Default)]
pub(crate) struct Center {
    pub session: Session,
    pub records: Vec<Record>,
    pub unread_failures: usize,
    pub visible_unread_failure: Option<String>,
    pub selection: Option<String>,
    pub search: Editor,
    pub filter: Filter,
    pub bar: crate::bar::InputBar,
    pub viewport: ratatui::widgets::ListState,
    pub rows: Vec<(ratatui::layout::Rect, String)>,
    pub footer: ratatui::layout::Rect,
    pub content: ratatui::layout::Rect,
    pub maximum: usize,
    pub error: Option<String>,
    pub animation: crate::spinner::Spinner,
    worker: Option<Worker>,
    refreshed: Option<Instant>,
    observed: Option<(crate::message::Message, Instant)>,
    completion: Option<(crate::message::Message, Instant)>,
    pending_requests: usize,
    pending_reads: BTreeSet<String>,
    clicks: crate::input::clicks::Clicks,
}
impl Center {
    pub(crate) fn input_bar_mut<'a>(
        &'a mut self,
        underlying: &'a mut crate::bar::InputBar,
    ) -> &'a mut crate::bar::InputBar {
        if self.session.open() {
            &mut self.bar
        } else {
            underlying
        }
    }

    fn observe(&mut self, text: crate::message::Message, now: Instant) {
        self.observed = Some((text, now));
    }
    fn connect(&mut self, store: engine::state::Store) {
        if self.worker.is_some() {
            return;
        }
        let (tx, requests) = mpsc::channel();
        let (updates, rx) = mpsc::channel();
        let language = i18n::language();
        let handle = thread::spawn(move || {
            let _language = i18n::using(language);
            while let Ok(request) = requests.recv() {
                let read_failure = match &request {
                    Request::ReadFailure(id) => Some(id.clone()),
                    _ => None,
                };
                let (result, selected) = match request {
                    Request::Stop => break,
                    Request::Cancel(id) => (store.cancel_task(&id), Some(id)),
                    Request::ReadFailure(id) => (store.read_task_failure(&id), Some(id)),
                    Request::Refresh(selected) => (Ok(()), selected),
                };
                let journal = result.and_then(|()| store.task_journal(200, selected.as_deref()));
                if updates
                    .send(Update {
                        read_failure,
                        journal,
                    })
                    .is_err()
                {
                    break;
                }
            }
        });
        self.worker = Some(Worker { tx, rx, handle });
    }
    pub fn selected(&self) -> Option<&Record> {
        self.selection
            .as_ref()
            .and_then(|id| self.records.iter().find(|r| r.id == *id))
    }
    pub fn visible_indices(&self) -> Vec<usize> {
        self.matching_indices(self.selection.as_deref())
    }
    fn matching_indices(&self, retained: Option<&str>) -> Vec<usize> {
        let query = self.search.text.trim().to_lowercase();
        self.records
            .iter()
            .enumerate()
            .filter(|(_, record)| {
                // 详情会把当前失败标为已读。保留当前项直到用户移动选中项，
                // 避免自动跳到下一条并连续标记已读。
                self.filter.includes(record)
                    || (self.filter == Filter::Unread
                        && retained == Some(record.id.as_str())
                        && Filter::Failed.includes(record))
            })
            .filter(|(_, record)| {
                query.is_empty()
                    || record.spec.title.to_lowercase().contains(&query)
                    || status_label(record).to_lowercase().contains(&query)
                    || match &record.spec.scope {
                        Scope::Repository { path } => {
                            path.to_string_lossy().to_lowercase().contains(&query)
                        }
                        Scope::All => i18n::tr!(TasksAll).to_lowercase().contains(&query),
                        Scope::Application => {
                            i18n::tr!(TasksApplication).to_lowercase().contains(&query)
                        }
                    }
            })
            .map(|(index, _)| index)
            .collect()
    }
    fn visible_selection(&self, visible: &[usize]) -> Option<usize> {
        self.selection.as_ref().and_then(|id| {
            visible
                .iter()
                .position(|&index| self.records[index].id == *id)
        })
    }
    fn reconcile_selection(&mut self) {
        if matches!(self.session, Session::List) {
            self.select_matching(&self.visible_indices());
        }
    }
    fn select_matching(&mut self, visible: &[usize]) {
        if self.visible_selection(visible).is_none() {
            self.selection = visible.first().map(|&index| self.records[index].id.clone());
        }
    }
    fn move_by(&mut self, delta: isize) {
        let visible = self.visible_indices();
        let cursor = self
            .visible_selection(&visible)
            .unwrap_or(0)
            .saturating_add_signed(delta)
            .min(visible.len().saturating_sub(1));
        self.selection = visible
            .get(cursor)
            .map(|&index| self.records[index].id.clone());
    }
    fn search_changed(&mut self) {
        self.viewport = Default::default();
        self.rows.clear();
        if matches!(self.session, Session::List) {
            self.select_matching(&self.matching_indices(None));
        }
    }
    pub fn update_search(&mut self, editor: Editor) {
        let changed = self.search.text != editor.text;
        self.search = editor;
        if changed {
            self.search_changed();
        }
    }

    pub(crate) fn recent_notice(&self) -> Option<&crate::message::Message> {
        self.completion
            .as_ref()
            .filter(|(_, at)| at.elapsed() < Duration::from_secs(5))
            .or_else(|| {
                self.observed
                    .as_ref()
                    .filter(|(_, at)| at.elapsed() < Duration::from_secs(5))
            })
            .map(|(text, _)| text)
            .filter(|text| !text.is_empty())
    }
}
impl Drop for Center {
    fn drop(&mut self) {
        if let Some(worker) = self.worker.take() {
            let _ = worker.tx.send(Request::Stop);
            let _ = worker.handle.join();
        }
    }
}
impl Screen {
    pub(crate) fn open_tasks(&mut self) {
        match self.tasks.session {
            Session::List => return,
            Session::Detail { .. } => self.tasks.session = Session::List,
            Session::Closed => {
                self.tasks.session = Session::List;
                self.tasks.search = Editor::default();
                self.command_reader.clear();
            }
        }
        self.sync_task_bar();
        self.tasks.search_changed();
        self.tasks.refreshed = None;
        self.command_items.clear();
        self.drag = None;
        self.clicks.reset();
    }
    pub(crate) fn close_tasks(&mut self) {
        if self.tasks.session.open() {
            self.tasks.session = Session::Closed;
            self.sync_task_bar();
            self.command_items.clear();
            self.command_reader.clear();
            if let crate::bar::InputBar::Command {
                menu: command::completion::Menu::Loading(request),
                ..
            } = &self.bar
            {
                // 恢复打开任务页之前的补全请求，保持原草稿可继续编辑。
                self.command_reader.request(request.clone(), &self.service);
            }
        }
    }

    pub(crate) fn refresh_tasks(&mut self) {
        self.tasks.refreshed = None;
    }

    pub(crate) fn poll_tasks(&mut self) -> bool {
        let mut changed = false;
        let mut records_changed = false;
        let now = Instant::now();
        if self.tasks.observed.as_ref().map(|(text, _)| text) != Some(&self.message) {
            self.tasks.observe(self.message.clone(), now);
            changed = true;
        } else if let Some((text, at)) = &mut self.tasks.observed
            && !text.is_empty()
            && now.duration_since(*at) >= Duration::from_secs(5)
        {
            self.message.clear();
            text.clear();
            changed = true;
        }
        if self
            .tasks
            .completion
            .as_ref()
            .is_some_and(|(_, at)| now.duration_since(*at) >= Duration::from_secs(5))
        {
            self.tasks.completion = None;
            changed = true;
        }
        if self.tasks.records.iter().any(Record::active) {
            changed |= self.tasks.animation.tick(now);
        }
        self.tasks.connect(self.service.store.clone());
        if let Some(worker) = &self.tasks.worker {
            loop {
                let result = match worker.rx.try_recv() {
                    Ok(result) => result,
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        let error = i18n::tr!(BackgroundWorkerTerminatedUnexpectedly).to_owned();
                        changed |= self.tasks.error.as_ref() != Some(&error);
                        self.tasks.error = Some(error);
                        self.tasks.pending_requests = 0;
                        break;
                    }
                };
                self.tasks.pending_requests = self.tasks.pending_requests.saturating_sub(1);
                if let Some(id) = result.read_failure {
                    self.tasks.pending_reads.remove(&id);
                }
                match result.journal {
                    Ok(Journal {
                        records,
                        unread_failures,
                    }) => {
                        if unread_failures != self.tasks.unread_failures {
                            self.tasks.unread_failures = unread_failures;
                            changed = true;
                        }
                        if records != self.tasks.records {
                            if let Some(record) = records.iter().find(|record| {
                                matches!(
                                    record.state,
                                    engine::tasks::State::Finished {
                                        outcome: engine::tasks::Outcome::Succeeded { .. },
                                        ..
                                    }
                                ) && self
                                    .tasks
                                    .records
                                    .iter()
                                    .any(|old| old.id == record.id && old.active())
                            }) {
                                self.tasks.completion = Some((
                                    crate::message::Message::information(format!(
                                        "{} · {}",
                                        record.spec.title,
                                        i18n::tr!(TasksSuccess)
                                    )),
                                    now,
                                ));
                            }
                            self.tasks.records = records;
                            changed = true;
                            records_changed = true;
                        }
                        changed |= self.tasks.error.take().is_some();
                        if !matches!(self.tasks.session, Session::List)
                            && self.tasks.selected().is_none()
                        {
                            self.tasks.selection = self.tasks.records.first().map(|r| r.id.clone());
                        }
                    }
                    Err(error) => {
                        let detail = format!("{error:#}");
                        if self.tasks.error.as_ref() != Some(&detail) {
                            changed = true;
                            self.tasks.error = Some(detail);
                        }
                    }
                }
            }
            if self.tasks.pending_requests == 0
                && self
                    .tasks
                    .refreshed
                    .is_none_or(|at| now.duration_since(at) >= Duration::from_millis(500))
            {
                match worker
                    .tx
                    .send(Request::Refresh(self.tasks.selection.clone()))
                {
                    Ok(()) => self.tasks.pending_requests += 1,
                    Err(_) => {
                        self.tasks.error =
                            Some(i18n::tr!(BackgroundWorkerTerminatedUnexpectedly).into());
                        changed = true;
                    }
                }
                self.tasks.refreshed = Some(now);
                changed |=
                    self.tasks.session.open() && self.tasks.records.iter().any(Record::active);
            }
        }
        if records_changed {
            self.tasks.reconcile_selection();
        }
        changed
    }
    fn task_request(&mut self, request: Request) {
        if let Some(worker) = &self.tasks.worker {
            match worker.tx.send(request) {
                Ok(()) => self.tasks.pending_requests += 1,
                Err(_) => {
                    self.tasks.error =
                        Some(i18n::tr!(BackgroundWorkerTerminatedUnexpectedly).into())
                }
            }
        }
    }
    pub(crate) fn acknowledge_task_frame(&mut self) {
        if let Some(id) = self.tasks.visible_unread_failure.take()
            && self.tasks.pending_reads.insert(id.clone())
        {
            self.tasks.connect(self.service.store.clone());
            self.task_request(Request::ReadFailure(id));
        }
    }

    fn open_task_detail(&mut self, id: String) {
        self.tasks.session = Session::Detail { id, scroll: 0 };
        self.sync_task_bar();
    }

    pub(crate) fn execute_task(
        &mut self,
        operation: Operation,
        _permit: &crate::command::target::ExecutionPermit,
    ) {
        match operation {
            Operation::FilterMenu => self.open_option_menu(
                command::options::Kind::TaskFilter,
                None,
                crate::clipboard::Selection::None,
            ),
            Operation::Filter(filter) => {
                self.tasks.filter = filter;
                self.tasks.session = Session::List;
                self.tasks.search_changed();
                self.sync_task_bar();
            }
            Operation::Cancel(id) => self.task_request(Request::Cancel(id)),
            Operation::FetchAll { parent } => {
                self.start_check_retry(checks::CheckTarget::All, parent)
            }
            Operation::Fetch { path, parent } => self.fetch_repository_retry(path, parent),
        }
    }

    pub(crate) fn tasks_key(
        &mut self,
        key: crossterm::event::KeyEvent,
        root: &Cancel,
    ) -> Result<bool> {
        use crossterm::event::{KeyCode as K, KeyModifiers as M};
        self.tasks.clicks.reset();
        let action = match (key.code, key.modifiers) {
            (K::Char('x'), M::CONTROL) => Some(Action::TaskCancel),
            (K::Char('r'), M::CONTROL) => Some(Action::TaskRetry),
            _ => None,
        };
        if let Some(action) = action {
            return self.activate(action, None, self.copy_selection(), root);
        }
        match self.bar_key(key, root)? {
            crate::bar::BarEvent::Handled => return Ok(false),
            crate::bar::BarEvent::Quit => return Ok(true),
            crate::bar::BarEvent::Pass => {}
        }
        match key.code {
            K::Esc | K::Left => match self.tasks.session {
                Session::Detail { .. } => {
                    self.tasks.session = Session::List;
                    self.tasks.reconcile_selection();
                    self.sync_task_bar();
                }
                Session::List => self.close_tasks(),
                Session::Closed => unreachable!("task key requires the task view"),
            },
            K::Enter | K::Right if matches!(self.tasks.session, Session::List) => {
                if let Some(id) = self.tasks.selection.clone() {
                    self.open_task_detail(id);
                }
            }
            K::Up | K::Down | K::PageUp | K::PageDown | K::Home | K::End => {
                let delta = match key.code {
                    K::Up => -1,
                    K::Down => 1,
                    K::PageUp => -10,
                    K::PageDown => 10,
                    K::Home => isize::MIN,
                    _ => isize::MAX,
                };
                match &mut self.tasks.session {
                    Session::List => self.tasks.move_by(delta),
                    Session::Detail { scroll, .. } => {
                        *scroll = scroll.saturating_add_signed(delta).min(self.tasks.maximum)
                    }
                    Session::Closed => {}
                }
            }
            _ => {}
        }
        Ok(false)
    }
    pub(crate) fn tasks_mouse(&mut self, event: crossterm::event::MouseEvent) -> bool {
        use crossterm::event::{MouseButton, MouseEventKind as K};
        use ratatui::layout::Position;
        let point = Position::new(event.column, event.row);
        if !self.tasks.session.open() {
            if self.config.mouse.precise_click
                && event.kind == K::Down(MouseButton::Left)
                && self.tasks.footer.contains(point)
            {
                self.open_tasks();
                return true;
            }
            return false;
        }
        if !self.active_bar().is_plain() {
            return true;
        }
        match event.kind {
            K::Down(MouseButton::Left) if self.config.mouse.precise_click => {
                if let Some((_, id)) = self
                    .tasks
                    .rows
                    .iter()
                    .find(|(area, _)| area.contains(point))
                {
                    let id = id.clone();
                    self.tasks.selection = Some(id.clone());
                    if self.tasks.clicks.press(
                        crate::input::clicks::Identity::Task(id.clone()),
                        point,
                        Instant::now(),
                    ) {
                        self.open_task_detail(id);
                    }
                }
            }
            K::ScrollUp | K::ScrollDown
                if self.config.mouse.scroll && self.tasks.content.contains(point) =>
            {
                self.tasks.clicks.reset();
                let delta = if event.kind == K::ScrollUp { -3 } else { 3 };
                match &mut self.tasks.session {
                    Session::List => self.tasks.move_by(delta),
                    Session::Detail { scroll, .. } => {
                        *scroll = scroll.saturating_add_signed(delta).min(self.tasks.maximum)
                    }
                    Session::Closed => {}
                }
            }
            _ => {}
        }
        true
    }
}
pub(crate) fn status_label(record: &Record) -> &'static str {
    match &record.state {
        State::Running { .. } => i18n::tr!(TasksRunning),
        State::Cancelling { .. } => i18n::tr!(TasksCancelling),
        State::Finished { outcome, .. } => match outcome {
            Completion::Succeeded { .. } => i18n::tr!(TasksSuccess),
            Completion::Cancelled { .. } => i18n::tr!(TasksCancelled),
            Completion::Failed { .. } => i18n::tr!(TasksFailed),
        },
    }
}

pub(crate) fn spec(title: &str, path: Option<PathBuf>, retry: Retry) -> engine::tasks::Spec {
    engine::tasks::Spec {
        title: title.into(),
        scope: path.map_or(engine::tasks::Scope::Application, |path| {
            engine::tasks::Scope::Repository { path }
        }),
        source: engine::tasks::Source::Interactive,
        retry,
        retry_of: None,
        operation: None,
    }
}
pub(crate) fn outcome(
    result: &Result<Outcome>,
    cancel: &Cancel,
    title: &str,
    effect: Effect,
) -> engine::tasks::Outcome {
    let outcome = engine::tasks::Outcome::capture(result, cancel, |value| match value {
        Outcome::Text(text) => text.clone(),
        Outcome::Scanned(report) => {
            format!("{} · {}", i18n::tr!(ScanResults), report.repositories.len())
        }
        Outcome::RegistrationsAdded(count) => i18n::tf!(AddedRepositories, count),
        Outcome::Committed(value) => format!("{value:#?}"),
        Outcome::Composed(prepared) => format!(
            "{}: {}\n{}: {}",
            i18n::tr!(ComposePending),
            prepared.plan.commits.len(),
            i18n::tr!(ComposeUnits),
            prepared.snapshot.units.len()
        ),
        Outcome::CompositionCommitted(progress) => match progress.as_ref() {
            engine::composition::Progress::More { result, .. } => format!("{result:#?}"),
            engine::composition::Progress::Complete { completed } => {
                format!("{:#?}", completed.last())
            }
        },
        Outcome::Published(Publication::Pushed { target }) => push_detail::from_target(target),
        Outcome::PushPrepared(preparation) => crate::push::preparation_notice(preparation).into(),
        Outcome::MutationCompleted(engine::operations::mutation::Change::FetchSkipped {
            reason,
        }) => reason.label().into(),
        Outcome::MutationPrepared { plan, .. }
            if matches!(plan.as_ref(), engine::operations::mutation::Plan::Sync(_)) =>
        {
            plan.notice()
                .map(str::to_owned)
                .unwrap_or_else(|| crate::mutations::review(plan))
        }
        Outcome::MutationCompleted(engine::operations::mutation::Change::Synchronized {
            result: change,
        }) => change.detail(),
        Outcome::MutationCompleted(engine::operations::mutation::Change::Replayed {
            kind,
            commit,
        }) => format!("{}\n{}: {}", kind.label(), i18n::tr!(ReplaySource), commit),
        Outcome::MutationCompleted(engine::operations::mutation::Change::ReplayedSequence {
            commits,
        }) => format!(
            "{}\n{}\n{}",
            i18n::tr!(ReplayBatchTitle),
            i18n::tr!(ReplayOrder),
            commits.join("\n")
        ),
        Outcome::MutationCompleted(engine::operations::mutation::Change::ReplayControlled {
            kind,
            control,
        }) => format!("{} · {}", kind.label(), control.label()),
        Outcome::MutationCompleted(value) => format!("{value:#?}"),
        _ => title.to_owned(),
    });
    match outcome {
        engine::tasks::Outcome::Cancelled { detail }
            if matches!(
                effect,
                Effect::IndexWrite | Effect::RepositoryWrite | Effect::Commit | Effect::Push
            ) =>
        {
            engine::tasks::Outcome::Cancelled {
                detail: i18n::tf!(OperationInterruptedSomeWritesMayHaveCompleted, e = detail),
            }
        }
        outcome => outcome,
    }
}
#[cfg(test)]
impl Screen {
    pub(crate) fn tasks_refresh_for_test(&mut self) {
        self.tasks.refreshed = None;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn success_notice_expires_after_five_seconds() {
        let mut center = Center::default();
        center.observe("done".into(), Instant::now() - Duration::from_secs(4));
        assert_eq!(
            center.recent_notice().map(|message| message.as_str()),
            Some("done")
        );
        center.observe("done".into(), Instant::now() - Duration::from_secs(5));
        assert!(center.recent_notice().is_none());
    }
}
