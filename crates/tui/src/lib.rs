//! Terminal adapter over UI-independent services and typed interaction states.
mod bar;
mod checks;
mod command;
mod editor;
mod input;
mod task;
mod view;
use anyhow::{Context, Result};
use command::Action;
use crossterm::{
    event::{self, Event as TerminalEvent, KeyEventKind},
    event::{DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use editor::Editor;
use engine::{
    cancel::Cancel,
    domain::{self, Filter, Publication, RepositoryResult},
    local::{
        files::{Area, Change, Files},
        review::CommitPlan,
    },
    operations::{commit::CommitResult, push::PushPlan},
    service::{self, Service, Snapshot},
    state::Config,
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{
    collections::{BTreeMap, BTreeSet},
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
    Settings,
    Help,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Order {
    Priority,
    Path,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileArea {
    Worktree,
    Index,
}
#[derive(Debug, Clone)]
struct FileSession {
    snapshot: Files,
    area: FileArea,
    cursor: usize,
    selected: BTreeSet<usize>,
    query: String,
}
impl FileSession {
    fn rows(&self) -> Vec<&Change> {
        self.snapshot
            .changes
            .iter()
            .filter(|c| match self.area {
                FileArea::Index => c.area == Area::Staged,
                FileArea::Worktree => c.area != Area::Staged,
            })
            .filter(|c| {
                c.location
                    .name()
                    .to_string_lossy()
                    .to_lowercase()
                    .contains(&self.query.to_lowercase())
            })
            .collect()
    }
    fn change_index(&self, change: &Change) -> usize {
        self.snapshot
            .changes
            .iter()
            .position(|c| c == change)
            .expect("visible change belongs to snapshot")
    }
    fn selection(&self) -> Vec<Change> {
        let rows = self.rows();
        if self.selected.is_empty() {
            rows.get(self.cursor)
                .map(|c| vec![(*c).clone()])
                .unwrap_or_default()
        } else {
            self.snapshot
                .changes
                .iter()
                .enumerate()
                .filter(|(i, _)| self.selected.contains(i))
                .map(|(_, c)| c.clone())
                .collect()
        }
    }
}
#[derive(Debug)]
enum Mode {
    Browsing,
    Input {
        action: Action,
        editor: Editor,
    },
    Confirm(Confirmation),
    Files(FileSession),
    Diff {
        session: FileSession,
        content: String,
        scroll: usize,
    },
    CommitMessage {
        plan: CommitPlan,
        editor: Editor,
        scroll: usize,
    },
    CommitConfirm {
        plan: CommitPlan,
        message: String,
        scroll: usize,
    },
    PushConfirm {
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
            Self::Files(s) | Self::Diff { session: s, .. } => Some(s),
            Self::Error { origin, .. } => origin.file_session(),
            _ => None,
        }
    }
}
#[derive(Debug)]
enum Confirmation {
    Apply,
    Remove(PathBuf),
    Configure { action: Action, value: String },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Effect {
    ReadOnly,
    IndexWrite,
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
}
#[derive(Debug)]
enum WorkerEvent {
    Done(Result<Outcome>),
}
#[derive(Debug)]
enum Outcome {
    TerminalOpened(PathBuf),
    Files(Files, FileArea),
    Diff(FileSession, String),
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
    bar: bar::InputBar,
    repository_viewport: ratatui::widgets::TableState,
    scroll_input: input::scroll::ScrollInput,
    repository_area: Option<ratatui::layout::Rect>,
    body_area: Option<ratatui::layout::Rect>,
    filter: Filter,
    query: String,
    order: Order,
    setting_selected: usize,
    scroll: usize,
    message: String,
    check_task: Option<checks::CheckTask>,
    updates: BTreeMap<PathBuf, checks::Update>,
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
            bar: bar::InputBar::default(),
            repository_viewport,
            scroll_input: input::scroll::ScrollInput::default(),
            repository_area: None,
            body_area: None,
            filter: Filter::All,
            query: String::new(),
            order: Order::Priority,
            setting_selected: 0,
            scroll: 0,
            message: String::new(),
            check_task: None,
            updates: BTreeMap::new(),
        };
        screen.load_updates()?;
        Ok(screen)
    }
    fn rows(&self) -> Vec<Row> {
        let mut rows = Vec::new();
        for path in &self.config.repositories {
            let result = self.result_for(path);
            if self.filter != Filter::All && !result.is_some_and(|r| self.filter.matches(r)) {
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
        rows.sort_by(|a, b| {
            let priority = |r: &Row| r.result.as_ref().map(|r| r.priority()).unwrap_or(2);
            if self.order == Order::Priority {
                priority(a).cmp(&priority(b)).then(a.path.cmp(&b.path))
            } else {
                a.path.cmp(&b.path)
            }
        });
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
        self.mode
            .file_session()
            .map(|s| s.snapshot.path.clone())
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
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    terminal.clear()?;
    let result = (|| -> Result<()> {
        let mut dirty = true;
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
            let changed = screen.poll(&root) | screen.poll_check() | screen.animate_check();
            if dirty || changed {
                terminal.draw(|frame| view::render(frame, &mut screen))?;
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
                    TerminalEvent::Resize(..) => dirty = true,
                    _ => {}
                }
            }
        }
        Ok(())
    })();
    screen.shutdown();
    terminal
        .show_cursor()
        .context(i18n::tr!(UnableToRestoreTerminalCursor))?;
    result
}

#[cfg(test)]
mod tests;
