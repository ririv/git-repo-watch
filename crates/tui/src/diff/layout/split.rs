//! Pair edits within each change block, padding both logical and wrapped line gaps.
use super::*;

type Entry<'a> = (usize, &'a Code, Mark);
enum Pair<'a> {
    Both(Entry<'a>, Entry<'a>),
    Removed(Entry<'a>),
    Added(Entry<'a>),
}

pub(super) fn arrange(rows: &[Row], digits: u16, width: u16) -> Vec<VisualRow> {
    let mut out = Vec::new();
    let left_width = width.saturating_sub(1) / 2;
    let right_width = width.saturating_sub(1) - left_width;
    let mut index = 0;
    while index < rows.len() {
        match &rows[index] {
            Row::Metadata(line) => {
                full(&mut out, index, line, width);
                index += 1;
            }
            Row::Context { old, new } => {
                pair(
                    &mut out,
                    Pair::Both((index, old, Mark::Context), (index, new, Mark::Context)),
                    digits,
                    left_width,
                    right_width,
                );
                index += 1;
            }
            Row::Added(_) | Row::Removed(_) => {
                let end = rows[index..]
                    .iter()
                    .position(|row| !row.is_edit())
                    .map_or(rows.len(), |offset| index + offset);
                let mut removed = Vec::new();
                let mut added = Vec::new();
                for (offset, row) in rows[index..end].iter().enumerate() {
                    match row {
                        Row::Removed(code) => removed.push((index + offset, code, Mark::Removed)),
                        Row::Added(code) => added.push((index + offset, code, Mark::Added)),
                        _ => unreachable!(),
                    }
                }
                for row in 0..removed.len().max(added.len()) {
                    let entry = match (removed.get(row).copied(), added.get(row).copied()) {
                        (Some(old), Some(new)) => Pair::Both(old, new),
                        (Some(old), None) => Pair::Removed(old),
                        (None, Some(new)) => Pair::Added(new),
                        (None, None) => unreachable!(),
                    };
                    pair(&mut out, entry, digits, left_width, right_width);
                }
                index = end;
            }
        }
    }
    out
}
fn column((row, code, mark): Entry<'_>, digits: u16, width: u16) -> Vec<(Line<'static>, Anchor)> {
    let number_width = usize::from(digits);
    wrap::line(&code.content, width.saturating_sub(digits + 3).max(1))
        .into_iter()
        .enumerate()
        .map(|(index, part)| {
            let number = if index == 0 {
                format!("{:>number_width$}", code.number)
            } else {
                " ".repeat(number_width)
            };
            (
                decorate(part.line, format!("{number} │"), mark, index == 0),
                Anchor {
                    row,
                    offset: part.offset,
                },
            )
        })
        .collect()
}
fn pair(out: &mut Vec<VisualRow>, entry: Pair<'_>, digits: u16, left_width: u16, right_width: u16) {
    let (old, new) = match entry {
        Pair::Both(old, new) => (
            column(old, digits, left_width),
            column(new, digits, right_width),
        ),
        Pair::Removed(old) => (column(old, digits, left_width), Vec::new()),
        Pair::Added(new) => (Vec::new(), column(new, digits, right_width)),
    };
    let mut old = old.into_iter();
    let mut new = new.into_iter();
    loop {
        let (left, right, anchors) = match (old.next(), new.next()) {
            (Some((left, a)), Some((right, b))) => (left, right, Anchors::Pair(a, b)),
            (Some((left, anchor)), None) => (left, Line::default(), Anchors::One(anchor)),
            (None, Some((right, anchor))) => (Line::default(), right, Anchors::One(anchor)),
            (None, None) => break,
        };
        out.push(VisualRow::Columns {
            left,
            right,
            anchors,
        });
    }
}
