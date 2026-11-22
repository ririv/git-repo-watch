//! Task entries reserve one preview line for failures.
use super::*;
use unicode_width::UnicodeWidthStr;

fn heading(record: &Record, width: usize, animation: &'static str) -> Vec<Line<'static>> {
    let (icon, _, color) = status(record);
    let symbol = match icon {
        Icon::Spinner => animation,
        Icon::Symbol(symbol) => symbol,
    };
    let mut line = Line::from(vec![
        Span::styled(symbol, Style::default().fg(color).bold()),
        Span::raw(" "),
    ]);
    let content_width = width.saturating_sub(line.width());
    let progress = match &record.state {
        State::Running { progress } | State::Cancelling { progress } if !progress.is_empty() => {
            format!(" · {progress}")
        }
        _ => String::new(),
    };
    let title = format!("{} · {}{progress}", scope(record), safe(&record.spec.title));
    let time = local_time(&record.started_at);
    if width < 42 {
        line.spans.push(Span::raw(truncate(&title, content_width)));
        return vec![
            line,
            Line::styled(truncate(&time, width), Style::default().fg(MUTED)).right_aligned(),
        ];
    }
    let title_width = content_width.saturating_sub(time.width() + 1);
    let title = truncate(&title, title_width);
    let padding = " ".repeat(title_width.saturating_sub(title.width()) + 1);
    line.spans.extend([
        Span::raw(title),
        Span::raw(padding),
        Span::styled(time, Style::default().fg(MUTED)),
    ]);
    vec![line]
}
fn reason(record: &Record, width: usize) -> Vec<Line<'static>> {
    let State::Finished {
        outcome: Completion::Failed {
            detail, summary, ..
        },
        ..
    } = &record.state
    else {
        return vec![];
    };
    if width <= 2 {
        return vec![];
    }
    let reason = if matches!(record.spec.scope, TaskScope::All) {
        detail
    } else {
        summary.as_deref().unwrap_or(detail)
    };
    let text = multiline(reason);
    let color = if record.unread_failure() { BAD } else { MUTED };
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
        .map(|line| Line::styled(format!("╰─{line}"), Style::default().fg(color)))
        .collect()
}
pub(super) fn render(frame: &mut Frame, screen: &mut Screen, area: Rect) {
    let inner = list_heading::render(
        frame,
        i18n::tr!(Tasks),
        ratatui::widgets::Padding::horizontal(1),
        &[list_heading::Field::new(
            i18n::tr!(RepositoryFilterLabel),
            screen.tasks.filter.label(),
            crate::pointer::Action::Command(Action::TaskFilterMenu),
        )],
        area,
        &mut screen.pointer,
    );
    screen.tasks.content = inner;
    let visible = screen.tasks.visible_indices();
    let records = &screen.tasks.records;
    let index = screen
        .tasks
        .selection
        .as_ref()
        .and_then(|id| visible.iter().position(|&index| records[index].id == *id));
    screen.tasks.viewport.select(index);
    if inner.is_empty() {
        return;
    }
    if visible.is_empty() {
        frame.render_widget(
            Paragraph::new(if records.is_empty() {
                i18n::tr!(TasksEmpty)
            } else {
                i18n::tr!(TasksNoMatches)
            })
            .style(Style::default().fg(MUTED)),
            inner,
        );
        return;
    }
    let entries = visible
        .iter()
        .map(|&index| {
            let record = &records[index];
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
    .highlight_style(selected_style());
    frame.render_stateful_widget(widget, inner, &mut screen.tasks.viewport);
    let mut row = 0usize;
    let offset = screen.tasks.viewport.offset();
    for (row_index, &index) in visible.iter().enumerate().skip(offset) {
        let record = &records[index];
        let height = heights[row_index];
        if row + height > usize::from(inner.height) {
            break;
        }
        let y = inner.y + row as u16;
        if record.unread_failure() && inner.x > area.x {
            frame.buffer_mut()[(inner.x - 1, y)]
                .set_symbol("•")
                .set_fg(WARN);
        }
        screen.tasks.rows.push((
            Rect::new(inner.x, y, inner.width, height as u16),
            record.id.clone(),
        ));
        row += height;
    }
    let total = heights.iter().sum();
    let position = heights.iter().take(offset).sum();
    crate::scrollbar::vertical(frame, area, inner, total, position);
}
