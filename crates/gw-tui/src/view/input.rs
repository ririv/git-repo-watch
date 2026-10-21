//! Bordered input with explicit editable and locked states.
use super::*;

const INPUT_ACCENT: Color = Color::Rgb(180, 145, 220);

enum Content<'a> {
    Editable {
        editor: &'a Editor,
        placeholder: &'static str,
    },
    Locked(String),
}

pub(super) fn render(frame: &mut Frame, screen: &Screen, area: Rect) {
    let (title, content, hint) = match &screen.mode {
        Mode::Browsing | Mode::Files(_) | Mode::Diff { .. } => {
            let command = matches!(screen.bar, crate::bar::InputBar::Command { .. });
            let files = screen.mode.file_session().is_some();
            let hint = if command {
                gw_i18n::tr!(TabCompleteCandidatesEnterRunEscClear)
            } else {
                match screen.mode {
                    Mode::Files(_) => gw_i18n::tr!(SelectSpaceInsMarkCtrlAAll),
                    Mode::Diff { .. } => gw_i18n::tr!(ScrollEscFilesCommands),
                    _ => match screen.page {
                        Page::Repositories => gw_i18n::tr!(SelectEnterDetailsCtrlFFilterTab),
                        Page::Settings => gw_i18n::tr!(SelectEnterEditTabReposCommands),
                        _ => gw_i18n::tr!(PgdnScrollEscBackCommands),
                    },
                }
            };
            (
                if command {
                    gw_i18n::tr!(CommandsAlt)
                } else if files {
                    gw_i18n::tr!(SearchFiles)
                } else {
                    gw_i18n::tr!(SearchRepositories)
                },
                Content::Editable {
                    editor: screen.bar.editor(),
                    placeholder: gw_i18n::tr!(TypeToSearchOrForCommands),
                },
                hint,
            )
        }
        Mode::Input { editor, .. } => (
            gw_i18n::tr!(ConfigurationInputAlt),
            Content::Editable {
                editor,
                placeholder: gw_i18n::tr!(EnterAConfigurationValue),
            },
            gw_i18n::tr!(EnterNextEscCancel),
        ),
        Mode::CommitMessage { editor, .. } => (
            gw_i18n::tr!(CommitMessageAlt),
            Content::Editable {
                editor,
                placeholder: gw_i18n::tr!(DescribeThisCommit),
            },
            gw_i18n::tr!(EnterConfirmEscCancelPgupPgdnReview),
        ),
        Mode::Confirm(_) => (
            gw_i18n::tr!(ConfirmOperationAlt),
            Content::Locked(gw_i18n::tr!(AwaitingConfirmation).into()),
            gw_i18n::tr!(YConfirmNEscCancel),
        ),
        Mode::CommitConfirm { .. } => (
            gw_i18n::tr!(ConfirmCommitAlt),
            Content::Locked(gw_i18n::tr!(AwaitingCommitConfirmation).into()),
            gw_i18n::tr!(YCommitNEscCancelPgupPgdn),
        ),
        Mode::PushConfirm { .. } => (
            gw_i18n::tr!(ConfirmPushAlt),
            Content::Locked(gw_i18n::tr!(AwaitingPushConfirmation).into()),
            gw_i18n::tr!(YPushNEscCancelPgupPgdn),
        ),
        Mode::Error { .. } => (
            gw_i18n::tr!(OperationIncomplete),
            Content::Locked(gw_i18n::tr!(SeeErrorDetails).into()),
            gw_i18n::tr!(PgdnFullErrorEnterEscBack),
        ),
        Mode::Busy(p) => (
            gw_i18n::tr!(RunningAlt),
            Content::Locked(if p.state == TaskState::Cancelling {
                gw_i18n::tr!(Cancelling).into()
            } else {
                safe(&p.title)
            }),
            gw_i18n::tr!(EscCtrlCCancelAndWaitFor),
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
    let editable = matches!(content, Content::Editable { .. });
    let input_style = Style::default().bg(Color::Reset).fg(Color::Reset);
    let block = Block::bordered()
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title(title)
        .title_style(Style::default().fg(if editable { INPUT_ACCENT } else { MUTED }))
        .border_style(Style::default().fg(if editable { INPUT_ACCENT } else { MUTED }))
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
