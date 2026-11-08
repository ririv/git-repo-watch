//! Pair edits within each change block without wrapping source lines.
use super::*;

type Entry<'a> = (usize, &'a Code, Mark);
enum Pair<'a> {
    Both(Entry<'a>, Entry<'a>),
    Removed(Entry<'a>),
    Added(Entry<'a>),
}

pub(super) fn arrange(rows: &[Row], digits: u16) -> Vec<VisualRow> {
    let mut out = Vec::new();
    let mut index = 0;
    while index < rows.len() {
        match &rows[index] {
            Row::Information(line) | Row::Metadata(line) => {
                full(&mut out, index, line);
                index += 1;
            }
            Row::Context { old, new } => {
                pair(
                    &mut out,
                    Pair::Both((index, old, Mark::Context), (index, new, Mark::Context)),
                    digits,
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
                    pair(&mut out, entry, digits);
                }
                index = end;
            }
        }
    }
    out
}
fn column((row, code, mark): Entry<'_>, digits: u16) -> (crate::links::Line, Anchor) {
    let number_width = usize::from(digits);
    (
        decorate(
            code.content.clone(),
            format!("{:>number_width$} ", code.number),
            mark,
        ),
        Anchor { row },
    )
}
fn pair(out: &mut Vec<VisualRow>, entry: Pair<'_>, digits: u16) {
    let gutter = usize::from(digits) + 3;
    let ending = |entry: Entry<'_>| match entry.1.ending {
        Ending::Newline => Line::default().into(),
        Ending::EndOfFile => notice::ending(gutter),
    };
    let (left_ending, right_ending) = match &entry {
        Pair::Both(old, new) => (ending(*old), ending(*new)),
        Pair::Removed(old) => (ending(*old), Line::default().into()),
        Pair::Added(new) => (Line::default().into(), ending(*new)),
    };
    let (left, right, anchors) = match entry {
        Pair::Both(old, new) => {
            let (left, a) = column(old, digits);
            let (right, b) = column(new, digits);
            (left, right, Anchors::Pair(a, b))
        }
        Pair::Removed(old) => {
            let (left, anchor) = column(old, digits);
            (left, Line::default().into(), Anchors::One(anchor))
        }
        Pair::Added(new) => {
            let (right, anchor) = column(new, digits);
            (Line::default().into(), right, Anchors::One(anchor))
        }
    };
    out.push(VisualRow::Columns {
        left,
        right,
        anchors,
        fixed: gutter,
        kind: RowKind::Content,
    });
    if left_ending.width() > 0 || right_ending.width() > 0 {
        out.push(VisualRow::Columns {
            fixed: left_ending.width().max(right_ending.width()),
            left: left_ending,
            right: right_ending,
            anchors,
            kind: RowKind::Annotation,
        });
    }
}
