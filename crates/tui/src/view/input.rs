//! Bordered input with explicit editable and locked states.
mod prefix;
mod surface;
use super::feedback::Preview;
use super::*;
pub(crate) use surface::Surface;

const INPUT_ACCENT: Color = Color::Rgb(180, 145, 220);

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
        command::Input::Global(_) => i18n::tr!(TabCompleteCandidatesEnterRunEscClear),
    }
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
            Mode::Workflow(_) => Self::Compact {
                hint: task_hint(screen).unwrap_or(i18n::tr!(WorkflowEditorHint)),
                preview,
            },
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
pub(crate) fn content_area(area: Rect) -> Rect {
    editor_block(area).inner(area)
}

enum Content<'a> {
    CommandEntry,
    Editable {
        editor: &'a Editor,
        placeholder: &'static str,
    },
    Locked(String),
    Layout {
        selected: crate::diff::Preference,
    },
    CommitActions {
        draft: &'a crate::commit_form::Draft,
    },
    Choices {
        choice: confirmation::Choice,
        execute: &'static str,
        back: &'static str,
    },
}

fn browsing_hint(screen: &Screen) -> &'static str {
    match input_mode(screen) {
        Mode::Browsing => match screen.page {
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
            crate::stashes::View::Detail { .. } => i18n::tr!(PgdnScrollEscBackCommands),
        },
        Mode::History(view) => match view.as_ref() {
            crate::history::View::Graph(_) => i18n::tr!(HistoryInputHint),
            crate::history::View::Commit(commit) => match commit.focus {
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
        Mode::Files(_) => i18n::tr!(SelectSpaceInsMarkCtrlAAll),
        Mode::Changes(session) => match session.focus {
            crate::changes::Focus::Files if screen.config.text_wrap => {
                i18n::tr!(WrappedChangesInputHint)
            }
            crate::changes::Focus::Files => i18n::tr!(ChangesInputHint),
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
    let (title, content, hint) = if screen.focus_mode == crate::focus::State::Selecting {
        (
            i18n::tr!(PaneFocusTitle),
            Content::Locked(screen.focused_pane_name().into()),
            i18n::tr!(PaneFocusHint),
        )
    } else if let crate::bar::InputBar::Layout { selected, .. } = bar {
        (
            i18n::tr!(LayoutPickerTitle),
            Content::Layout {
                selected: *selected,
            },
            i18n::tr!(LayoutPickerHint),
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
            Mode::Browsing
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
                        Some((argument, _))
                            if argument.action().available(screen.config.ai.enabled) =>
                        {
                            argument.hint()
                        }
                        _ => command_hint(bar.command_input().expect("command input")),
                    }
                } else if !bar.editor().text.is_empty()
                    && support.mode() == Some(crate::search::Mode::Locate)
                {
                    i18n::tr!(SearchLocateHint)
                } else if !bar.editor().text.is_empty() {
                    i18n::tr!(SearchEditingHint)
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
            Mode::CommitMessage { editor, .. } => (
                i18n::tr!(ConfirmOperationAlt),
                Content::CommitActions { draft: editor },
                browsing_hint(screen),
            ),
            Mode::Confirm { operation, choice } => (
                i18n::tr!(ConfirmOperationAlt),
                Content::Choices {
                    choice: *choice,
                    execute: match operation {
                        Confirmation::Schedule(operation) => match operation {
                            engine::scheduler::Control::Enable => i18n::tr!(EnableScheduledTask),
                            engine::scheduler::Control::Disable => i18n::tr!(DisableScheduledTask),
                            engine::scheduler::Control::Apply => i18n::tr!(ActionApply),
                        },
                        Confirmation::Mutation { .. } => i18n::tr!(MutationRun),
                        Confirmation::Remove(_) => i18n::tr!(ActionUnregister),
                        Confirmation::Configure { command, .. } => match command {
                            command::ConfigArgument::Add => i18n::tr!(ActionAdd),
                            command::ConfigArgument::Exclude => i18n::tr!(ActionExclude),
                            command::ConfigArgument::Unexclude => i18n::tr!(ActionUnexclude),
                            command::ConfigArgument::Scan => i18n::tr!(ActionScan),
                            command::ConfigArgument::Import => i18n::tr!(ActionImport),
                            _ => i18n::tr!(ActionSave),
                        },
                    },
                    back: if matches!(
                        operation,
                        Confirmation::Configure { .. } | Confirmation::Mutation { .. }
                    ) {
                        i18n::tr!(ActionReturnEdit)
                    } else {
                        i18n::tr!(ActionCancel)
                    },
                },
                i18n::tr!(ConfirmSelectHint),
            ),
            Mode::PushConfirm { choice, .. } => (
                i18n::tr!(ConfirmPushAlt),
                Content::Choices {
                    choice: *choice,
                    execute: i18n::tr!(ActionPush),
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
    let task_hint = task_hint(screen);
    let hint = task_hint.unwrap_or(hint);
    let show_cursor = task_hint.is_none();
    let title = if matches!(bar, crate::bar::InputBar::Search(_))
        && screen.focus_mode == crate::focus::State::Inactive
        && matches!(content, Content::Editable { .. })
        && !matches!(mode, Mode::CommitMessage { .. })
    {
        search_title
    } else {
        title.to_owned()
    };
    let active = !matches!(mode, Mode::CommitMessage { editor, .. } if editor.focused())
        && matches!(
            content,
            Content::CommandEntry
                | Content::Editable { .. }
                | Content::Choices { .. }
                | Content::CommitActions { .. }
                | Content::Layout { .. }
        );
    let input_style = Style::default().bg(Color::Reset).fg(Color::Reset);
    let accent = if active {
        active_accent(screen)
    } else if screen.config.interaction_mode == engine::state::InteractionMode::TextSelection {
        GOOD
    } else {
        MUTED
    };
    let mut block = editor_block(area)
        .title_style(Style::default().fg(accent))
        .border_style(Style::default().fg(accent))
        .style(input_style);
    if area.height >= 3 && !title.trim().is_empty() {
        block = block.title(format!(" {} ", title.trim()));
    }
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let mut surface = Surface::Plain;
    match content {
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
            let command = bar.command_input();
            let prefix = command
                .and_then(|input| input.context())
                .map(|context| {
                    let prefix = format!("/{}", context.name());
                    let budget = inner.width.saturating_sub(4) as usize;
                    if unicode_width::UnicodeWidthStr::width(prefix.as_str()) <= budget {
                        prefix
                    } else {
                        truncate(&prefix, budget)
                    }
                })
                .filter(|prefix| !prefix.is_empty());
            let prefix_width = prefix.as_deref().map_or(0, |prefix| {
                unicode_width::UnicodeWidthStr::width(prefix) as u16 + 1
            });
            if prefix.is_some() {
                surface = Surface::Prefix(Rect::new(
                    inner.x,
                    inner.y,
                    prefix_width.min(inner.width),
                    inner.height.min(1),
                ));
            }
            let width = inner.width.saturating_sub(2 + prefix_width) as usize;
            let (text, cursor) = editor.viewport(width);
            let value = if editor.text.is_empty() {
                Span::styled(
                    truncate(placeholder, width),
                    Style::default().fg(MUTED).add_modifier(Modifier::ITALIC),
                )
            } else {
                Span::styled(text, input_style)
            };
            let mut spans = Vec::new();
            if let Some(prefix) = prefix {
                spans.extend(prefix::spans(
                    &prefix,
                    command.and_then(|input| input.prefix_selection()),
                ));
                spans.push(Span::raw(" "));
            }
            spans.push(Span::styled("› ", Style::default().fg(INPUT_ACCENT)));
            spans.push(value);
            frame.render_widget(Paragraph::new(Line::from(spans)).style(input_style), inner);
            if !inner.is_empty()
                && show_cursor
                && !command.is_some_and(|input| input.prefix_selected())
            {
                frame.set_cursor_position((
                    (inner.x + prefix_width + 2 + cursor).min(inner.right() - 1),
                    inner.y,
                ));
            }
        }
        Content::Layout { selected } => {
            for (area, preference, label) in layout_picker::options(inner, screen.diff_preference) {
                frame.render_widget(
                    Paragraph::new(Span::styled(
                        label,
                        if selected == preference {
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
    render_hint(frame, hint_area, hint);
}
fn render_hint(frame: &mut Frame, area: Rect, hint: &str) {
    let hint_area = footer_text_area(area);
    frame.render_widget(
        Paragraph::new(truncate(hint, hint_area.width as usize)).style(Style::default().fg(MUTED)),
        hint_area,
    );
}
