//! Bordered input with explicit editable and locked states.
pub(in crate::view) mod actions;
mod argument;
mod conflict;
mod prefix;
mod surface;
use super::command_syntax as syntax;
use super::feedback::Preview;
use super::*;
pub(crate) use surface::Surface;
use unicode_width::UnicodeWidthStr;

const INPUT_ACCENT: Color = Color::Rgb(180, 145, 220);
const INPUT_STYLE: Style = Style::new().bg(Color::Reset).fg(Color::Reset);
const SEARCH_COMMAND_SEPARATOR: &str = " ❘ ";

/// Retain the input presentation throughout a foreground operation.
#[derive(Debug)]
pub(crate) struct Snapshot {
    bar: crate::bar::InputBar,
    search_title: String,
    support: crate::search::Support,
}
impl Snapshot {
    pub fn capture(screen: &mut Screen) -> Self {
        Self {
            search_title: screen.search_title(),
            support: screen.search_support(),
            bar: std::mem::replace(&mut screen.bar, crate::bar::InputBar::CommandsOnly),
        }
    }
}

fn retained(screen: &Screen) -> Option<(&Mode, &Snapshot)> {
    match &screen.mode {
        Mode::Busy(pending) => Some((pending.origin.as_ref(), &pending.input)),
        _ => None,
    }
}
fn input_mode(screen: &Screen) -> &Mode {
    retained(screen).map_or(&screen.mode, |(mode, _)| mode)
}
fn command_hint(input: &command::Input) -> &'static str {
    match input {
        command::Input::Prefix(_) => i18n::tr!(ContextPrefixHint),
        command::Input::Scoped { editor, .. } if editor.text.is_empty() => {
            i18n::tr!(ContextGlobalCommandHint)
        }
        command::Input::Scoped { .. } => i18n::tr!(ContextCommandHint),
        command::Input::Global(editor) if editor.text == "/" => i18n::tr!(CommandHistoryEntryHint),
        command::Input::Global(_) => i18n::tr!(TabCompleteCandidatesEnterRunEscClear),
    }
}
fn search_hint(support: crate::search::Support) -> String {
    let navigation = match support.mode() {
        Some(crate::search::Mode::Locate) => i18n::tr!(SearchLocateHint),
        _ => i18n::tr!(SearchEditingHint),
    };
    format!("{} · {navigation}", i18n::tr!(SearchCommandOnly))
}
fn task_hint(screen: &Screen) -> Option<&'static str> {
    match &screen.mode {
        Mode::Busy(pending)
            if pending.feedback.visible() || pending.state == TaskState::Cancelling =>
        {
            Some(i18n::tr!(EscCtrlCCancelAndWaitFor))
        }
        _ => None,
    }
}

pub(super) fn active_accent(screen: &Screen) -> Color {
    if screen.focus_mode == crate::focus::State::Selecting {
        return ACCENT;
    }
    match screen.config.interaction_mode {
        engine::state::InteractionMode::TextSelection => GOOD,
        engine::state::InteractionMode::Application => INPUT_ACCENT,
    }
}

pub(super) fn connect_menu(frame: &mut Frame, screen: &Screen, input: Rect) {
    let style = Style::default().fg(active_accent(screen));
    // 只替换连接处的两个角，保留输入框已经绘制的标题和顶边。
    frame.buffer_mut()[(input.x, input.y)]
        .set_symbol("├")
        .set_style(style);
    frame.buffer_mut()[(input.right() - 1, input.y)]
        .set_symbol("┤")
        .set_style(style);
}

