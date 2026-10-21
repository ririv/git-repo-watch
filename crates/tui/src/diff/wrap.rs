//! Hard wrap styled graphemes without dropping indentation or splitting wide characters.
use super::*;
use unicode_width::UnicodeWidthStr;

pub(super) struct Fragment {
    pub offset: usize,
    pub line: Line<'static>,
}
pub(super) fn line(line: &Line<'_>, width: u16) -> Vec<Fragment> {
    let width = usize::from(width.max(1));
    let mut out = Vec::new();
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut columns = 0;
    let mut offset = 0;
    let mut start = 0;
    for grapheme in line.styled_graphemes(Style::default()) {
        let size = UnicodeWidthStr::width(grapheme.symbol);
        if columns > 0 && columns + size > width {
            out.push(Fragment {
                offset: start,
                line: Line::from(std::mem::take(&mut spans)).style(line.style),
            });
            start = offset;
            columns = 0;
        }
        if let Some(last) = spans.last_mut()
            && last.style == grapheme.style
        {
            last.content.to_mut().push_str(grapheme.symbol);
        } else {
            spans.push(Span::styled(grapheme.symbol.to_owned(), grapheme.style));
        }
        columns += size;
        offset += grapheme.symbol.len();
    }
    if !spans.is_empty() || out.is_empty() {
        out.push(Fragment {
            offset: start,
            line: Line::from(spans).style(line.style),
        });
    }
    out
}
