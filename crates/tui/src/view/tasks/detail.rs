//! Semantic task details keep metadata quiet and give failures a distinct section.
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
pub(super) fn text(record: &Record, width: u16) -> Text<'static> {
    let (status, color) = label(record);
    let source = match record.spec.source {
        Source::Interactive => i18n::tr!(TasksInteractive),
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
    if !body.trim().is_empty() {
        lines.push(Line::default());
        lines.push(section(title, color, width));
        lines.push(Line::default());
        lines.extend(multiline(body).lines().map(|line| {
            Line::styled(
                line.to_owned(),
                Style::default().fg(if color == BAD {
                    Color::LightRed
                } else {
                    Color::Gray
                }),
            )
        }));
    }
    Text::from(lines)
}
pub(super) fn panel(area: Rect) -> Block<'static> {
    block(i18n::tr!(TasksDetails)).padding(ratatui::widgets::Padding::new(
        1,
        1,
        u16::from(area.height >= 8),
        0,
    ))
}
pub(super) fn preview(frame: &mut Frame, record: &Record, area: Rect) {
    let panel = panel(area);
    let inner = panel.inner(area);
    frame.render_widget(panel, area);
    frame.render_widget(
        Paragraph::new(text(record, inner.width)).wrap(Wrap { trim: false }),
        inner,
    );
}