/// Bottom chrome follows the active input surface instead of always reserving a command bar.
pub(super) enum Footer {
    InputBar {
        editor: u16,
        hint: u16,
        preview: Preview,
    },
    Compact {
        hint: &'static str,
        preview: Preview,
    },
}
impl Footer {
    pub fn for_screen(screen: &Screen, area: Rect) -> Self {
        let preview = Preview::fit(area.width, u16::from(area.height >= 4));
        if matches!(preview, Preview::Hidden)
            && let Some(hint) = screen.exit_sequence.notice()
        {
            // 极矮窗口没有结果栏时，复用现有输入行，保证退出前能看到提示。
            return Self::Compact { hint, preview };
        }
        match input_mode(screen) {
            Mode::Composition(session)
                if matches!(session.state, crate::composition::State::Editing(_)) =>
            {
                Self::Compact {
                    hint: task_hint(screen).unwrap_or(i18n::tr!(ComposeEditHint)),
                    preview,
                }
            }
            Mode::PromptEditor(_) => Self::Compact {
                hint: task_hint(screen).unwrap_or(i18n::tr!(AgentPromptEditorHint)),
                preview,
            },
            _ => {
                let (editor, hint) = match area.height {
                    14.. => (3, 1),
                    9.. => (3, 0),
                    _ => (area.height.min(1), 0),
                };
                Self::InputBar {
                    editor,
                    hint,
                    preview,
                }
            }
        }
    }
    pub fn height(&self) -> u16 {
        match self {
            Self::InputBar {
                editor,
                hint,
                preview,
            } => editor + hint + preview.height(),
            Self::Compact { preview, .. } => 1 + preview.height(),
        }
    }
    pub fn render(&self, frame: &mut Frame, screen: &mut Screen, area: Rect) {
        match self {
            Self::InputBar {
                editor,
                hint,
                preview,
            } => {
                let parts = Layout::vertical([
                    Constraint::Length(*editor),
                    Constraint::Length(*hint),
                    Constraint::Length(preview.height()),
                ])
                .split(area);
                screen.input_area = Some(parts[0]);
                render(frame, screen, parts[0], parts[1]);
                preview.render(frame, screen, parts[2]);
            }
            Self::Compact { hint, preview } => {
                if area.is_empty() {
                    return;
                }
                render_hint(frame, Rect::new(area.x, area.y, area.width, 1), hint);
                preview.render(
                    frame,
                    screen,
                    Rect::new(
                        area.x,
                        area.y + 1,
                        area.width,
                        area.height.saturating_sub(1),
                    ),
                );
            }
        }
    }
}
fn editor_block(area: Rect) -> Block<'static> {
    if area.height >= 3 {
        Block::bordered().border_type(ratatui::widgets::BorderType::Rounded)
    } else {
        Block::default()
    }
}
fn styled_editor_block(area: Rect, accent: Color) -> Block<'static> {
    editor_block(area)
        .title_style(Style::default().fg(accent))
        .border_style(Style::default().fg(accent))
        .style(INPUT_STYLE)
}

fn prompt() -> Span<'static> {
    Span::styled("› ", Style::default().fg(INPUT_ACCENT))
}

fn plain_value(
    editor: &Editor,
    visible: std::ops::Range<usize>,
    placeholder: &str,
    width: usize,
) -> Span<'static> {
    if editor.text.is_empty() {
        Span::styled(
            truncate(placeholder, width),
            Style::default().fg(MUTED).add_modifier(Modifier::ITALIC),
        )
    } else {
        Span::styled(editor.text[visible].to_owned(), INPUT_STYLE)
    }
}

fn render_editable(
    frame: &mut Frame,
    screen: &Screen,
    inner: Rect,
    editor: &Editor,
    command: Option<&command::Input>,
    placeholder: &str,
    command_search: Option<&str>,
    show_cursor: bool,
) -> Surface {
    let prefix = command
        .and_then(|input| input.context())
        .map(|context| {
            let prefix = format!("/{}", context.name());
            let budget = inner.width.saturating_sub(4) as usize;
            if prefix.width() <= budget {
                prefix
            } else {
                truncate(&prefix, budget)
            }
        })
        .filter(|prefix| !prefix.is_empty());
    let prefix_width = prefix
        .as_deref()
        .map_or(0, |prefix| prefix.width() as u16 + 1);
    let mut spans = Vec::new();
    let mut surface = Surface::Plain;
    let prompt = prompt();
    // 缩窄时优先给命令留空间，搜索词仍保存在当前视图。
    let search_budget = (inner.width / 3).min(inner.width.saturating_sub(prefix_width + 11));
    let search_width = if let Some(query) = command_search.filter(|_| search_budget > 0) {
        let mut query = safe(query);
        if query.width() > usize::from(search_budget) {
            query = truncate(&query, search_budget.into());
        }
        let width = query.width() as u16 + 5;
        spans.push(prompt.clone());
        spans.push(Span::styled(query, INPUT_STYLE));
        spans.push(Span::styled(
            SEARCH_COMMAND_SEPARATOR,
            Style::default().fg(MUTED),
        ));
        width
    } else {
        0
    };
    if prefix.is_some() {
        surface = Surface::Prefix(Rect::new(
            inner.x + search_width,
            inner.y,
            prefix_width.min(inner.width),
            inner.height.min(1),
        ));
    }
    let show_command_prompt = prefix.is_some() || search_width == 0;
    let prompt_width = search_width + prefix_width + if show_command_prompt { 2 } else { 0 };
    let width = inner.width.saturating_sub(prompt_width) as usize;
    let (visible, cursor) = editor.viewport_range(width);
    let value = if let Some(command) = command.filter(|_| !editor.text.is_empty()) {
        syntax::spans(command, visible)
    } else {
        vec![plain_value(editor, visible, placeholder, width)]
    };
    if let Some(prefix) = prefix {
        spans.extend(prefix::spans(
            &prefix,
            command.and_then(|input| input.prefix_selection()),
            screen.background,
        ));
        spans.push(Span::raw(" "));
    }
    if show_command_prompt {
        spans.push(prompt);
    }
    spans.extend(value);
    if let Some(hint) = command.and_then(|input| {
        argument::hint(
            input,
            &screen.config,
            width.saturating_sub(usize::from(cursor)),
        )
    }) {
        spans.push(hint);
    }
    let cursor = (show_cursor && !command.is_some_and(|input| input.prefix_selected()))
        .then_some(prompt_width.saturating_add(cursor));
    paint_editor(frame, inner, spans, cursor);
    surface
}

