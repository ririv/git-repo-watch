use super::*;

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
            VisualRow::Columns { left, right, .. } => Some((&left.text, &right.text)),
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
fn unequal_edits_keep_logical_rows_and_pad_the_missing_side() {
    let rows = vec![
        Row::Removed(code(1, &"中文".repeat(60))),
        Row::Removed(code(2, "extra deletion")),
        Row::Added(code(1, "short replacement")),
        Row::Context {
            old: code(3, "following context"),
            new: code(2, "following context"),
        },
    ];
    let cache = layout::arrange(&rows, 1, 140, Layout::Split, false, &[]);
    let pairs = columns(&cache);
    assert!(pairs[0].0.to_string().contains("1 │-"));
    assert!(pairs[0].1.to_string().contains("1 │+short"));
    let next = pairs
        .iter()
        .position(|(left, _)| left.to_string().contains("following context"))
        .unwrap();
    assert_eq!(next, 2);
    assert!(pairs[next].1.to_string().contains("following context"));
    assert!(
        pairs[1..next]
            .iter()
            .all(|(_, right)| right.to_string().is_empty())
    );
    assert!(pairs[0].0.width() > 69);
    assert!(pairs[1].0.to_string().starts_with("2 │-"));
}

#[test]
fn pure_addition_deletion_and_empty_lines_stay_on_the_correct_side() {
    for (row, added) in [
        (Row::Added(code(1, "added")), true),
        (Row::Removed(code(1, "removed")), false),
        (Row::Added(code(1, "")), true),
    ] {
        let cache = layout::arrange(&[row], 1, 140, Layout::Split, false, &[]);
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
    let unified = layout::arrange(&rows, 1, 80, Layout::Unified, false, &[]);
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
    let split = layout::arrange(&rows, 1, 140, Layout::Split, false, &[]);
    assert_eq!(split.rows.iter().filter(|row| matches!(row, VisualRow::Full { line, .. } if line.to_string() == "@@ block @@")).count(), 1);
}

#[test]
fn repeated_resize_and_mode_changes_keep_an_exact_source_anchor_without_drift() {
    let mut view = View {
        interaction: Interaction::ReadOnly,
        export: Export::Unavailable,
        rows: vec![
            Row::Metadata(Line::raw("header")),
            Row::Removed(code(1, "old")),
            Row::Added(code(1, &"long content ".repeat(80))),
        ],
        digits: 1,
        comparison: Comparison::Working,
        cache: None,
        horizontal: Default::default(),
    };
    let mut scroll = 2;
    let cache = view.arrange(80, Preference::Auto, false, &[], &mut scroll);
    cache.settle(scroll);
    let original_scroll = scroll;
    let original_anchor = cache.anchor(scroll).unwrap();
    view.horizontal.advance(100);
    for (width, preference) in [
        (160, Preference::Auto),
        (80, Preference::Auto),
        (200, Preference::Split),
        (200, Preference::Unified),
        (80, Preference::Auto),
    ] {
        let cache = view.arrange(width, preference, false, &[], &mut scroll);
        cache.settle(scroll);
        assert_eq!(cache.anchor(scroll), Some(original_anchor));
        assert_eq!(view.horizontal.offset(), 100);
    }
    assert_eq!(scroll, original_scroll);
    view.arrange(2000, Preference::Unified, false, &[], &mut scroll);
    assert_eq!(view.horizontal.offset(), 0);
    assert!(!view.horizontal.overflowing());
}

#[test]
fn wrapped_split_rows_pad_the_short_side_and_preserve_gutters_backgrounds_and_links() {
    let url = "https://example.invalid/complete/long/target/path";
    let rows = vec![
        Row::Removed(code(
            1,
            &format!("中文 e\u{301}👩‍💻 {url} {}", "long".repeat(20)),
        )),
        Row::Added(code(1, "short replacement")),
        Row::Context {
            old: code(2, "FOLLOWING"),
            new: code(2, "FOLLOWING"),
        },
    ];
    let cache = layout::arrange(&rows, 1, 51, Layout::Split, true, &[]);
    let pairs = columns(&cache);
    let next = pairs
        .iter()
        .position(|(left, _)| left.to_string().contains("FOLLOWING"))
        .unwrap();
    assert!(next > 3);
    assert!(pairs[next].1.to_string().contains("FOLLOWING"));
    assert!(
        pairs[1..next]
            .iter()
            .all(|(_, right)| right.to_string().is_empty())
    );
    assert!(
        pairs
            .iter()
            .all(|(left, right)| left.width() <= 25 && right.width() <= 25)
    );
    assert!(pairs[1..next].iter().all(|(left, _)| left.to_string().starts_with("    ") && left.style.bg == Some(REMOVED)));
    let anchors = cache
        .rows
        .iter()
        .filter_map(|row| match row {
            VisualRow::Columns { left, .. } => Some(&left.anchors),
            _ => None,
        })
        .flatten()
        .collect::<Vec<_>>();
    assert!(anchors.len() > 1);
    assert!(anchors.iter().all(|anchor| anchor.target.uri() == url));
    assert_eq!(cache.horizontal_maximum(), 0);
    let unified = layout::arrange(&rows, 1, 32, Layout::Unified, true, &[]);
    assert!(unified.rows.len() > 3);
    assert!(
        unified
            .rows
            .iter()
            .all(|row| matches!(row, VisualRow::Full { line, .. } if line.width() <= 32))
    );
}
