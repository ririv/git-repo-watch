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
        Mode::Browsing | Mode::Files(_) | Mode::Diff { .. } => {
            let command = matches!(screen.bar, crate::bar::InputBar::Command { .. });
            let files = screen.mode.file_session().is_some();
            let hint = if command {
                i18n::tr!(TabCompleteCandidatesEnterRunEscClear)
            } else {
                match screen.mode {
                    Mode::Files(_) => i18n::tr!(SelectSpaceInsMarkCtrlAAll),
                    Mode::Diff { .. } => i18n::tr!(ScrollEscFilesCommands),
                    _ => match screen.page {
                        Page::Repositories => i18n::tr!(SelectEnterDetailsCtrlFFilterTab),
                        Page::Settings => i18n::tr!(SelectEnterEditTabReposCommands),
                        _ => i18n::tr!(PgdnScrollEscBackCommands),
                    },
                }
            };
            (
                if command {
                    i18n::tr!(CommandsAlt)
                } else if files {
                    i18n::tr!(SearchFiles)
                } else {
                    i18n::tr!(SearchRepositories)
                },
                Content::Editable {
                    editor: screen.bar.editor(),
                    placeholder: i18n::tr!(TypeToSearchOrForCommands),
                },
                hint,
            )
        }
        Mode::Input { editor, .. } => (
            i18n::tr!(ConfigurationInputAlt),
            Content::Editable {
                editor,
                placeholder: i18n::tr!(EnterAConfigurationValue),
            },
            i18n::tr!(EnterNextEscCancel),
        ),
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
                    Confirmation::Apply => i18n::tr!(ActionApply),
                    Confirmation::Remove(_) => i18n::tr!(ActionUnregister),
                    Confirmation::Configure { action, .. } => match action {
                        Action::Add => i18n::tr!(ActionAdd),
                        Action::Exclude => i18n::tr!(ActionExclude),
                        Action::Unexclude => i18n::tr!(ActionUnexclude),
                        Action::Scan => i18n::tr!(ActionScan),
                        Action::Import => i18n::tr!(ActionImport),
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
                safe(&p.title)
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