pub(super) fn render_task_input(
    frame: &mut Frame,
    screen: &mut Screen,
    area: Rect,
    hint_area: Rect,
    view_hint: &str,
) {
    let bar = screen.active_bar();
    let search = &screen.tasks.search.text;
    let command = bar.command_input();
    let command_search = command.and((!search.is_empty()).then_some(search.as_str()));
    let title = match bar {
        crate::bar::InputBar::Command { .. } if command_search.is_some() => {
            format!(
                "{}{SEARCH_COMMAND_SEPARATOR}{}",
                i18n::tr!(TasksSearchTitle),
                i18n::tr!(CommandsAlt)
            )
        }
        crate::bar::InputBar::Command { .. } => i18n::tr!(CommandsAlt).into(),
        crate::bar::InputBar::Search(_) => i18n::tr!(TasksSearchTitle).into(),
        crate::bar::InputBar::CommandsOnly => String::new(),
    };
    let mut block = styled_editor_block(area, active_accent(screen));
    if area.height >= 3 && !title.is_empty() {
        block = block.title(format!(" {title} "));
    }
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let placeholder = match bar {
        crate::bar::InputBar::Command { .. } => i18n::tr!(ContextCommandPlaceholder),
        crate::bar::InputBar::Search(_) => i18n::tr!(TasksSearchPlaceholder),
        crate::bar::InputBar::CommandsOnly => i18n::tr!(SearchCommandOnly),
    };
    let surface = render_editable(
        frame,
        screen,
        inner,
        bar.editor(),
        command,
        placeholder,
        command_search,
        true,
    );
    let hint = screen.command_enter_hint();
    let hint = hint
        .as_deref()
        .or_else(|| command.map(command_hint))
        .unwrap_or(view_hint);
    render_hint(frame, hint_area, hint);
    screen.input_surface.update(frame, surface);
}

fn paint_editor(frame: &mut Frame, inner: Rect, spans: Vec<Span<'static>>, cursor: Option<u16>) {
    frame.render_widget(Paragraph::new(Line::from(spans)).style(INPUT_STYLE), inner);
    if !inner.is_empty()
        && let Some(offset) = cursor
    {
        frame.set_cursor_position((
            inner.x.saturating_add(offset).min(inner.right() - 1),
            inner.y,
        ));
    }
}

pub(crate) fn content_area(area: Rect) -> Rect {
    editor_block(area).inner(area)
}

enum Content<'a> {
    Actions {
        items: Vec<actions::Item>,
    },
    CommandEntry,
    Editable {
        editor: &'a Editor,
        placeholder: &'static str,
    },
    Locked(String),
    StashActions {
        session: &'a crate::stash_save::Session,
    },
    CommitActions {
        draft: &'a crate::commit_form::Draft,
    },
    Conflict {
        selected: crate::conflict::controls::Action,
    },
    Recovery {
        selected: engine::operations::replay::Control,
        enabled: [bool; 3],
    },
    Choices {
        choice: confirmation::Choice,
        execute: &'static str,
        back: &'static str,
    },
}

