//! Width-aware tables preserve cell styles and CJK display widths.
use super::*;
use pulldown_cmark::Alignment;
use unicode_width::UnicodeWidthChar;

pub(super) fn render<'a>(
    events: &mut impl Iterator<Item = Event<'a>>,
    align: &[Alignment],
    width: u16,
) -> Vec<Line<'static>> {
    let mut rows = Vec::<Vec<Line<'static>>>::new();
    let mut row = Vec::new();
    let mut cell = Renderer::default();
    for event in events.by_ref() {
        match event {
            Event::End(TagEnd::Table) => break,
            Event::Start(Tag::TableCell) => cell = Renderer::default(),
            Event::End(TagEnd::TableCell) => {
                cell.flush();
                row.push(cell.lines.pop().unwrap_or_default());
            }
            Event::End(TagEnd::TableHead | TagEnd::TableRow) => rows.push(std::mem::take(&mut row)),
            Event::Start(Tag::TableHead | Tag::TableRow) => {}
            other => cell.event(other),
        }
    }
    let count = align.len();
    if count == 0 {
        return Vec::new();
    }
    let mut widths = (0..count)
        .map(|column| {
            rows.iter()
                .filter_map(|row| row.get(column))
                .map(Line::width)
                .max()
                .unwrap_or(1)
                .max(2)
        })
        .collect::<Vec<_>>();
    // 窄到无法容纳边框时改为逐字段展示，保持完整内容和表头对应关系。
    let minimum = rows
        .first()
        .map(|row| {
            row.iter()
                .map(|cell| cell.width().max(2))
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| vec![2; count]);
    if usize::from(width) < minimum.iter().sum::<usize>() + count * 3 + 1 {
        return stacked(rows);
    }
    let budget = usize::from(width).saturating_sub(count * 3 + 1);
    while widths.iter().sum::<usize>() > budget {
        let index = widths
            .iter()
            .enumerate()
            .filter(|(index, value)| **value > minimum[*index])
            .max_by_key(|(_, value)| *value)
            .map(|(index, _)| index)
            .expect("nonempty table");
        widths[index] -= 1;
    }
    let border = |left: &str, middle: &str, right: &str| {
        Line::styled(
            format!(
                "{left}{}{right}",
                widths
                    .iter()
                    .map(|w| "─".repeat(w + 2))
                    .collect::<Vec<_>>()
                    .join(middle)
            ),
            Style::default().fg(MUTED),
        )
    };
    let mut output = vec![border("┌", "┬", "┐")];
    for (index, row) in rows.iter().enumerate() {
        let cells = row
            .iter()
            .zip(&widths)
            .map(|(cell, width)| wrap(cell, *width))
            .collect::<Vec<_>>();
        for y in 0..cells.iter().map(Vec::len).max().unwrap_or(1) {
            let mut spans = vec![Span::styled("│", Style::default().fg(MUTED))];
            for (column, width) in widths.iter().enumerate() {
                let line = cells
                    .get(column)
                    .and_then(|cell| cell.get(y))
                    .cloned()
                    .unwrap_or_default();
                let padding = width.saturating_sub(line.width());
                let left = match align[column] {
                    Alignment::Right => padding,
                    Alignment::Center => padding / 2,
                    _ => 0,
                };
                spans.push(Span::raw(" ".repeat(left + 1)));
                spans.extend(line.spans.into_iter().map(|span| {
                    if index == 0 {
                        span.patch_style(Style::default().add_modifier(Modifier::BOLD))
                    } else {
                        span
                    }
                }));
                spans.push(Span::raw(" ".repeat(padding - left + 1)));
                spans.push(Span::styled("│", Style::default().fg(MUTED)));
            }
            output.push(Line::from(spans));
        }
        if index == 0 {
            output.push(border("├", "┼", "┤"));
        }
    }
    output.push(border("└", "┴", "┘"));
    output
}

fn stacked(rows: Vec<Vec<Line<'static>>>) -> Vec<Line<'static>> {
    let Some(headers) = rows.first() else {
        return Vec::new();
    };
    if rows.len() == 1 {
        return headers.clone();
    }
    let mut result = Vec::new();
    for row in rows.iter().skip(1) {
        for (header, cell) in headers.iter().zip(row) {
            let mut spans = header.spans.clone();
            spans.push(Span::raw(": "));
            spans.extend(cell.spans.clone());
            result.push(Line::from(spans));
        }
        result.push(Line::default());
    }
    result
}

fn wrap(line: &Line<'static>, width: usize) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut spans = Vec::new();
    let mut used = 0;
    for span in &line.spans {
        for c in span.content.chars() {
            let size = c.width().unwrap_or(0);
            if used + size > width && used > 0 {
                lines.push(Line::from(std::mem::take(&mut spans)));
                used = 0;
            }
            spans.push(Span::styled(c.to_string(), span.style));
            used += size;
        }
    }
    lines.push(Line::from(spans));
    lines
}
