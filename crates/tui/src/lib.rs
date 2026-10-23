//! Terminal adapter over UI-independent services and typed interaction states.
mod bar;
mod branches;
mod changes;
mod checks;
mod clipboard;
mod command;
mod confirmation;
mod diff;
mod document;
mod editor;
mod files;
mod graphics;
mod history;
mod input;
mod layout_picker;
mod mutations;
mod overview;
mod preview;
mod repository_list;
mod repository_tasks;
mod spinner;
mod stashes;
mod task;
mod view;
mod workspace;
use anyhow::{Context, Result};
use command::{Action, Configuration};
use crossterm::{
    event::{self, Event as TerminalEvent, KeyEventKind},
    event::{DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use editor::Editor;
use engine::{
    cancel::Cancel,
    domain::{self, Publication, RepositoryResult},
    local::{
        files::{Area, Change, Files},
        review::CommitPlan,
    },
    operations::{commit::CommitResult, push::PushPlan},
    service::{self, Service, Snapshot},
    state::Config,
};
use files::FileSession;
use ratatui::{Terminal, backend::CrosstermBackend};
use repository_list::{Filter, Order};
use std::{
    collections::BTreeMap,
    io::{self, IsTerminal},
    path::PathBuf,
    sync::{Arc, mpsc},
    thread,
    time::Duration,
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Page {
    Repositories,
    Detail,
    Configuration(Configuration),
    Help,
}
#[derive(Debug)]
enum Mode {
    Browsing,
    Panel(workspace::Section),
    Confirm {
        operation: Confirmation,
        choice: confirmation::Choice,
    },
    Files(FileSession),
    History(Box<history::View>),
    Branches(Box<branches::View>),
    Stashes(Box<stashes::View>),
    Changes(Box<changes::Session>),
    CommitMessage {
        plan: CommitPlan,
        editor: Editor,
        scroll: usize,
    },
    CommitConfirm {
        choice: confirmation::Choice,
        plan: CommitPlan,
        message: String,
        scroll: usize,
    },
    PushConfirm {
        choice: confirmation::Choice,
        plan: PushPlan,
        scroll: usize,
    },
    Busy(Pending),
    Error {
        origin: Box<Mode>,
        title: String,
        detail: String,
        scroll: usize,
    },
}
impl Mode {
    fn file_session(&self) -> Option<&FileSession> {
        match self {
            Self::Files(s) => Some(s),
            Self::Changes(session) => Some(&session.files),
            Self::Error { origin, .. } => origin.file_session(),
            _ => None,
        }
    }
}
#[derive(Debug)]
enum Confirmation {
    Mutation {
        plan: Box<engine::operations::mutation::Plan>,
        origin: Box<Mode>,
        draft: bar::CommandDraft,
    },
    Schedule(engine::scheduler::Control),
    Remove(PathBuf),
    Configure {
        command: command::ConfigArgument,
        value: String,
        origin: Box<Mode>,
        draft: bar::CommandDraft,
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Effect {
    ReadOnly,
    Check,
    IndexWrite,
    RepositoryWrite,
    Commit,
    Push,
    Configuration,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TaskState {
    Running,
    Cancelling,
}
#[derive(Debug)]
struct Pending {
    title: String,
    cancel: Cancel,
    state: TaskState,
    effect: Effect,
    rx: mpsc::Receiver<WorkerEvent>,
    handle: thread::JoinHandle<()>,
    origin: Box<Mode>,
    animation: spinner::Spinner,
}
#[derive(Debug)]
enum WorkerEvent {
    Done(Result<Outcome>),
}
#[derive(Debug)]
enum Outcome {
    Checked(domain::Report),
    Branches(Box<branches::View>),
    Stashes(engine::local::stash::Snapshot),
    MutationPrepared {
        plan: engine::operations::mutation::Plan,
        draft: bar::CommandDraft,
    },
    MutationCompleted(engine::operations::mutation::Change),
    TerminalOpened(PathBuf),
    Files(Files),
    Changes(Files),
    History(Box<history::View>),
    CommitPrepared(CommitPlan),
    PushPrepared(PushPlan),
    Committed(CommitResult),
    Published(Publication),
    Text(String),
}
struct Screen {
    service: Arc<Service>,
    snapshot: Snapshot,
    config: Config,
    page: Page,
    history: Vec<Page>,
    mode: Mode,
    diff_preference: diff::Preference,
    graphics: graphics::Display,
    bar: bar::InputBar,
    clipboard: clipboard::Writer,
    repository_viewport: ratatui::widgets::TableState,
    scroll_input: input::scroll::ScrollInput,
    repository_area: Option<ratatui::layout::Rect>,
    body_area: Option<ratatui::layout::Rect>,
    command_items: Vec<(ratatui::layout::Rect, usize)>,
    filter: Filter,
    query: String,
    order: Order,
    setting_selected: usize,
    scroll: usize,
    message: String,
    check_task: Option<checks::CheckTask>,
    repository_tasks: BTreeMap<PathBuf, repository_tasks::Task>,
    repository_errors: BTreeMap<PathBuf, String>,
    view_revision: u64,
    updates: BTreeMap<PathBuf, checks::Update>,
    overview: overview::Overview,
    workspace: workspace::Workspace,
    workspace_areas: Vec<(ratatui::layout::Rect, workspace::Section)>,
}
#[derive(Debug, Clone)]
struct Row {
    path: PathBuf,
    result: Option<RepositoryResult>,
}
impl Screen {
    fn new(service: Service, cancel: &Cancel) -> Result<Self> {
        let snapshot = service.snapshot(cancel)?;
        let config = service.store.config()?;
        let repository_viewport = ratatui::widgets::TableState::default()
            .with_selected((!config.repositories.is_empty()).then_some(0));
        let mut screen = Self {
            service: Arc::new(service),
            snapshot,
            config,
            page: Page::Repositories,
            history: Vec::new(),
            mode: Mode::Browsing,
            diff_preference: Default::default(),
            graphics: graphics::Display::new(graphics::Support::Unavailable(
                i18n::tr!(GraphicsUnsupported).into(),
            )),
            bar: bar::InputBar::default(),
            clipboard: Default::default(),
            repository_viewport,
            scroll_input: input::scroll::ScrollInput::default(),
            repository_area: None,
            body_area: None,
            command_items: Vec::new(),
            filter: Filter::All,
            query: String::new(),
            order: Order::Priority,
            setting_selected: 0,
            scroll: 0,
            message: String::new(),
            check_task: None,
            repository_tasks: BTreeMap::new(),
            repository_errors: BTreeMap::new(),
            view_revision: 0,
            updates: BTreeMap::new(),
            overview: Default::default(),
            workspace: Default::default(),
            workspace_areas: Vec::new(),
        };
        screen.load_updates()?;
        Ok(screen)
    }
    fn rows(&self) -> Vec<Row> {
        let mut rows = Vec::new();
        for path in &self.config.repositories {
            let result = self.result_for(path);
            if !self.filter.matches(result) {
                continue;
            }
            if !path
                .to_string_lossy()
                .to_lowercase()
                .contains(&self.query.to_lowercase())
            {
                continue;
            }
            rows.push(Row {
                path: path.clone(),
                result: result.cloned(),
            })
        }
        self.sort_rows(&mut rows);
        rows
    }
    fn selected(&self) -> usize {
        self.repository_viewport.selected().unwrap_or(0)
    }
    fn select(&mut self, index: usize) {
        let last = self.rows().len().checked_sub(1);
        self.repository_viewport
            .select(last.map(|last| index.min(last)));
    }
    fn selected_path(&self) -> Option<PathBuf> {
        self.repository_viewport
            .selected()
            .and_then(|index| self.rows().get(index).map(|row| row.path.clone()))
    }
    fn scope_path(&self) -> Option<PathBuf> {
        if let Mode::Stashes(view) = &self.mode {
            return Some(view.session().snapshot.path.clone());
        }
        if let Mode::Branches(view) = &self.mode {
            return Some(view.session().snapshot.path.clone());
        }
        if let Mode::History(view) = &self.mode {
            return Some(view.session().references.path.clone());
        }
        self.mode
            .file_session()
            .map(|s| s.snapshot.path.clone())
            .or_else(|| {
                (self.page == Page::Detail)
                    .then(|| self.workspace.path().map(std::path::Path::to_owned))
                    .flatten()
            })
            .or_else(|| self.selected_path())
    }
    fn focus(&mut self, path: Option<PathBuf>) {
        let rows = self.rows();
        let index = path
            .and_then(|p| rows.iter().position(|r| r.path == p))
            .unwrap_or(0);
        self.select(index);
    }
    fn reload(&mut self, cancel: &Cancel) -> Result<()> {
        let path = self.selected_path();
        self.snapshot = self.service.snapshot(cancel)?;
        self.config = self.service.store.config()?;
        self.load_updates()?;
        self.focus(path);
        Ok(())
    }
    fn take_mode(&mut self) -> Mode {
        std::mem::replace(&mut self.mode, Mode::Browsing)
    }
}

struct TerminalGuard;
impl TerminalGuard {
    fn enter() -> Result<Self> {
        enable_raw_mode()?;
        let guard = Self;
        execute!(
            io::stdout(),
            EnterAlternateScreen,
            EnableBracketedPaste,
            EnableMouseCapture
        )?;
        Ok(guard)
    }
}
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(
            io::stdout(),
            DisableBracketedPaste,
            DisableMouseCapture,
            LeaveAlternateScreen,
            crossterm::cursor::Show
        );
    }
}
pub fn run(service: Service, root: Cancel) -> Result<()> {
    anyhow::ensure!(
        io::stdin().is_terminal() && io::stdout().is_terminal(),
        i18n::tr!(InteractiveTerminalRequired)
    );
    let mut screen = Screen::new(service, &root)?;
    let _guard = TerminalGuard::enter()?;
    screen.graphics = graphics::Display::detect();
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    terminal.clear()?;
    let result = (|| -> Result<()> {
        // 先显示上轮结果，再仅在本次启动时发起一次后台检查。
        terminal.draw(|frame| view::render(frame, &mut screen))?;
        screen.graphics.finish_frame(&mut io::stdout())?;
        screen.check_on_startup();
        let mut dirty = true;
        let mut schedule_second = chrono::Local::now().timestamp();
        loop {
            if root.is_cancelled() {
                if let Mode::Busy(p) = &mut screen.mode {
                    p.cancel.cancel();
                    if p.state == TaskState::Running {
                        p.state = TaskState::Cancelling;
                        dirty = true;
                    }
                } else {
                    break;
                }
            }
            let now = chrono::Local::now().timestamp();
            let schedule_changed = screen.page == Page::Configuration(Configuration::Schedule)
                && now != schedule_second;
            schedule_second = now;
            let changed = schedule_changed
                | screen.poll(&root)
                | screen.poll_check()
                | screen.poll_workspace()
                | screen.poll_changes()
                | screen.poll_clipboard()
                | screen.workspace.animate()
                | screen.animate_check()
                | screen.animate_task()
                | screen.graphics.poll();
            let changed =
                changed | screen.overview.poll(&screen.service) | screen.overview.animate();
            if dirty || changed {
                terminal.draw(|frame| view::render(frame, &mut screen))?;
                screen.graphics.finish_frame(&mut io::stdout())?;
                dirty = false;
            }
            if event::poll(Duration::from_millis(80))? {
                let event = event::read()?;
                match event {
                    TerminalEvent::Key(key) if key.kind != KeyEventKind::Release => {
                        dirty = true;
                        if screen.key(key, &root)? {
                            break;
                        }
                    }
                    TerminalEvent::Mouse(mouse) => {
                        dirty |= screen.mouse(mouse, &root)?;
                    }
                    TerminalEvent::Paste(text) => {
                        screen.paste(&text);
                        dirty = true;
                    }
                    TerminalEvent::Resize(..) => {
                        screen.graphics.refresh_metrics();
                        dirty = true;
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    })();
    screen.shutdown();
    screen.graphics.begin_frame();
    screen.graphics.finish_frame(&mut io::stdout())?;
    terminal
        .show_cursor()
        .context(i18n::tr!(UnableToRestoreTerminalCursor))?;
    result
}

#[cfg(test)]
mod tests;
