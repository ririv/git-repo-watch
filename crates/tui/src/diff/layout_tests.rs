use super::*;
use layout::{Anchor, Anchors};
use unicode_width::UnicodeWidthStr;

fn code(number: u32, text: &str) -> Code {
    Code {
        number: NonZeroU32::new(number).unwrap(),
        content: Line::raw(text.to_owned()),
    }
}
fn columns(cache: &Cache) -> Vec<(&Line<'_>, &Line<'_>)> {
    cache
        .rows
        .iter()
        .filter_map(|row| match row {
            VisualRow::Columns { left, right, .. } => Some((left, right)),
            _ => None,
        })
        .collect()
}

#[test]
fn automatic_layout_obeys_width_and_code_space_while_manual_modes_are_explicit() {
    for digits in [1, 5] {
        assert_eq!(Preference::Auto.resolve(139, digits), Layout::Unified);
        assert_eq!(Preference::Auto.resolve(140, digits), Layout::Split);
    }
    assert_eq!(Preference::Auto.resolve(140, 10), Layout::Unified);
    assert_eq!(Preference::Auto.resolve(160, 10), Layout::Split);
    assert_eq!(Preference::Unified.resolve(200, 1), Layout::Unified);
    assert_eq!(Preference::Split.resolve(80, 1), Layout::Split);
}

#[test]
fn unequal_edits_and_unequal_wrapping_pad_each_side_before_the_next_context() {
    let rows = vec![
        Row::Removed(code(1, &"中文".repeat(60))),
        Row::Removed(code(2, "extra deletion")),
        Row::Added(code(1, "short replacement")),
        Row::Context {
            old: code(3, "following context"),
            new: code(2, "following context"),
        },
    ];
    let cache = layout::arrange(&rows, 1, 140, Layout::Split);
    let pairs = columns(&cache);
    assert!(pairs[0].0.to_string().contains("1 │-"));
    assert!(pairs[0].1.to_string().contains("1 │+short"));
    let next = pairs
        .iter()
        .position(|(left, _)| left.to_string().contains("following context"))
        .unwrap();
    assert!(next >= 4);
    assert!(pairs[next].1.to_string().contains("following context"));
    assert!(
        pairs[1..next]
            .iter()
            .all(|(_, right)| right.to_string().is_empty())
    );
    assert!(
        pairs
            .iter()
            .all(|(left, right)| left.width() <= 69 && right.width() <= 70)
    );
    assert!(pairs[1].0.to_string().starts_with("  │ "));
}

#[test]
fn pure_addition_deletion_and_empty_lines_stay_on_the_correct_side() {
    for (row, added) in [
        (Row::Added(code(1, "added")), true),
        (Row::Removed(code(1, "removed")), false),
        (Row::Added(code(1, "")), true),
    ] {
        let cache = layout::arrange(&[row], 1, 140, Layout::Split);
        let pairs = columns(&cache);
        assert_eq!(pairs.len(), 1);
        let (left, right) = pairs[0];
        if added {
            assert!(left.to_string().is_empty());
            assert!(right.to_string().contains("1 │+"));
        } else {
            assert!(right.to_string().is_empty());
            assert!(left.to_string().contains("1 │-"));
        }
    }
}

#[test]
fn unified_order_stays_removed_then_added_and_metadata_is_not_duplicated() {
    let rows = vec![
        Row::Metadata(Line::raw("@@ block @@")),
        Row::Removed(code(1, "old one")),
        Row::Removed(code(2, "old two")),
        Row::Added(code(1, "new one")),
    ];
    let unified = layout::arrange(&rows, 1, 80, Layout::Unified);
    let text = unified
        .rows
        .iter()
        .map(|row| match row {
            VisualRow::Full { line, .. } => line.to_string(),
            _ => unreachable!(),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        text,
        [
            "@@ block @@",
            "1   │-old one",
            "2   │-old two",
            "  1 │+new one"
        ]
    );
    let split = layout::arrange(&rows, 1, 140, Layout::Split);
    assert_eq!(split.rows.iter().filter(|row| matches!(row, VisualRow::Full { line, .. } if line.to_string() == "@@ block @@")).count(), 1);
}

#[test]
fn repeated_resize_and_mode_changes_keep_an_exact_source_anchor_without_drift() {
    let mut view = View {
        export: Export::Unavailable,
        rows: vec![
            Row::Metadata(Line::raw("header")),
            Row::Removed(code(1, "old")),
            Row::Added(code(1, &"long content ".repeat(80))),
        ],
        digits: 1,
        comparison: Comparison::Working,
        cache: None,
    };
    let mut scroll = 0;
    let cache = view.arrange(80, Preference::Auto, &mut scroll);
    scroll = cache
        .rows
        .iter()
        .position(
            |row| matches!(row.anchors(), Anchors::One(Anchor { row: 2, offset }) if offset >= 140),
        )
        .unwrap();
    cache.settle(scroll);
    let original_scroll = scroll;
    let original_anchor = cache.anchor(scroll).unwrap();
    for (width, preference) in [
        (160, Preference::Auto),
        (80, Preference::Auto),
        (200, Preference::Split),
        (200, Preference::Unified),
        (80, Preference::Auto),
    ] {
        let cache = view.arrange(width, preference, &mut scroll);
        cache.settle(scroll);
        assert_eq!(cache.anchor(scroll), Some(original_anchor));
    }
    assert_eq!(scroll, original_scroll);
    scroll += 1;
    view.arrange(80, Preference::Auto, &mut scroll)
        .settle(scroll);
    let moved = view.cache.as_ref().unwrap().anchor(scroll).unwrap();
    assert!(moved.offset > original_anchor.offset);
    assert_eq!(
        view.arrange(160, Preference::Auto, &mut scroll)
            .anchor(scroll),
        Some(moved)
    );
}

#[test]
fn styled_grapheme_wrapping_retains_unicode_whitespace_colors_and_byte_anchors() {
    let line = Line::from(vec![
        Span::styled("  中文e\u{301}👩‍💻", Style::default().fg(Color::LightBlue)),
        Span::styled(" tail end", Style::default().fg(Color::LightGreen)),
    ]);
    for width in [2, 5, 10] {
        let parts = wrap::line(&line, width);
        assert!(
            parts.iter().all(
                |part| UnicodeWidthStr::width(part.line.to_string().as_str()) <= width as usize
            )
        );
        assert_eq!(
            parts
                .iter()
                .map(|part| part.line.to_string())
                .collect::<String>(),
            line.to_string()
        );
        for part in parts {
            assert!(line.to_string().is_char_boundary(part.offset));
            assert!(part.line.spans.iter().all(|span| span.style.fg.is_some()));
        }
    }
}
