//! Bordered input with explicit editable and locked states.
use super::*;

const INPUT_ACCENT: Color = Color::Rgb(180, 145, 220);

enum Content<'a> {
    Editable {
        editor: &'a Editor,
        placeholder: &'static str,
    },
    Locked(String),
    Choices {
        choice: confirmation::Choice,
        execute: &'static str,
        back: &'static str,
    },
}

pub(super) fn render(frame: &mut Frame, screen: &Screen, area: Rect) {
    let (title, content, hint) = match &screen.mode {
        Mode::Browsing | Mode::Panel(_) | Mode::Files(_) | Mode::Diff { .. } | Mode::History(_) => {
            let command = matches!(screen.bar, crate::bar::InputBar::Command { .. });
            let files = screen.mode.file_session().is_some();
            let hint = if matches!(
                &screen.bar,
                crate::bar::InputBar::Command {
                    menu: command::path::Menu::Directories(_),
                    ..
                }
            ) {
                i18n::tr!(DirectoryCompletionHint)
            } else if command {
                match command::argument_input(
                    screen
                        .bar
                        .editor()
                        .text
                        .strip_prefix('/')
                        .expect("command prefix"),
                ) {
                    Some((argument, _)) => argument.hint(),
                    None => i18n::tr!(TabCompleteCandidatesEnterRunEscClear),
                }
            } else if !screen.bar.editor().text.is_empty() {
                i18n::tr!(SearchEditingHint)
            } else {
                match screen.mode {
                    Mode::History(ref view) => match view.as_ref() {
                        crate::history::View::Graph(_) => i18n::tr!(HistoryInputHint),
                        crate::history::View::Commit(_) => i18n::tr!(HistoryCommitInputHint),
                        crate::history::View::Diff { .. } => i18n::tr!(HistoryDiffHint),
                    },
                    Mode::Files(_) => i18n::tr!(SelectSpaceInsMarkCtrlAAll),
                    Mode::Diff { .. } => i18n::tr!(ScrollEscFilesCommands),
                    _ => match screen.page {
                        Page::Repositories => i18n::tr!(SelectEnterDetailsCtrlFFilterTab),
                        Page::Detail => i18n::tr!(OverviewInputHint),
                        Page::Configuration(Configuration::Schedule) => i18n::tr!(ScheduleHelpHint),
                        Page::Configuration(_) => i18n::tr!(SelectEnterEditTabReposCommands),
                        _ => i18n::tr!(PgdnScrollEscBackCommands),
                    },
                }
            };
            (
                if command {
                    i18n::tr!(CommandsAlt)
                } else if matches!(&screen.mode, Mode::History(view) if matches!(view.as_ref(), crate::history::View::Graph(_)))
                {
                    i18n::tr!(HistorySearch)
                } else if files || matches!(screen.mode, Mode::History(_)) {
                    i18n::tr!(SearchFiles)
                } else {
                    i18n::tr!(SearchRepositories)
                },
                Content::Editable {
                    editor: screen.bar.editor(),
                    placeholder: if matches!(&screen.mode, Mode::History(view) if matches!(view.as_ref(), crate::history::View::Graph(_)))
                    {
                        i18n::tr!(HistorySearchHint)
                    } else {
                        i18n::tr!(TypeToSearchOrForCommands)
                    },
                },
                hint,
            )
        }
        Mode::CommitMessage { editor, .. } => (
            i18n::tr!(CommitMessageAlt),
            Content::Editable {
                editor,
                placeholder: i18n::tr!(DescribeThisCommit),
            },
            i18n::tr!(EnterConfirmEscCancelPgupPgdnReview),
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
                    Confirmation::Remove(_) => i18n::tr!(ActionUnregister),
                    Confirmation::Configure { command, .. } => match command {
                        command::ArgumentAction::Add => i18n::tr!(ActionAdd),
                        command::ArgumentAction::Exclude => i18n::tr!(ActionExclude),
                        command::ArgumentAction::Unexclude => i18n::tr!(ActionUnexclude),
                        command::ArgumentAction::Scan => i18n::tr!(ActionScan),
                        command::ArgumentAction::Import => i18n::tr!(ActionImport),
                        _ => i18n::tr!(ActionSave),
                    },
                },
                back: if matches!(operation, Confirmation::Configure { .. }) {
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
        Mode::Error { .. } => (
            i18n::tr!(OperationIncomplete),
            Content::Locked(i18n::tr!(SeeErrorDetails).into()),
            i18n::tr!(PgdnFullErrorEnterEscBack),
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
    };
    let parts = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Length(1),
    ])
    .split(area);
    frame.render_widget(
        Paragraph::new(truncate(&safe(&screen.message), area.width as usize))
            .style(Style::default().fg(WARN)),
        parts[0],
    );
    let active = matches!(content, Content::Editable { .. } | Content::Choices { .. });
    let input_style = Style::default().bg(Color::Reset).fg(Color::Reset);
    let block = Block::bordered()
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title(title)
        .title_style(Style::default().fg(if active { INPUT_ACCENT } else { MUTED }))
        .border_style(Style::default().fg(if active { INPUT_ACCENT } else { MUTED }))
        .style(input_style);
    let inner = block.inner(parts[1]);
    frame.render_widget(block, parts[1]);
    match content {
        Content::Editable {
            editor,
            placeholder,
        } => {
            let width = inner.width.saturating_sub(2) as usize;
            let (text, cursor) = editor.viewport(width);
            let value = if editor.text.is_empty() {
                Span::styled(
                    truncate(placeholder, width),
                    Style::default().fg(MUTED).add_modifier(Modifier::ITALIC),
                )
            } else {
                Span::styled(text, input_style)
            };
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("› ", Style::default().fg(INPUT_ACCENT)),
                    value,
                ]))
                .style(input_style),
                inner,
            );
            frame.set_cursor_position((inner.x + 2 + cursor, inner.y));
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
    frame.render_widget(
        Paragraph::new(truncate(hint, area.width as usize)).style(Style::default().fg(MUTED)),
        parts[2],
    );
}