fn browsing_hint(screen: &Screen) -> &'static str {
    if screen.focused_conflict_control().is_some() {
        return i18n::tr!(ConflictControlsHint);
    }
    if screen.focused_replay_control().is_some() {
        return i18n::tr!(ReplayControlsHint);
    }
    if matches!(input_mode(screen), Mode::Files(files) if files.horizontal.overflowing())
        || matches!(input_mode(screen), Mode::Changes(session) if session.focus == crate::changes::Focus::Files && session.files.horizontal.overflowing())
    {
        return i18n::tr!(WorktreeHorizontalHint);
    }
    match input_mode(screen) {
        Mode::Browsing | Mode::RepositoryEnvironment(_) => match screen.page {
            Page::Repositories => i18n::tr!(RepositoryListHint),
            Page::Detail => i18n::tr!(RepositoryPanesHint),
            Page::Configuration(Configuration::Schedule) => i18n::tr!(ScheduleHelpHint),
            Page::Configuration(_) => i18n::tr!(SelectEnterEditBackCommands),
            Page::Help => i18n::tr!(PgdnScrollEscBackCommands),
            Page::View(_) => i18n::tr!(PaneReadHint),
        },
        Mode::Panel(crate::workspace::Section::Overview) => i18n::tr!(OverviewInputHint),
        Mode::Panel(_) => i18n::tr!(PaneReadHint),
        Mode::Browser(session) => match session.focus {
            crate::browser::Focus::Tree => i18n::tr!(BrowserHint),
            crate::browser::Focus::Preview if screen.config.text_wrap => {
                i18n::tr!(BrowserWrappedPreviewHint)
            }
            crate::browser::Focus::Preview => i18n::tr!(BrowserPreviewHint),
        },
        Mode::Branches(view) => match view.as_ref() {
            crate::branches::View::List(_) => i18n::tr!(BranchHint),
            crate::branches::View::Detail { .. } => i18n::tr!(OverviewInputHint),
        },
        Mode::Stashes(view) => match view.as_ref() {
            crate::stashes::View::List(_) => i18n::tr!(StashHint),
            crate::stashes::View::Detail { inspect, .. } => match inspect.focus {
                crate::inspection::Focus::Files => i18n::tr!(StashPreviewHint),
                crate::inspection::Focus::Message => i18n::tr!(PgdnScrollEscBackCommands),
                crate::inspection::Focus::Preview => i18n::tr!(HistoryReviewInputHint),
            },
            crate::stashes::View::Diff { content, .. }
                if matches!(content.as_ref(), crate::document::View::Images(_)) =>
            {
                i18n::tr!(ImageInputHint)
            }
            crate::stashes::View::Diff { .. } if screen.config.text_wrap => {
                i18n::tr!(StashWrappedDiffHint)
            }
            crate::stashes::View::Diff { .. } => i18n::tr!(StashDiffHint),
        },
        Mode::History(view) => match view.as_ref() {
            crate::history::View::Graph(session)
                if session.focus == crate::history::GraphFocus::Files =>
            {
                i18n::tr!(PgdnScrollEscBackCommands)
            }
            crate::history::View::Graph(session) if session.selection.active() => {
                i18n::tr!(HistorySelectionHint)
            }
            crate::history::View::Graph(_) => i18n::tr!(HistoryInputHint),
            crate::history::View::Commit(commit) => match commit.inspect.focus {
                crate::history::Focus::Files => i18n::tr!(HistoryCommitInputHint),
                crate::history::Focus::Message => i18n::tr!(PgdnScrollEscBackCommands),
                crate::history::Focus::Preview => i18n::tr!(HistoryReviewInputHint),
            },
            crate::history::View::Diff { content, .. }
                if matches!(content.as_ref(), crate::document::View::Images(_)) =>
            {
                i18n::tr!(ImageInputHint)
            }
            crate::history::View::Diff { .. } if screen.config.text_wrap => {
                i18n::tr!(HistoryWrappedDiffHint)
            }
            crate::history::View::Diff { .. } => i18n::tr!(HistoryDiffHint),
        },
        Mode::Composition(_) => i18n::tr!(ComposeHint),
        Mode::CompositionWaiting(waiting) => waiting.hint(),
        Mode::CommitMessage { editor, .. } if editor.generating() => {
            i18n::tr!(CommitGeneratingHint)
        }
        Mode::CommitMessage { editor, .. } if editor.submitting() => {
            i18n::tr!(CommitSubmittingHint)
        }
        Mode::CommitMessage { editor, .. } if editor.focused() && screen.config.ai.enabled => {
            i18n::tr!(AgentHint)
        }
        Mode::CommitMessage { editor, .. } if editor.focused() => i18n::tr!(CommitEditingHint),
        Mode::CommitMessage { .. } => i18n::tr!(CommitActionsHint),
        Mode::Management(session)
            if matches!(session.focus, crate::management::Focus::Actions(_)) =>
        {
            i18n::tr!(ScanActionsHint)
        }
        Mode::Management(session) => match session.source {
            crate::management::Source::Managed => i18n::tr!(ManageHint),
            crate::management::Source::Scan { .. } => i18n::tr!(ScanHint),
        },
        Mode::Files(files) if matches!(files.cursor, crate::files::Cursor::Directory { .. }) => {
            i18n::tr!(WorktreeDirectoryInputHint)
        }
        Mode::Files(_) => i18n::tr!(SelectSpaceInsMarkCtrlAAll),
        Mode::Changes(session) => match session.focus {
            crate::changes::Focus::Files
                if matches!(session.files.cursor, crate::files::Cursor::Directory { .. }) =>
            {
                i18n::tr!(DirectoryInputHint)
            }
            crate::changes::Focus::Files if screen.config.text_wrap => {
                i18n::tr!(WrappedChangesInputHint)
            }
            crate::changes::Focus::Files => i18n::tr!(ChangesInputHint),
            crate::changes::Focus::Preview if screen.partial_mode().is_some() => {
                match screen.partial_mode().expect("editable preview") {
                    crate::changes::edit::Granularity::Hunk => i18n::tr!(PartialHunkHint),
                    crate::changes::edit::Granularity::Line => i18n::tr!(PartialLineHint),
                }
            }
            crate::changes::Focus::Preview if matches!(&session.preview, crate::changes::Preview::Ready {content,..} if matches!(content.as_ref(), crate::document::View::Conflict(_))) => {
                if screen
                    .conflict_view()
                    .is_some_and(|view| view.resolution.resolved())
                {
                    i18n::tr!(ConflictResolvedHint)
                } else if screen
                    .conflict_view()
                    .is_some_and(|view| view.resolution.reason().is_some())
                {
                    i18n::tr!(ConflictReadingHint)
                } else {
                    i18n::tr!(ConflictInputHint)
                }
            }
            crate::changes::Focus::Preview if screen.config.text_wrap => {
                i18n::tr!(WrappedDiffPreviewHint)
            }
            crate::changes::Focus::Preview => i18n::tr!(DiffPreviewInputHint),
        },
        _ => unreachable!("browsing hints require a browsing view"),
    }
}

