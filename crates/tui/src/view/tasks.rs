//! Task rendering uses journal records; viewing never acknowledges a failure.
mod detail;
mod list;
pub(in crate::view) mod summary;
use super::*;
use crate::task_center::Session;
use engine::tasks::{Outcome as Completion, Record, Scope as TaskScope, Source, State};
enum Icon {
    Spinner,
    Symbol(&'static str),
}

fn status(record: &Record) -> (Icon, &'static str, Color) {
    use Icon::{Spinner, Symbol};

    match &record.state {
        State::Running { .. } => (Spinner, i18n::tr!(TasksRunning), ACCENT),
        State::Cancelling { .. } => (Symbol("⊘"), i18n::tr!(TasksCancelling), WARN),
        State::Finished {
            outcome: Completion::Succeeded { .. },
            ..
        } => (Symbol("✓"), i18n::tr!(TasksSuccess), GOOD),
        State::Finished {
            outcome: Completion::Cancelled { .. },
            ..
        } => (Symbol("−"), i18n::tr!(TasksCancelled), MUTED),
        State::Finished {
            outcome: Completion::Failed { .. },
            ..
        } => (Symbol("×"), i18n::tr!(TasksFailed), BAD),
    }
}
fn scope(record: &Record) -> String {
    match &record.spec.scope {
        TaskScope::All => i18n::tr!(TasksAll).into(),
        TaskScope::Application => i18n::tr!(TasksApplication).into(),
        TaskScope::Repository { path } => name(path),
    }
}
pub(super) fn render(frame: &mut Frame, screen: &mut Screen, area: Rect) {
    frame.render_widget(Clear, area);
    screen.tasks.rows.clear();
    let parts = Layout::vertical([
        Constraint::Length(u16::from(area.height > 2)),
        Constraint::Min(0),
        Constraint::Length(u16::from(area.height > 1)),
        Constraint::Length(u16::from(area.height > 3)),
    ])
    .split(area);
    let failures = screen.tasks.unread_failures;
    let running = screen.tasks.records.iter().filter(|r| r.active()).count();
    let mut heading = Line::from(Span::styled(
        format!(
            " {}{}",
            i18n::tr!(Tasks),
            match &screen.tasks.session {
                Session::Detail { id, .. } => screen
                    .tasks
                    .records
                    .iter()
                    .find(|record| record.id == *id)
                    .map(|record| format!(" › {}", record.spec.title))
                    .unwrap_or_default(),
                _ => String::new(),
            }
        ),
        Style::default().fg(ACCENT),
    ));
    let counts = summary::counts(failures, running);
    if !counts.spans.is_empty() {
        heading
            .spans
            .push(Span::styled(" · ", Style::default().fg(MUTED)));
        heading.spans.extend(counts.spans);
    }
    frame.render_widget(
        Paragraph::new(crate::horizontal::clip(&heading, 0, parts[0].width.into())),
        parts[0],
    );
    let body = parts[1];
    let hint = if matches!(screen.tasks.session, Session::Detail { .. }) {
        i18n::tr!(TasksDetailsHint)
    } else {
        i18n::tr!(TasksHint)
    };
    frame.render_widget(
        Paragraph::new(truncate(hint, parts[2].width.into())).style(Style::default().fg(MUTED)),
        footer_text_area(parts[2]),
    );
    let notice = screen
        .exit_sequence
        .notice()
        .or(screen.tasks.error.as_deref())
        .unwrap_or(&screen.message);
    let notice_color = if screen.exit_sequence.notice().is_some() {
        WARN
    } else if screen.tasks.error.is_some() || screen.message.is_error() {
        BAD
    } else {
        MUTED
    };
    frame.render_widget(
        Paragraph::new(truncate(
            notice.lines().next().unwrap_or(""),
            parts[3].width.into(),
        ))
        .style(Style::default().fg(notice_color)),
        footer_text_area(parts[3]),
    );
    match &mut screen.tasks.session {
        Session::Closed => {}
        Session::List => {
            let panels = if body.width >= 100 {
                Layout::horizontal([Constraint::Percentage(58), Constraint::Percentage(42)])
                    .split(body)
                    .to_vec()
            } else {
                vec![body]
            };
            list::render(frame, screen, panels[0]);
            if panels.len() > 1
                && let Some(record) = screen.tasks.selected()
            {
                detail::preview(frame, record, panels[1]);
            }
        }
        Session::Detail { id, scroll } => {
            let Some(record) = screen.tasks.records.iter().find(|record| record.id == *id) else {
                return;
            };
            let panel = detail::panel(body);
            let inner = panel.inner(body);
            let paragraph =
                Paragraph::new(detail::text(record, inner.width)).wrap(Wrap { trim: false });
            let lines = paragraph.line_count(inner.width);
            screen.tasks.maximum = lines.saturating_sub(inner.height.into());
            *scroll = (*scroll).min(screen.tasks.maximum);
            screen.tasks.content = inner;
            frame.render_widget(panel, body);
            frame.render_widget(
                paragraph.scroll(((*scroll).min(u16::MAX.into()) as u16, 0)),
                inner,
            );
            crate::scrollbar::vertical(frame, body, inner, lines, *scroll);
        }
    }
}
