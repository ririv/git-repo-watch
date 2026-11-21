//! Semantic task details keep metadata quiet and give failures a distinct section.
mod batch;
use super::*;
use unicode_width::UnicodeWidthStr;
fn field(label: &str, value: String, label_width: usize) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!(
                "{label}{}  ",
                " ".repeat(label_width.saturating_sub(label.width()))
            ),
            Style::default().fg(MUTED),
        ),
        Span::styled(value, Style::default().fg(Color::Gray)),
    ])
}
fn section(title: &str, color: Color, width: u16) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!(" {title} "), Style::default().fg(color).bold()),
        Span::styled(
            "─".repeat(usize::from(width).saturating_sub(title.width() + 2)),
            Style::default().fg(BORDER),
        ),
    ])
}
fn text(record: &Record, width: u16) -> (Text<'static>, Option<usize>) {
    let (_, status, color) = status(record);
    let source = match record.spec.source {
        Source::Interactive => i18n::tr!(TasksInteractive),
        Source::Automatic => i18n::tr!(AutoFetch),
        Source::Startup => i18n::tr!(WorkflowStartup),
        Source::Scheduled => i18n::tr!(TasksScheduled),
        Source::Imported => i18n::tr!(TasksImported),
    };
    let (scope_label, path) = match &record.spec.scope {
        TaskScope::Repository { path } => (i18n::tr!(Path), safe(&path.display().to_string())),
        _ => (i18n::tr!(TasksScope), scope(record)),
    };
    let mut fields = vec![
        (i18n::tr!(TasksSource), source.to_owned()),
        (scope_label, path),
        (i18n::tr!(TasksStarted), local_time(&record.started_at)),
    ];
    if let State::Finished { at, .. } = &record.state {
        fields.push((i18n::tr!(TasksFinished), local_time(at)));
    }
    let label_width = fields
        .iter()
        .map(|(label, _)| label.width())
        .max()
        .unwrap_or(0);
    let mut lines = vec![
        Line::styled(safe(&record.spec.title), Style::default().fg(ACCENT).bold()),
        Line::from(vec![Span::styled(
            format!(" {status} "),
            Style::default().fg(color).bg(Color::Rgb(36, 43, 52)).bold(),
        )]),
        Line::default(),
    ];
    lines.extend(
        fields
            .into_iter()
            .map(|(label, value)| field(label, value, label_width)),
    );
    let (title, color, body) = match &record.state {
        State::Running { progress } | State::Cancelling { progress } => {
            (i18n::tr!(TasksProgress), ACCENT, progress.as_str())
        }
        State::Finished {
            outcome: Completion::Failed { detail, .. },
            ..
        } => (i18n::tr!(TasksFailureReason), BAD, detail.as_str()),
        State::Finished {
            outcome: Completion::Succeeded { detail },
            ..
        } => (i18n::tr!(OperationResult), GOOD, detail.as_str()),
        State::Finished {
            outcome: Completion::Cancelled { detail },
            ..
        } => (i18n::tr!(OperationResult), WARN, detail.as_str()),
    };
    let mut failure_line = record.unread_failure().then_some(1);
    if !body.trim().is_empty() {
        let legacy_push = match &record.state {
            State::Finished {
                outcome: Completion::Succeeded { .. },
                ..
            } => crate::task_center::push_detail::from_legacy(body),
            _ => None,
        };
        let body = multiline(legacy_push.as_deref().unwrap_or(body));
        lines.push(Line::default());
        if color == BAD
            && let Some(first_reason) = batch::append_lines(record, &body, width, &mut lines)
        {
            failure_line = failure_line.map(|_| first_reason);
        } else {
            lines.push(section(title, color, width));
            lines.push(Line::default());
            if failure_line.is_some() {
                failure_line = Some(lines.len());
            }
            let body_color = if color == BAD { BAD } else { Color::Gray };
            lines.extend(
                body.lines()
                    .map(|line| Line::styled(line.to_owned(), Style::default().fg(body_color))),
            );
        }
    }
    if !record.stage_timings.is_empty() {
        lines.push(Line::default());
        lines.push(section(i18n::tr!(ProgressTimings), ACCENT, width));
        let label_width = record
            .stage_timings
            .iter()
            .map(|timing| timing.stage.label().width())
            .max()
            .unwrap_or(0);
        lines.extend(record.stage_timings.iter().map(|timing| {
            field(
                timing.stage.label(),
                format!("{:.3} s", timing.elapsed_ms as f64 / 1000.0),
                label_width,
            )
        }));
    }
    (Text::from(lines), failure_line)
}
pub(super) fn panel(area: Rect) -> Block<'static> {
    block(i18n::tr!(TasksDetails)).padding(ratatui::widgets::Padding::new(
        1,
        1,
        u16::from(area.height >= 8),
        0,
    ))
}
pub(super) struct Rendered {
    pub area: Rect,
    pub maximum: usize,
    pub shows_unread_failure: bool,
}

pub(super) fn render(
    frame: &mut Frame,
    record: &Record,
    area: Rect,
    scroll: Option<&mut usize>,
) -> Rendered {
    let panel = panel(area);
    let inner = panel.inner(area);
    let (text, failure_line) = text(record, inner.width);
    let height = usize::from(inner.height);
    let failure_start = failure_line.filter(|_| !inner.is_empty()).map(|line| {
        Paragraph::new(Text::from(text.lines[..line].to_vec()))
            .wrap(Wrap { trim: false })
            .line_count(inner.width)
    });
    let paragraph = Paragraph::new(text).wrap(Wrap { trim: false });
    let full = scroll.map(|scroll| {
        let lines = paragraph.line_count(inner.width);
        *scroll = (*scroll).min(lines.saturating_sub(height));
        (lines, (*scroll).min(u16::MAX.into()))
    });
    let offset = full.map_or(0, |(_, offset)| offset);
    frame.render_widget(panel, area);
    frame.render_widget(paragraph.scroll((offset as u16, 0)), inner);
    if let Some((lines, offset)) = full {
        crate::scrollbar::vertical(frame, area, inner, lines, offset);
    }
    Rendered {
        area: inner,
        maximum: full.map_or(0, |(lines, _)| lines.saturating_sub(height)),
        shows_unread_failure: failure_start
            .is_some_and(|start| start < offset.saturating_add(height)),
    }
}
