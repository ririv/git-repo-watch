//! Bordered input with explicit editable and locked states.
use super::feedback::Preview;
use super::*;

const INPUT_ACCENT: Color = Color::Rgb(180, 145, 220);

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
        let budget = if area.height >= 9 {
            (area.height / 3).min(4)
        } else {
            0
        };
        let preview = Preview::fit(&status_text(screen, area), area.width, budget);
        match &screen.mode {
            Mode::PromptEditor(_) => Self::Compact {
                hint: i18n::tr!(AgentPromptEditorHint),
                preview,
            },
            Mode::Busy(pending) if matches!(pending.origin.as_ref(), Mode::PromptEditor(_)) => {
                Self::Compact {
                    hint: i18n::tr!(EscCtrlCCancelAndWaitFor),
                    preview,
                }
            }
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
pub(crate) fn status_text(screen: &Screen, body: Rect) -> String {
    if matches!(screen.bar, crate::bar::InputBar::Layout { .. }) {
        return format!(
            "{}: {}",
            i18n::tr!(LayoutCurrent),
            layout_picker::summary(screen, body)
        );
    }
    match &screen.mode {
        Mode::Busy(pending) if pending.state == TaskState::Cancelling => {
            i18n::tr!(Cancelling).into()
        }
        Mode::Busy(pending) => format!("{} {}", pending.animation.symbol(), safe(&pending.title)),
        _ => screen
            .commit_generator
            .label()
            .filter(|_| matches!(screen.mode, Mode::CommitMessage { .. }))
            .unwrap_or_else(|| screen.message.clone()),
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
    Editable {
        editor: &'a Editor,
        placeholder: &'static str,
    },
    Locked(String),
    Layout {
        selected: crate::diff::Preference,
    },
    Choices {
        choice: confirmation::Choice,
        execute: &'static str,
        back: &'static str,
    },
}

pub(super) fn render(frame: &mut Frame, screen: &Screen, area: Rect, hint_area: Rect) {
    let (title, content, hint) = if screen.focus_mode == crate::focus::State::Selecting {
        (
            i18n::tr!(PaneFocusTitle),
            Content::Locked(screen.focused_pane_name().into()),
            i18n::tr!(PaneFocusHint),
        )
    } else if let crate::bar::InputBar::Layout { selected, .. } = &screen.bar {
        (
            i18n::tr!(LayoutPickerTitle),
            Content::Layout {
                selected: *selected,
            },
            i18n::tr!(LayoutPickerHint),
        )
    } else {
        match &screen.mode {
            Mode::Browsing
            | Mode::Browser(_)
            | Mode::Panel(_)
            | Mode::Stashes(_)
            | Mode::Branches(_)
            | Mode::Files(_)
            | Mode::Changes(_)
            | Mode::History(_) => {
                let command = matches!(screen.bar, crate::bar::InputBar::Command { .. });
                let files = screen.mode.file_session().is_some();
                let hint = if matches!(
                    &screen.bar,
                    crate::bar::InputBar::Command {
                        menu: command::completion::Menu::Directories(_)
                            | command::completion::Menu::Values(_),
                        ..
                    }
                ) {
                    i18n::tr!(DirectoryCompletionHint)
                } else if command {
                    match command::argument_input(
                        screen.bar.text().strip_prefix('/').expect("command prefix"),
                    ) {
                        Some((argument, _))
                            if argument.action().available(screen.config.ai.enabled) =>
                        {
                            argument.hint()
                        }
                        _ => match screen.bar.command_input() {
                            Some(input) if input.prefix_selected() => i18n::tr!(ContextPrefixHint),
                            Some(input) if input.context().is_some() => {
                                i18n::tr!(ContextCommandHint)
                            }
                            _ => i18n::tr!(TabCompleteCandidatesEnterRunEscClear),
                        },
                    }
                } else if matches!(&screen.mode, Mode::Changes(session) if session.focus == crate::changes::Focus::Preview)
                {
                    i18n::tr!(ChangesInputHint)
                } else if !screen.bar.editor().text.is_empty()
                    && screen.search_support().mode() == Some(crate::search::Mode::Locate)
                {
                    i18n::tr!(SearchLocateHint)
                } else if !screen.bar.editor().text.is_empty() {
                    i18n::tr!(SearchEditingHint)
                } else {
                    match screen.mode {
                        Mode::Browser(_) => i18n::tr!(BrowserHint),
                        Mode::Stashes(_) => i18n::tr!(StashHint),
                        Mode::Branches(ref view) => match view.as_ref() {
                            crate::branches::View::List(_) => i18n::tr!(BranchHint),
                            crate::branches::View::Detail { .. } => i18n::tr!(OverviewInputHint),
                        },
                        Mode::History(ref view) => match view.as_ref() {
                            crate::history::View::Graph(_) => i18n::tr!(HistoryInputHint),
                            crate::history::View::Commit(_) => i18n::tr!(HistoryCommitInputHint),
                            crate::history::View::Diff { content, .. }
                                if matches!(content.as_ref(), crate::document::View::Images(_)) =>
                            {
                                i18n::tr!(ImageInputHint)
                            }
                            crate::history::View::Diff { .. } => i18n::tr!(HistoryDiffHint),
                        },
                        Mode::Files(_) => i18n::tr!(SelectSpaceInsMarkCtrlAAll),
                        Mode::Changes(_) => i18n::tr!(ChangesInputHint),
                        _ => match screen.page {
                            Page::Repositories => i18n::tr!(RepositoryListHint),
                            Page::Detail => i18n::tr!(OverviewInputHint),
                            Page::Configuration(Configuration::Schedule) => {
                                i18n::tr!(ScheduleHelpHint)
                            }
                            Page::Configuration(_) => i18n::tr!(SelectEnterEditBackCommands),
                            _ => i18n::tr!(PgdnScrollEscBackCommands),
                        },
                    }
                };
                (
                    if command {
                        i18n::tr!(CommandsAlt)
                    } else if matches!(screen.mode, Mode::Browser(_)) {
                        i18n::tr!(BrowserSearch)
                    } else if matches!(&screen.mode, Mode::History(view) if matches!(view.as_ref(), crate::history::View::Graph(_)))
                    {
                        i18n::tr!(HistorySearch)
                    } else if matches!(screen.mode, Mode::Stashes(_)) {
                        i18n::tr!(StashSearch)
                    } else if matches!(screen.mode, Mode::Branches(_)) {
                        i18n::tr!(BranchSearch)
                    } else if files || matches!(screen.mode, Mode::History(_)) {
                        i18n::tr!(SearchFiles)
                    } else {
                        i18n::tr!(SearchRepositories)
                    },
                    Content::Editable {
                        editor: screen.bar.editor(),
                        placeholder: if screen
                            .bar
                            .command_input()
                            .is_some_and(|input| input.menu_context().is_some())
                        {
                            i18n::tr!(ContextCommandPlaceholder)
                        } else if screen.search_support() == crate::search::Support::Unavailable {
                            i18n::tr!(SearchCommandOnly)
                        } else if matches!(screen.mode, Mode::Browser(_)) {
                            i18n::tr!(BrowserPlaceholder)
                        } else if matches!(&screen.mode, Mode::History(view) if matches!(view.as_ref(), crate::history::View::Graph(_)))
                        {
                            i18n::tr!(HistorySearchHint)
                        } else {
                            i18n::tr!(TypeToSearchOrForCommands)
                        },
                    },
                    hint,
                )
            }
            Mode::PromptEditor(_) => unreachable!("the document editor has a compact footer"),
            Mode::CommitMessage { editor, .. } => (
                i18n::tr!(CommitMessageAlt),
                Content::Editable {
                    editor,
                    placeholder: i18n::tr!(DescribeThisCommit),
                },
                if !screen.config.ai.enabled {
                    i18n::tr!(EnterConfirmEscCancelPgupPgdnReview)
                } else if screen.commit_generator.running() {
                    i18n::tr!(AgentRunningHint)
                } else {
                    i18n::tr!(AgentHint)
                },
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
            Mode::CommitConfirm { choice, .. } => (
                i18n::tr!(ConfirmCommitAlt),
                Content::Choices {
                    choice: *choice,
                    execute: i18n::tr!(ActionCommit),
                    back: i18n::tr!(ActionReturnEdit),
                },
                i18n::tr!(ConfirmReviewHint),
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
            Mode::Error { .. } if screen.bar.command_input().is_some() => (
                i18n::tr!(CommandsAlt),
                Content::Editable {
                    editor: screen.bar.editor(),
                    placeholder: i18n::tr!(ContextCommandPlaceholder),
                },
                if screen
                    .bar
                    .command_input()
                    .is_some_and(|input| input.prefix_selected())
                {
                    i18n::tr!(ContextPrefixHint)
                } else {
                    i18n::tr!(ContextCommandHint)
                },
            ),
            Mode::Error { .. } => (
                i18n::tr!(OperationIncomplete),
                Content::Locked(i18n::tr!(SeeErrorDetails).into()),
                i18n::tr!(CopyErrorHint),
            ),
            Mode::Busy(p) => (
                i18n::tr!(RunningAlt),
                Content::Locked(if p.state == TaskState::Cancelling {
                    i18n::tr!(Cancelling).into()
                } else {
                    format!("{} {}", p.animation.symbol(), safe(&p.title))
                }),
                i18n::tr!(EscCtrlCCancelAndWaitFor),
            ),
        }
    };
    let title = if matches!(screen.bar, crate::bar::InputBar::Search(_))
        && screen.focus_mode == crate::focus::State::Inactive
        && matches!(content, Content::Editable { .. })
        && !matches!(screen.mode, Mode::CommitMessage { .. })
    {
        screen.search_title()
    } else {
        title.to_owned()
    };
    let active = matches!(
        content,
        Content::Editable { .. } | Content::Choices { .. } | Content::Layout { .. }
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
    if area.height >= 3 {
        block = block.title(format!(" {} ", title.trim()));
    }
    let inner = block.inner(area);
    frame.render_widget(block, area);
    match content {
        Content::Editable {
            editor,
            placeholder,
        } => {
            let command = screen.bar.command_input();
            let prefix = command
                .and_then(|input| input.context())
                .map(|context| {
                    let prefix = format!(" /{} ", context.name());
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
                spans.push(Span::styled(
                    prefix,
                    if command.is_some_and(|input| input.prefix_selected()) {
                        Style::default().fg(Color::Black).bg(INPUT_ACCENT).bold()
                    } else {
                        Style::default()
                            .fg(Color::LightCyan)
                            .bg(Color::Rgb(34, 52, 65))
                            .bold()
                    },
                ));
                spans.push(Span::raw(" "));
            }
            spans.push(Span::styled("› ", Style::default().fg(INPUT_ACCENT)));
            spans.push(value);
            frame.render_widget(Paragraph::new(Line::from(spans)).style(input_style), inner);
            if !inner.is_empty() && !command.is_some_and(|input| input.prefix_selected()) {
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
        Content::Choices {
            choice,
            execute,
            back,
        } => {
            let button = |label: &str, focused: bool| {
                Span::styled(
                    format!(" {label} "),
                    if focused {
                        Style::default()
                            .fg(Color::Black)
                            .bg(INPUT_ACCENT)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Gray)
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
    render_hint(frame, hint_area, hint);
}
fn render_hint(frame: &mut Frame, area: Rect, hint: &str) {
    let hint_area = footer_text_area(area);
    frame.render_widget(
        Paragraph::new(truncate(hint, hint_area.width as usize)).style(Style::default().fg(MUTED)),
        hint_area,
    );
}
