//! Task rendering reports which failure was shown in the current frame.
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

    let (icon, color) = match &record.state {
        State::Running { .. } => (Spinner, ACCENT),
        State::Cancelling { .. } => (Symbol("⊘"), WARN),
        State::Finished { outcome, .. } => match outcome {
            Completion::Succeeded { .. } => (Symbol("✓"), GOOD),
            Completion::Cancelled { .. } => (Symbol("−"), MUTED),
            Completion::Failed { .. } => (Symbol("×"), BAD),
        },
    };
    (icon, crate::task_center::status_label(record), color)
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
    let input_height = match area.height {
        9.. => 3,
        5.. => 1,
        _ => 0,
    };
    let parts = Layout::vertical([
        Constraint::Length(u16::from(area.height > 2)),
        Constraint::Min(0),
        Constraint::Length(input_height),
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
    if !parts[2].is_empty() {
        screen.input_area = Some(parts[2]);
    }
    input::render_task_input(frame, screen, parts[2], parts[3], hint);
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
            parts[4].width.into(),
        ))
        .style(Style::default().fg(notice_color)),
        footer_text_area(parts[4]),
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
                && detail::render(frame, record, panels[1], None).shows_unread_failure
            {
                screen.tasks.visible_unread_failure = Some(record.id.clone());
            }
        }
        Session::Detail { id, scroll } => {
            let Some(record) = screen.tasks.records.iter().find(|record| record.id == *id) else {
                return;
            };
            let rendered = detail::render(frame, record, body, Some(scroll));
            screen.tasks.maximum = rendered.maximum;
            screen.tasks.content = rendered.area;
            if rendered.shows_unread_failure {
                screen.tasks.visible_unread_failure = Some(record.id.clone());
            }
        }
    }
    commands::render(frame, screen, body);
}
