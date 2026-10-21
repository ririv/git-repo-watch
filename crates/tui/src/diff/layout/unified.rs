//! Unified rows retain the patch order and both source line numbers.
use super::*;

pub(super) fn arrange(rows: &[Row], digits: u16, width: u16) -> Vec<VisualRow> {
    let mut out = Vec::new();
    let number_width = usize::from(digits);
    let code_width = width.saturating_sub(2 * digits + 4).max(1);
    for (index, row) in rows.iter().enumerate() {
        let (code, old, new, mark) = match row {
            Row::Metadata(line) => {
                full(&mut out, index, line, width);
                continue;
            }
            Row::Context { old, new } => (new, Some(old.number), Some(new.number), Mark::Context),
            Row::Added(code) => (code, None, Some(code.number), Mark::Added),
            Row::Removed(code) => (code, Some(code.number), None, Mark::Removed),
        };
        for (fragment, part) in wrap::line(&code.content, code_width)
            .into_iter()
            .enumerate()
        {
            let number = |number: Option<NonZeroU32>| {
                if fragment == 0 {
                    number
                        .map(|n| format!("{n:>number_width$}"))
                        .unwrap_or_else(|| " ".repeat(number_width))
                } else {
                    " ".repeat(number_width)
                }
            };
            let gutter = format!("{} {} │", number(old), number(new));
            out.push(VisualRow::Full {
                anchor: Anchor {
                    row: index,
                    offset: part.offset,
                },
                line: decorate(part.line, gutter, mark, fragment == 0),
            });
        }
    }
    out
}
