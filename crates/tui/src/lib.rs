//! Terminal adapter over UI-independent services and typed interaction states.
mod startup;
pub use startup::Start;
mod bar;
mod branches;
mod browser;
mod changes;
mod checks;
mod clipboard;
mod command;
mod command_trigger;
mod commit_form;
mod commit_message;
mod composition;
mod confirmation;
mod diff;
mod document;
mod editor;
mod files;
mod flow;
mod focus;
mod graphics;
mod history;
mod horizontal;
mod input;
mod interaction;
mod layout_picker;
mod links;
mod loading;
mod markdown;
mod multiline_editor;
mod mutations;
mod navigation;
mod overview;
mod pointer;
mod preview;
mod prompt_editor;
mod push;
mod replay;
mod repository_list;
mod repository_tasks;
mod scrollbar;
mod search;
mod settings;
mod spinner;
mod stashes;
mod task;
mod task_center;
mod view;
mod workflow;
mod workspace;
use anyhow::{Context, Result};
use command::{Action, Configuration};
use crossterm::{
    event::{self, Event as TerminalEvent, KeyEventKind},
    event::{DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste},
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
    View(navigation::View),
    Repositories,
    Detail,
    Configuration(Configuration),
    Help,
}
#[derive(Debug)]
enum Mode {
    Workflow(Box<workflow::Session>),
    PromptEditor(Box<prompt_editor::Session>),
    Composition(Box<composition::Session>),
    CompositionWaiting(Box<composition::waiting::Session>),
    Browsing,
    Browser(Box<browser::Session>),
    /// Focused pane without an interactive content session (overview or pending data).
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
        editor: commit_form::Draft,
        scroll: usize,
    },
    PushConfirm {
        choice: confirmation::Choice,
        plan: PushPlan,
        scroll: usize,
    },
    PushTarget(Box<push::Session>),
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
    ReplayParents {
        candidate: Box<engine::operations::replay::Candidate>,
        selected: usize,
        origin: Box<Mode>,
        draft: bar::CommandDraft,
    },
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
    Terminal,
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
    input: view::input::Snapshot,
    feedback: loading::Feedback<()>,
}
#[derive(Debug)]
enum WorkerEvent {
    Done(Result<Outcome>),
}
#[derive(Debug)]
enum Outcome {
    Opened(Box<navigation::Content>),
    BrowserDiff {
        files: FileSession,
    },
    Checked(domain::Report),
    Branches(Box<branches::View>),
    Stashes(engine::local::stash::Snapshot),
    ReplayPrepared {
        preparation: Box<engine::operations::replay::Preparation>,
        draft: bar::CommandDraft,
    },
    MutationPrepared {
        plan: Box<engine::operations::mutation::Plan>,
        draft: bar::CommandDraft,
    },
    MutationCompleted(engine::operations::mutation::Change),
    TerminalOpened(PathBuf),
    Files(Files),
    History(Box<history::View>),
    Composed(Box<engine::composition::Prepared>),
    CompositionCommitted(Box<engine::composition::Progress>),
    CommitPrepared(CommitPlan),
    CommitForGeneration(CommitPlan),
    PushPrepared(Box<engine::operations::push::Preparation>),
    Committed(CommitResult),
    Published(Publication),
    Text(String),
    TextWrapConfigured(bool),
}
struct Screen {
    service: Arc<Service>,
    snapshot: Snapshot,
    config: Config,
    page: Page,
    history: Vec<navigation::Visit>,
    mode: Mode,
    focus_mode: focus::State,
    interaction_save: interaction::Writer,
    diff_preference: diff::Preference,
    file_layout: files::Layout,
    graphics: graphics::Display,
    bar: bar::InputBar,
    exit_sequence: input::interrupt::Sequence,
    clipboard: clipboard::Writer,
    link_opener: links::Opener,
    commit_generator: commit_message::Generator,
    commit_draft_id: u64,
    repository_viewport: ratatui::widgets::TableState,
    scroll_input: input::scroll::ScrollInput,
    repository_area: Option<ratatui::layout::Rect>,
    body_area: Option<ratatui::layout::Rect>,
    input_area: Option<ratatui::layout::Rect>,
    pointer: pointer::Map,
    clicks: input::clicks::Clicks,
    drag: Option<pointer::drag::Grab>,
    command_items: Vec<(ratatui::layout::Rect, usize)>,
    command_viewport: view::CandidateViewport,
    breadcrumb_items: Vec<(ratatui::layout::Rect, navigation::breadcrumbs::Destination)>,
    filter: Filter,
    query: String,
    search_mode: search::Mode,
    order: Order,
    setting_selected: usize,
    settings_focus: Option<settings::Focus>,
    input_surface: view::input::Surface,
    scroll: usize,
    message: String,
    tasks: task_center::Center,
    check_task: Option<checks::CheckTask>,
    repository_tasks: BTreeMap<PathBuf, repository_tasks::Task>,
    repository_errors: BTreeMap<PathBuf, String>,
    view_revision: u64,
    updates: BTreeMap<PathBuf, checks::Update>,
    overview: overview::Overview,
    replay: replay::Cache,
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
            focus_mode: Default::default(),
            interaction_save: Default::default(),
            diff_preference: Default::default(),
            file_layout: Default::default(),
            graphics: graphics::Display::new(graphics::Support::Unavailable(
                i18n::tr!(GraphicsUnsupported).into(),
            )),
            bar: bar::InputBar::default(),
            exit_sequence: Default::default(),
            clipboard: Default::default(),
            link_opener: Default::default(),
            commit_generator: Default::default(),
            commit_draft_id: 0,
            repository_viewport,
            scroll_input: input::scroll::ScrollInput::default(),
            repository_area: None,
            body_area: None,
            input_area: None,
            pointer: Default::default(),
            clicks: Default::default(),
            drag: None,
            command_items: Vec::new(),
            command_viewport: Default::default(),
            breadcrumb_items: Vec::new(),
            filter: Filter::All,
            query: String::new(),
            search_mode: Default::default(),
            order: Order::Priority,
            setting_selected: 0,
            settings_focus: None,
            input_surface: Default::default(),
            scroll: 0,
            message: String::new(),
            tasks: Default::default(),
            check_task: None,
            repository_tasks: BTreeMap::new(),
            repository_errors: BTreeMap::new(),
            view_revision: 0,
            updates: BTreeMap::new(),
            overview: Default::default(),
            replay: Default::default(),
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
            if !self.filter.matches(result, self.repository_status(path)) {
                continue;
            }
            if self.search_mode == search::Mode::Filter
                && !path
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
        if let Mode::CompositionWaiting(waiting) = &self.mode {
            return Some(waiting.path.clone());
        }
        if let Mode::Composition(session) = &self.mode {
            return Some(session.prepared.snapshot.path.clone());
        }
        if let Mode::CommitMessage { plan, .. } = &self.mode {
            return Some(plan.path.clone());
        }
        if let Mode::Browser(session) = &self.mode {
            return Some(session.root.clone());
        }
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
        let errors = self
            .repository_errors
            .keys()
            .map(|path| (path.clone(), self.observation_order(path)))
            .collect::<Vec<_>>();
        self.snapshot = self.service.snapshot(cancel)?;
        let ai_enabled = self.config.ai.enabled;
        self.config = self.service.store.config()?;
        if ai_enabled != self.config.ai.enabled {
            self.settings_focus = None;
        }
        self.hide_disabled_ai_settings();
        self.load_updates()?;
        for (path, previous) in errors {
            if self
                .observation_order(&path)
                .is_some_and(|next| previous.is_none_or(|old| next > old))
            {
                self.repository_errors.remove(&path);
            }
        }
        self.focus(path);
        Ok(())
    }
    fn take_mode(&mut self) -> Mode {
        std::mem::replace(&mut self.mode, Mode::Browsing)
    }
}