pub(super) fn render(frame: &mut Frame, screen: &mut Screen, area: Rect, hint_area: Rect) {
    let (mode, bar, support, search_title) = match retained(screen) {
        Some((mode, input)) => (mode, &input.bar, input.support, input.search_title.clone()),
        None => (
            &screen.mode,
            &screen.bar,
            screen.search_support(),
            screen.search_title(),
        ),
    };
    let search_hint = match bar {
        crate::bar::InputBar::Search(editor) if !editor.text.is_empty() => {
            Some(search_hint(support))
        }
        _ => None,
    };
    let (title, content, hint) = if screen.focus_mode == crate::focus::State::Selecting {
        (
            i18n::tr!(PaneFocusTitle),
            Content::Locked(screen.focused_pane_name().into()),
            i18n::tr!(PaneFocusHint),
        )
    } else if let Mode::History(view) = mode
        && let Some(menu) = view.session().selection.menu()
    {
        (
            i18n::tr!(HistoryActionsTitle),
            Content::Locked(i18n::tf!(HistoryActionsCount, count = menu.count())),
            i18n::tr!(HistoryActionsHint),
        )
    } else if let Mode::Management(session) = mode
        && let crate::management::Focus::Actions(crate::management::Controls { selected, .. }) =
            session.focus
    {
        (
            i18n::tr!(ScanActionsTitle),
            Content::Actions {
                items: actions::management(session, selected),
            },
            i18n::tr!(ScanActionsHint),
        )
    } else if let Mode::Workflow(session) = mode {
        (
            if session.page() == crate::input_pages::Pair::Content {
                i18n::tr!(WorkflowControls)
            } else {
                i18n::tr!(WorkflowFinishTitle)
            },
            Content::Actions {
                items: actions::workflow(session),
            },
            if session.page() == crate::input_pages::Pair::Content {
                i18n::tr!(WorkflowEditorHint)
            } else {
                i18n::tr!(WorkflowFinishHint)
            },
        )
    } else if let Some(selected) = screen.focused_conflict_control() {
        (
            i18n::tr!(ConflictControlsTitle),
            Content::Conflict { selected },
            i18n::tr!(ConflictControlsHint),
        )
    } else if let Some(selected) = screen.focused_replay_control() {
        let (_, operation) = screen
            .replay_controls()
            .expect("focused recovery operation");
        (
            i18n::tr!(ReplayControlsTitle),
            Content::Recovery {
                selected,
                enabled: crate::replay::controls::ACTIONS
                    .map(|control| crate::replay::controls::enabled(operation, control)),
            },
            i18n::tr!(ReplayControlsHint),
        )
    } else if let Mode::Composition(session) = mode
        && !matches!(session.state, crate::composition::State::Browsing)
    {
        match &session.state {
            crate::composition::State::Confirm(choice) => (
                i18n::tr!(ConfirmOperationAlt),
                Content::Choices {
                    choice: *choice,
                    execute: i18n::tr!(ComposeNext),
                    back: i18n::tr!(ActionReturnEdit),
                },
                i18n::tr!(ConfirmSelectHint),
            ),
            crate::composition::State::Moving { target, .. } => (
                i18n::tr!(ComposeMove),
                Content::Locked(
                    session.prepared.plan.commits[*target]
                        .message
                        .lines()
                        .next()
                        .unwrap_or("")
                        .into(),
                ),
                i18n::tr!(ComposeMoveHint),
            ),
            crate::composition::State::Submitting => (
                i18n::tr!(ComposeTitle),
                Content::Locked(i18n::tr!(CommitSubmitting).into()),
                i18n::tr!(CommitSubmittingHint),
            ),
            _ => unreachable!("composition text editing has a compact footer"),
        }
    } else {
        match mode {
            Mode::Management(_)
            | Mode::Browsing
            | Mode::RepositoryEnvironment(_)
            | Mode::Composition(_)
            | Mode::CompositionWaiting(_)
            | Mode::Browser(_)
            | Mode::Panel(_)
            | Mode::Stashes(_)
            | Mode::Branches(_)
            | Mode::Files(_)
            | Mode::Changes(_)
            | Mode::History(_) => {
                let command = matches!(bar, crate::bar::InputBar::Command { .. });
                let hint = if matches!(
                    bar,
                    crate::bar::InputBar::Command {
                        menu: command::completion::Menu::History(_),
                        ..
                    }
                ) {
                    i18n::tr!(CommandHistoryHint)
                } else if matches!(
                    bar,
                    crate::bar::InputBar::Command {
                        menu: command::completion::Menu::Loading(_),
                        ..
                    }
                ) && screen.command_reader.visible()
                {
                    i18n::tr!(ReadingCommandCandidates)
                } else if let crate::bar::InputBar::Command {
                    menu: command::completion::Menu::Values(values),
                    ..
                } = bar
                {
                    match values.kind {
                        command::values::Kind::Editor => i18n::tr!(OptionCompletionHint),
                        _ => i18n::tr!(ValueCompletionHint),
                    }
                } else if matches!(
                    bar,
                    crate::bar::InputBar::Command {
                        menu: command::completion::Menu::Options(_),
                        ..
                    }
                ) {
                    i18n::tr!(OptionCompletionHint)
                } else if matches!(
                    bar,
                    crate::bar::InputBar::Command {
                        menu: command::completion::Menu::Directories(_),
                        ..
                    }
                ) {
                    i18n::tr!(DirectoryCompletionHint)
                } else if command {
                    match command::argument_input(
                        bar.text().strip_prefix('/').expect("command prefix"),
                    ) {
                        Some((argument, _)) if argument.action().available(&screen.config) => {
                            argument.hint()
                        }
                        _ => command_hint(bar.command_input().expect("command input")),
                    }
                } else if let Some(hint) = search_hint.as_deref() {
                    hint
                } else {
                    browsing_hint(screen)
                };
                (
                    if matches!(bar, crate::bar::InputBar::CommandsOnly) {
                        ""
                    } else {
                        i18n::tr!(CommandsAlt)
                    },
                    if matches!(bar, crate::bar::InputBar::CommandsOnly) {
                        Content::CommandEntry
                    } else {
                        Content::Editable {
                            editor: bar.editor(),
                            placeholder: if command {
                                i18n::tr!(ContextCommandPlaceholder)
                            } else if matches!(mode, Mode::Browser(session) if session.focus == crate::browser::Focus::Preview)
                            {
                                i18n::tr!(ContentSearchPlaceholder)
                            } else if matches!(mode, Mode::Browser(_)) {
                                i18n::tr!(BrowserPlaceholder)
                            } else if matches!(mode, Mode::History(view) if matches!(view.as_ref(), crate::history::View::Graph(_)))
                            {
                                i18n::tr!(HistorySearchHint)
                            } else {
                                i18n::tr!(TypeToSearchOrForCommands)
                            },
                        }
                    },
                    hint,
                )
            }
            Mode::PromptEditor(_) | Mode::Workflow(_) => {
                unreachable!("the editor has a compact footer")
            }
            Mode::StashSave(session) => (
                i18n::tr!(ConfirmOperationAlt),
                Content::StashActions { session },
                i18n::tr!(StashSaveHint),
            ),
            Mode::CommitMessage { editor, .. } => (
                i18n::tr!(ConfirmOperationAlt),
                Content::CommitActions { draft: editor },
                browsing_hint(screen),
            ),
            Mode::Confirm {
                operation:
                    Confirmation::ReplayParents {
                        candidate,
                        selected,
                        ..
                    },
                ..
            } => (
                i18n::tr!(ReplayParentsTitle),
                Content::Locked(format!(
                    "{} {}",
                    i18n::tr!(ReplayMainline),
                    candidate.parents[*selected].number
                )),
                i18n::tr!(ReplayParentsHint),
            ),
            Mode::Confirm { operation, choice } => (
                i18n::tr!(ConfirmOperationAlt),
                Content::Choices {
                    choice: *choice,
                    execute: match operation {
                        Confirmation::ConflictReset { .. } => i18n::tr!(ConflictResetCommand),
                        Confirmation::DiscardFile { .. } => i18n::tr!(DiscardFile),
                        Confirmation::Partial { .. } => i18n::tr!(PartialDiscard),
                        Confirmation::Schedule(operation) => match operation {
                            engine::scheduler::Control::Enable => i18n::tr!(EnableScheduledTask),
                            engine::scheduler::Control::Disable => i18n::tr!(DisableScheduledTask),
                            engine::scheduler::Control::Apply => i18n::tr!(ActionApply),
                        },
                        Confirmation::Mutation { .. } => i18n::tr!(MutationRun),
                        Confirmation::ReplayParents { .. } => {
                            unreachable!("parent chooser has its own footer")
                        }
                        Confirmation::Remove { path: _, .. } => i18n::tr!(ActionUnregister),
                        Confirmation::Configure { command, .. } => match command {
                            command::ConfigArgument::Add => i18n::tr!(ActionAdd),
                            _ => i18n::tr!(ActionSave),
                        },
                    },
                    back: match operation {
                        Confirmation::Mutation { draft, .. }
                            if matches!(draft.return_to, crate::bar::ReturnTo::View) =>
                        {
                            i18n::tr!(HistorySelectionBack)
                        }
                        Confirmation::Configure { .. } | Confirmation::Mutation { .. } => {
                            i18n::tr!(ActionReturnEdit)
                        }
                        _ => i18n::tr!(ActionCancel),
                    },
                },
                if matches!(operation, Confirmation::DiscardFile { .. }) {
                    i18n::tr!(ConfirmReviewHint)
                } else {
                    i18n::tr!(ConfirmSelectHint)
                },
            ),
            Mode::PushTarget(session) => (
                i18n::tr!(PushTargetBranch),
                if session.editing() {
                    Content::Editable {
                        editor: session.editor(),
                        placeholder: i18n::tr!(PushTargetBranch),
                    }
                } else {
                    Content::Locked(session.editor().text.clone())
                },
                i18n::tr!(PushTargetHint),
            ),
            Mode::PushConfirm { choice, plan, .. } => (
                i18n::tr!(ConfirmPushAlt),
                Content::Choices {
                    choice: *choice,
                    execute: if plan.force_behind().is_some() {
                        i18n::tr!(PushForceTitle)
                    } else {
                        i18n::tr!(ActionPush)
                    },
                    back: i18n::tr!(ActionCancel),
                },
                i18n::tr!(ConfirmReviewHint),
            ),
            Mode::Error { .. } if bar.command_input().is_some() => (
                i18n::tr!(CommandsAlt),
                Content::Editable {
                    editor: bar.editor(),
                    placeholder: i18n::tr!(ContextCommandPlaceholder),
                },
                command_hint(bar.command_input().expect("error command input")),
            ),
            Mode::Error { .. } => ("", Content::CommandEntry, i18n::tr!(CopyErrorHint)),
            Mode::Busy(_) => (
                "",
                Content::CommandEntry,
                i18n::tr!(EscCtrlCCancelAndWaitFor),
            ),
        }
    };
    let command_hint = if screen.focus_mode == crate::focus::State::Inactive
        && matches!(bar, crate::bar::InputBar::Command { .. })
        && matches!(content, Content::Editable { .. })
    {
        screen.command_enter_hint()
    } else {
        None
    };
    let hint = command_hint.as_deref().unwrap_or(hint);
    let task_hint = task_hint(screen);
    let hint = task_hint.unwrap_or(hint);
    let hint = if task_hint.is_none()
        && (bar.is_plain() || matches!(bar, crate::bar::InputBar::Command { .. }))
        && (screen.replay_controls().is_some()
            || matches!(&screen.mode, Mode::Management(session) if matches!(session.source, crate::management::Source::Scan { .. }) && session.focus.is_input() && (bar.command_input().is_some() || !bar.editor().text.is_empty()))
            || matches!(&screen.mode, Mode::History(view) if matches!(view.as_ref(), crate::history::View::Graph(session) if session.selection.active() && !session.picks().is_empty())))
        && screen.focused_replay_control().is_none()
        && screen.focused_conflict_control().is_none()
    {
        format!("{} · {hint}", i18n::tr!(ReplayControlsEntryHint))
    } else {
        hint.to_owned()
    };
    let show_cursor = !matches!(&content, Content::Actions { .. })
        && task_hint.is_none()
        && screen.focused_replay_control().is_none()
        && screen.focused_conflict_control().is_none();
    let command_search = if bar.command_input().is_some() && support.mode().is_some() {
        screen
            .search_query_in(mode)
            .filter(|query| !query.is_empty())
    } else {
        None
    };
    let title = if (matches!(bar, crate::bar::InputBar::Search(_)) || command_search.is_some())
        && screen.focus_mode == crate::focus::State::Inactive
        && matches!(content, Content::Editable { .. })
        && !matches!(mode, Mode::CommitMessage { .. })
    {
        if command_search.is_some() {
            format!(
                "{search_title}{SEARCH_COMMAND_SEPARATOR}{}",
                i18n::tr!(CommandsAlt).trim()
            )
        } else {
            search_title
        }
    } else {
        title.to_owned()
    };
    let active = !matches!(mode, Mode::StashSave(session) if session.focus.is_input())
        && !matches!(mode, Mode::CommitMessage { editor, .. } if editor.focused())
        && matches!(
            content,
            Content::CommandEntry
                | Content::Editable { .. }
                | Content::Choices { .. }
                | Content::Recovery { .. }
                | Content::Actions { .. }
                | Content::Conflict { .. }
                | Content::CommitActions { .. }
                | Content::StashActions { .. }
        );
    let input_style = INPUT_STYLE;
    let accent = if active {
        active_accent(screen)
    } else if screen.config.interaction_mode == engine::state::InteractionMode::TextSelection {
        GOOD
    } else {
        MUTED
    };
    let mut block = styled_editor_block(area, accent);
    if area.height >= 3 {
        let recovery = screen
            .replay_controls()
            .filter(|_| bar.is_plain() && screen.focus_mode == crate::focus::State::Inactive);
        if let Some((_, operation)) = recovery {
            let mut spans = vec![Span::raw(if title.trim().is_empty() {
                format!(" {}", operation.kind.label())
            } else {
                format!(" {} · {}", title.trim(), operation.kind.label())
            })];
            if area.width >= 90 {
                spans.push(Span::styled(
                    format!(" · {}", &operation.commit.to_string()[..8]),
                    Style::default().fg(MUTED),
                ));
            }
            spans.push(Span::styled(
                format!(
                    " · {}: {}",
                    i18n::tr!(ReplayConflictsShort),
                    operation.conflicts.len()
                ),
                Style::default().fg(if operation.conflicts.is_empty() {
                    GOOD
                } else {
                    BAD
                }),
            ));
            spans.push(Span::styled(
                format!(
                    " · {}: {} ",
                    i18n::tr!(ReplayRemainingShort),
                    operation.remaining().len()
                ),
                Style::default().fg(MUTED),
            ));
            block = block.title(Line::from(spans));
        } else if !title.trim().is_empty() {
            block = block.title(format!(" {} ", title.trim()));
        }
    }
    if area.height >= 3
        && let Some((current, total)) = screen.footer_page_number()
    {
        block = block.title_top(Line::from(format!(" {current}/{total} ")).right_aligned());
    }
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let mut surface = Surface::Plain;
    match content {
        Content::Actions { items } => {
            let selected = items.iter().find(|item| {
                matches!(
                    item.state,
                    actions::State::Selected | actions::State::SelectedDisabled
                )
            });
            surface = Surface::Actions {
                area: Rect::new(inner.x, inner.y, inner.width, inner.height.min(1)),
                selected: selected.map(|item| (item.target.hit(), item.label)),
            };
            actions::render(frame, inner, &items, &mut screen.pointer);
        }
        Content::CommandEntry => {
            let width = inner.width.saturating_sub(2) as usize;
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("› ", Style::default().fg(INPUT_ACCENT)),
                    Span::styled(
                        truncate(i18n::tr!(SearchCommandOnly), width),
                        Style::default().fg(MUTED),
                    ),
                ]))
                .style(input_style),
                inner,
            );
            if !inner.is_empty() && show_cursor {
                frame.set_cursor_position(((inner.x + 2).min(inner.right() - 1), inner.y));
            }
        }
        Content::Editable {
            editor,
            placeholder,
        } => {
            surface = render_editable(
                frame,
                screen,
                inner,
                editor,
                bar.command_input(),
                placeholder,
                command_search,
                show_cursor,
            );
        }
        Content::Conflict { selected } => {
            surface = Surface::Conflict {
                area: Rect::new(inner.x, inner.y, inner.width, inner.height.min(1)),
                selected,
            };
            conflict::render(frame, screen, inner, selected);
        }
        Content::Recovery { selected, enabled } => {
            surface = Surface::Recovery {
                area: Rect::new(inner.x, inner.y, inner.width, inner.height.min(1)),
                selected,
            };
            super::replay::controls::render(frame, inner, selected, enabled, &mut screen.pointer);
        }
        Content::StashActions { session } => {
            for (area, choice) in super::stash_save::actions(inner) {
                let label = if choice == confirmation::Choice::Execute {
                    i18n::tr!(ActionSave)
                } else {
                    i18n::tr!(ActionCancel)
                };
                frame.render_widget(
                    Paragraph::new(Span::styled(
                        format!(" {label} "),
                        if session.focus == crate::stash_save::Focus::Actions(choice) {
                            Style::default().fg(Color::Black).bg(INPUT_ACCENT).bold()
                        } else {
                            Style::default().fg(Color::Gray)
                        },
                    )),
                    area,
                );
            }
        }
        Content::CommitActions { draft } => {
            for (area, action) in super::commit::options(inner, draft, screen.config.ai.enabled) {
                frame.render_widget(
                    Paragraph::new(Span::styled(
                        format!(" {} ", action.label()),
                        if draft.selected() == Some(action) {
                            Style::default().fg(Color::Black).bg(INPUT_ACCENT).bold()
                        } else {
                            Style::default().fg(Color::Gray)
                        },
                    )),
                    area,
                );
            }
        }
        Content::Choices {
            choice,
            execute,
            back,
        } => {
            surface = Surface::Confirmation {
                area: Rect::new(inner.x, inner.y, inner.width, inner.height.min(1)),
                choice,
            };
            let button = |label: &str, focused: bool| {
                Span::styled(
                    format!(" {label} "),
                    if focused {
                        Style::default()
                            .fg(Color::Black)
                            .bg(INPUT_ACCENT)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Gray).bg(Color::Reset)
                    },
                )
            };
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::raw(" "),
                    button(execute, choice == confirmation::Choice::Execute),
                    Span::raw("   "),
                    button(back, choice == confirmation::Choice::Return),
                ])),
                inner,
            );
        }
        Content::Locked(text) => frame.render_widget(
            Paragraph::new(truncate(&text, inner.width as usize)).style(Style::default().fg(MUTED)),
            inner,
        ),
    }
    screen.input_surface.update(frame, surface);
    render_hint(frame, hint_area, &hint);
}
fn render_hint(frame: &mut Frame, area: Rect, hint: &str) {
    let hint_area = footer_text_area(area);
    frame.render_widget(
        Paragraph::new(truncate(hint, hint_area.width as usize)).style(Style::default().fg(MUTED)),
        hint_area,
    );
}
