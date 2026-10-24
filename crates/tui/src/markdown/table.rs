//! Width-aware tables preserve cell styles and CJK display widths.
use super::*;
use pulldown_cmark::Alignment;

pub(super) fn render<'a>(
    events: &mut impl Iterator<Item = Event<'a>>,
    align: &[Alignment],
    width: u16,
    base: Option<&Path>,
) -> Vec<LinkedLine> {
    let mut rows = Vec::<Vec<LinkedLine>>::new();
    let mut row = Vec::new();
    let mut cell = Renderer {
        base: base.map(Path::to_owned),
        ..Renderer::default()
    };
    for event in events.by_ref() {
        match event {
            Event::End(TagEnd::Table) => break,
            Event::Start(Tag::TableCell) => {
                cell = Renderer {
                    base: base.map(Path::to_owned),
                    ..Renderer::default()
                }
            }
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
                .map(LinkedLine::width)
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
        LinkedLine::from(Line::styled(
            format!(
                "{left}{}{right}",
                widths
                    .iter()
                    .map(|w| "─".repeat(w + 2))
                    .collect::<Vec<_>>()
                    .join(middle)
            ),
            Style::default().fg(MUTED),
        ))
    };
    let mut output = vec![border("┌", "┬", "┐")];
    for (index, row) in rows.iter().enumerate() {
        let cells = row
            .iter()
            .zip(&widths)
            .map(|(cell, width)| wrap(cell, *width))
            .collect::<Vec<_>>();
        for y in 0..cells.iter().map(Vec::len).max().unwrap_or(1) {
            let mut output_line = LinkedLine::from(Line::from(vec![Span::styled(
                "│",
                Style::default().fg(MUTED),
            )]));
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
                output_line.append(LinkedLine::from(Line::from(Span::raw(
                    " ".repeat(left + 1),
                ))));
                let mut line = line;
                if index == 0 {
                    for span in &mut line.text.spans {
                        span.style = span
                            .style
                            .patch(Style::default().add_modifier(Modifier::BOLD));
                    }
                }
                output_line.append(line);
                output_line.append(LinkedLine::from(Line::from(vec![
                    Span::raw(" ".repeat(padding - left + 1)),
                    Span::styled("│", Style::default().fg(MUTED)),
                ])));
            }
            output.push(output_line);
        }
        if index == 0 {
            output.push(border("├", "┼", "┤"));
        }
    }
    output.push(border("└", "┴", "┘"));
    output
}

fn stacked(rows: Vec<Vec<LinkedLine>>) -> Vec<LinkedLine> {
    let Some(headers) = rows.first() else {
        return Vec::new();
    };
    if rows.len() == 1 {
        return headers.clone();
    }
    let mut result = Vec::new();
    for row in rows.iter().skip(1) {
        for (header, cell) in headers.iter().zip(row) {
            let mut line = header.clone();
            line.append(LinkedLine::from(Line::from(Span::raw(": "))));
            line.append(cell.clone());
            result.push(line);
        }
        result.push(LinkedLine::default());
    }
    result
}

fn wrap(line: &LinkedLine, width: usize) -> Vec<LinkedLine> {
    line.wrap(width, false)
}
