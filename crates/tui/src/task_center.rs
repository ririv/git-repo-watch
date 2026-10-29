//! A task screen and asynchronous journal cache do not take ownership of Git workers.
use super::*;
use engine::tasks::{Record, Retry};
use std::time::{Duration, Instant};
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
enum Request {
    Refresh,
    Cancel(String),
    Resolve(String),
    Stop,
}
struct Worker {
    tx: mpsc::Sender<Request>,
    rx: mpsc::Receiver<Result<Vec<Record>>>,
    handle: thread::JoinHandle<()>,
}
#[derive(Default)]
pub(crate) struct Center {
    pub session: Session,
    pub records: Vec<Record>,
    pub selection: Option<String>,
    pub viewport: ratatui::widgets::ListState,
    pub rows: Vec<(ratatui::layout::Rect, String)>,
    pub footer: ratatui::layout::Rect,
    pub content: ratatui::layout::Rect,
    pub maximum: usize,
    pub error: Option<String>,
    pub animation: crate::spinner::Spinner,
    worker: Option<Worker>,
    refreshed: Option<Instant>,
    observed: Option<(String, Instant)>,
    completion: Option<(String, Instant)>,
    pending_requests: usize,
    clicks: crate::input::clicks::Clicks,
}
impl Center {
    fn observe(&mut self, text: String, now: Instant) {
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
                let result = match request {
                    Request::Stop => break,
                    Request::Cancel(id) => store.cancel_task(&id),
                    Request::Resolve(id) => store.resolve_task(&id),
                    Request::Refresh => Ok(()),
                }
                .and_then(|()| store.tasks(200));
                if updates.send(result).is_err() {
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
    fn move_by(&mut self, delta: isize) {
        let cursor = self
            .selection
            .as_ref()
            .and_then(|id| self.records.iter().position(|r| r.id == *id))
            .unwrap_or(0);
        let cursor = cursor
            .saturating_add_signed(delta)
            .min(self.records.len().saturating_sub(1));
        self.selection = self.records.get(cursor).map(|r| r.id.clone());
    }
    pub fn footer_text(&self) -> String {
        let pending = self.records.iter().filter(|r| r.pending()).count();
        let running = self.records.iter().filter(|r| r.active()).count();
        if let Some(error) = &self.error {
            return format!("{}: {} · Alt+Enter", i18n::tr!(TasksStorageError), error);
        }
        if pending + running > 0 {
            let summary = i18n::tf!(TasksSummary, pending = pending, running = running);
            return if running > 0 {
                format!("{} {summary}", self.animation.symbol())
            } else {
                summary
            };
        }
        self.completion
            .as_ref()
            .filter(|(_, at)| at.elapsed() < Duration::from_secs(5))
            .or(self.observed.as_ref())
            .filter(|(_, at)| at.elapsed() < Duration::from_secs(5))
            .map(|(text, _)| text.lines().next().unwrap_or("").to_owned())
            .unwrap_or_default()
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
        self.tasks.session = Session::List;
        self.tasks.refreshed = None;
        self.drag = None;
        self.clicks.reset();
    }
    pub(crate) fn poll_tasks(&mut self) -> bool {
        let mut changed = false;
        let now = Instant::now();
        if self.tasks.observed.as_ref().map(|(text, _)| text.as_str())
            != Some(self.message.as_str())
        {
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
                match result {
                    Ok(records) => {
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
                                    format!("{} · {}", record.spec.title, i18n::tr!(TasksSuccess)),
                                    now,
                                ));
                            }
                            self.tasks.records = records;
                            changed = true;
                        }
                        changed |= self.tasks.error.take().is_some();
                        if self.tasks.selected().is_none() {
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
                match worker.tx.send(Request::Refresh) {
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
    fn retry_selected_task(&mut self) {
        let Some(record) = self.tasks.selected().cloned() else {
            return;
        };
        let parent = record.pending().then_some(record.id.clone());
        if record.active() {
            return;
        }
        match record.spec.retry {
            Retry::Unavailable => self.message = i18n::tr!(TasksUnavailable).into(),
            Retry::CheckAll => {
                self.start_check_retry(checks::CheckTarget::All, parent);
            }
            Retry::CheckRepository { path } => {
                self.start_check_retry(checks::CheckTarget::Repository(path), parent)
            }
            Retry::Fetch { path } => self.fetch_repository_retry(path, parent),
        }
    }
    pub(crate) fn tasks_key(&mut self, key: crossterm::event::KeyEvent) -> bool {
        use crossterm::event::{KeyCode as K, KeyModifiers as M};
        self.tasks.clicks.reset();
        if key.code == K::Enter && key.modifiers == M::ALT {
            if self.tasks.session.open() {
                self.tasks.session = Session::Closed;
            } else {
                self.open_tasks();
            }
            return true;
        }
        if !self.tasks.session.open() {
            return false;
        }
        if key.code == K::Char('c') && key.modifiers.contains(M::CONTROL) {
            return false;
        }
        match key.code {
            K::Esc | K::Left => {
                self.tasks.session = match self.tasks.session {
                    Session::Detail { .. } => Session::List,
                    _ => Session::Closed,
                }
            }
            K::Char('c') if self.tasks.selected().is_some_and(Record::active) => {
                let id = self.tasks.selection.clone().expect("selected");
                self.task_request(Request::Cancel(id));
            }
            K::Char('d') if self.tasks.selected().is_some_and(Record::pending) => {
                let id = self.tasks.selection.clone().expect("selected");
                self.task_request(Request::Resolve(id));
            }
            K::Char('r') => self.retry_selected_task(),
            K::Enter | K::Right if matches!(self.tasks.session, Session::List) => {
                if let Some(id) = self.tasks.selection.clone() {
                    self.tasks.session = Session::Detail { id, scroll: 0 };
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
        true
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
                        self.tasks.session = Session::Detail { id, scroll: 0 };
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
        Outcome::Published(value) => format!("{value:#?}"),
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
        assert_eq!(center.footer_text(), "done");
        center.observe("done".into(), Instant::now() - Duration::from_secs(5));
        assert_eq!(center.footer_text(), "");
    }
}
