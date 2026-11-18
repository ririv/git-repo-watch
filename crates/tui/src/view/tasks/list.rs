//! Task entries reserve one preview line for each pending failure.
use super::*;
use unicode_width::UnicodeWidthStr;

fn heading(record: &Record, width: usize, animation: &'static str) -> Vec<Line<'static>> {
    let (icon, _, color) = status(record);
    let symbol = match icon {
        Icon::Spinner => animation,
        Icon::Symbol(symbol) => symbol,
    };
    let status = Span::styled(symbol, Style::default().fg(color).bold());
    let content_width = width.saturating_sub(symbol.width() + 1);
    let progress = match &record.state {
        State::Running { progress } | State::Cancelling { progress } if !progress.is_empty() => {
            format!(" · {progress}")
        }
        _ => String::new(),
    };
    let title = format!("{} · {}{progress}", scope(record), safe(&record.spec.title));
    let time = local_time(&record.started_at);
    if width < 42 {
        return vec![
            Line::from(vec![
                status,
                Span::raw(" "),
                Span::raw(truncate(&title, content_width)),
            ]),
            Line::styled(truncate(&time, width), Style::default().fg(MUTED)).right_aligned(),
        ];
    }
    let title_width = content_width.saturating_sub(time.width() + 1);
    let title = truncate(&title, title_width);
    let padding = " ".repeat(title_width.saturating_sub(title.width()) + 1);
    vec![Line::from(vec![
        status,
        Span::raw(" "),
        Span::raw(title),
        Span::raw(padding),
        Span::styled(time, Style::default().fg(MUTED)),
    ])]
}
fn reason(record: &Record, width: usize) -> Vec<Line<'static>> {
    let State::Finished {
        outcome:
            Completion::Failed {
                detail,
                resolution: Resolution::Pending,
            },
        ..
    } = &record.state
    else {
        return vec![];
    };
    if width <= 2 {
        return vec![];
    }
    let text = multiline(detail);
    let mut lines = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .flat_map(|line| crate::links::Line::from(Line::raw(line.to_owned())).wrap(width - 2, true))
        .take(2)
        .map(|line| line.text)
        .collect::<Vec<_>>();
    if lines.len() > 1 {
        lines.truncate(1);
        let last = lines.last_mut().expect("one preview line");
        let shortened = truncate(&last.to_string(), width - 3);
        *last = Line::raw(if shortened.ends_with('…') {
            shortened
        } else {
            format!("{shortened}…")
        });
    }
    lines
        .into_iter()
        .map(|line| {
            Line::from(vec![
                Span::styled("╰─", Style::default().fg(MUTED)),
                Span::styled(line.to_string(), Style::default().fg(BAD)),
            ])
        })
        .collect()
}
pub(super) fn render(frame: &mut Frame, screen: &mut Screen, area: Rect) {
    let panel = block(i18n::tr!(Tasks)).padding(ratatui::widgets::Padding::horizontal(1));
    let inner = panel.inner(area);
    screen.tasks.content = inner;
    let index = screen.tasks.selection.as_ref().and_then(|id| {
        screen
            .tasks
            .records
            .iter()
            .position(|record| record.id == *id)
    });
    screen.tasks.viewport.select(index);
    if screen.tasks.records.is_empty() {
        frame.render_widget(panel, area);
        frame.render_widget(
            Paragraph::new(i18n::tr!(TasksEmpty)).style(Style::default().fg(MUTED)),
            inner,
        );
        return;
    }
    if inner.is_empty() {
        frame.render_widget(panel, area);
        return;
    }
    let entries = screen
        .tasks
        .records
        .iter()
        .map(|record| {
            let mut lines = heading(record, inner.width.into(), screen.tasks.animation.symbol());
            lines.extend(reason(record, inner.width.into()));
            lines.truncate(inner.height.into());
            lines
        })
        .collect::<Vec<_>>();
    let heights = entries.iter().map(Vec::len).collect::<Vec<_>>();
    let widget = List::new(
        entries
            .into_iter()
            .map(|lines| ListItem::new(Text::from(lines))),
    )
    .block(panel)
    .highlight_style(selected_style());
    frame.render_stateful_widget(widget, area, &mut screen.tasks.viewport);
    let mut row = 0usize;
    let offset = screen.tasks.viewport.offset();
    for (index, record) in screen.tasks.records.iter().enumerate().skip(offset) {
        let height = heights[index];
        if row + height > usize::from(inner.height) {
            break;
        }
        screen.tasks.rows.push((
            Rect::new(inner.x, inner.y + row as u16, inner.width, height as u16),
            record.id.clone(),
        ));
        row += height;
    }
    let total = heights.iter().sum();
    let position = heights.iter().take(offset).sum();
    crate::scrollbar::vertical(frame, area, inner, total, position);
}
