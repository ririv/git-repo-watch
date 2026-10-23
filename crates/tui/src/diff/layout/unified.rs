//! Unified rows retain the patch order and both source line numbers.
use super::*;

pub(super) fn arrange(rows: &[Row], digits: u16) -> Vec<VisualRow> {
    let mut out = Vec::new();
    let number_width = usize::from(digits);
    for (index, row) in rows.iter().enumerate() {
        let (code, old, new, mark) = match row {
            Row::Metadata(line) => {
                full(&mut out, index, line);
                continue;
            }
            Row::Context { old, new } => (new, Some(old.number), Some(new.number), Mark::Context),
            Row::Added(code) => (code, None, Some(code.number), Mark::Added),
            Row::Removed(code) => (code, Some(code.number), None, Mark::Removed),
        };
        let number = |number: Option<NonZeroU32>| {
            number
                .map(|n| format!("{n:>number_width$}"))
                .unwrap_or_else(|| " ".repeat(number_width))
        };
        let gutter = format!("{} {} │", number(old), number(new));
        out.push(VisualRow::Full {
            anchor: Anchor { row: index },
            fixed: 2 * number_width + 4,
            line: decorate(code.content.clone(), gutter, mark),
        });
    }
    out
}
