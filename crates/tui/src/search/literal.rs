//! Literal matching maps folded Unicode back to complete terminal-cell ranges.
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

pub(super) fn ranges(body: &str, needle: &str) -> Vec<Range<usize>> {
    if body.is_ascii() {
        return body
            .to_ascii_lowercase()
            .match_indices(needle)
            .map(|(start, value)| start..start + value.len())
            .collect();
    }
    let mut folded = String::new();
    let mut map = Vec::new();
    let mut column = 0;
    for grapheme in body.graphemes(true) {
        folded.push_str(&grapheme.to_lowercase());
        let next = column + grapheme.width();
        map.push((folded.len(), column..next));
        column = next;
    }
    let mut result = Vec::new();
    for (start, value) in folded.match_indices(needle) {
        let first = map.partition_point(|(end, _)| *end <= start);
        let last = map.partition_point(|(end, _)| *end < start + value.len());
        let columns = map[first].1.start..map[last].1.end;
        if result.last() != Some(&columns) {
            result.push(columns);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unicode_case_expansion_and_combining_graphemes_keep_real_cell_coordinates() {
        assert_eq!(ranges("中İ e\u{301}👩‍💻 中İ", "i"), vec![2..3, 10..11]);
        assert_eq!(ranges("e\u{301}e\u{301}", "\u{301}"), vec![0..1, 1..2]);
        assert_eq!(ranges("Foo FOO foo", "foo"), vec![0..3, 4..7, 8..11]);
    }
}