struct TerminalGuard(engine::state::InteractionMode);
impl TerminalGuard {
    fn enter(mode: engine::state::InteractionMode) -> Result<Self> {
        enable_raw_mode()?;
        let guard = Self(mode);
        execute!(io::stdout(), EnterAlternateScreen, EnableBracketedPaste)?;
        interaction::apply(mode, &mut io::stdout())?;
        Ok(guard)
    }
    fn sync(&mut self, mode: engine::state::InteractionMode) -> Result<()> {
        if self.0 != mode {
            interaction::apply(mode, &mut io::stdout())?;
            self.0 = mode;
        }
        Ok(())
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
pub fn run(service: Service, root: Cancel, start: Start) -> Result<()> {
    anyhow::ensure!(
        io::stdin().is_terminal() && io::stdout().is_terminal(),
        i18n::tr!(InteractiveTerminalRequired)
    );
    let mut screen = start.resolve(&root)?.open(service, &root)?;
    let mut guard = TerminalGuard::enter(screen.config.interaction_mode)?;
    screen.graphics = graphics::Display::detect();
    let mut link_output = links::Native::default();
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    terminal.clear()?;
    let result = (|| -> Result<()> {
        // 先显示上轮结果，再仅在本次启动时发起一次后台检查。
        let frame = terminal.draw(|frame| view::render(frame, &mut screen))?;
        link_output.flush(&mut io::stdout(), frame.buffer, &screen.pointer.links)?;
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
                | screen.poll_links()
                | screen.poll_interaction()
                | screen.poll_tasks()
                | screen.workspace.animate()
                | screen.animate_check()
                | screen.animate_task()
                | screen.graphics.poll();
            let changed =
                changed | screen.overview.poll(&screen.service) | screen.overview.animate();
            guard.sync(screen.config.interaction_mode)?;
            if dirty || changed {
                let frame = terminal.draw(|frame| view::render(frame, &mut screen))?;
                link_output.flush(&mut io::stdout(), frame.buffer, &screen.pointer.links)?;
                screen.graphics.finish_frame(&mut io::stdout())?;
                dirty = false;
            }
            if event::poll(Duration::from_millis(80))? {
                let event = event::read()?;
                match event {
                    TerminalEvent::Key(key) if key.kind != KeyEventKind::Release => {
                        if screen.key(key, &root)? {
                            break;
                        }
                        dirty = true;
                    }
                    TerminalEvent::Mouse(mouse) => {
                        dirty |= screen.mouse(mouse, &root)?;
                    }
                    TerminalEvent::Paste(text) => {
                        screen.paste(&text);
                        dirty = true;
                    }
                    TerminalEvent::Resize(..) => {
                        screen.clicks.reset();
                        screen.drag = None;
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
