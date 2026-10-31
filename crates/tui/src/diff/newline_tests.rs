use super::*;
use engine::local::files;
use test_support::{Fixture, git, write};

fn annotation(line: &crate::links::Line) {
    assert_eq!(line.to_string().trim(), notice::NO_NEWLINE);
    assert_eq!(line.text.style.bg, None);
    let mark = line.text.spans.last().unwrap();
    assert_eq!(mark.content, notice::NO_NEWLINE);
    assert_eq!(mark.style.fg, Some(Color::LightRed));
}

#[test]
fn newline_markers_are_separate_unnumbered_rows_on_affected_sides_and_keep_export() {
    let f = Fixture::create();
    let cases = [
        ("add-lf.txt", Some("same"), "same\n", [true, false]),
        ("remove-lf.txt", Some("same\n"), "same", [false, true]),
        ("neither-lf.txt", Some("old"), "new", [true, true]),
        ("new-file.txt", None, "added", [false, true]),
        ("deleted-file.txt", Some("removed"), "", [true, false]),
        ("normal.txt", Some("old\n"), "new\n", [false, false]),
    ];
    for (name, before, _, _) in cases {
        if let Some(before) = before {
            f.commit(name, before);
        }
    }
    for (name, _, after, _) in cases {
        if name == "deleted-file.txt" {
            std::fs::remove_file(f.repo.join(name)).unwrap();
        } else {
            write(f.repo.join(name), after);
        }
    }
    git(&f.repo, &["add", "-A"]);
    let snapshot = files::read(&f.repo).unwrap();
    for (name, before, after, missing) in cases {
        let change = snapshot
            .changes
            .iter()
            .find(|change| change.location.name() == std::path::Path::new(name))
            .unwrap();
        let patch = files::document(&snapshot, change, &Cancel::default()).unwrap();
        let original = patch.text();
        let view = prepare(name, patch, Area::Staged, &Cancel::default()).unwrap();
        for preference in [Preference::Unified, Preference::Split] {
            let cache = view.layout(180, preference, false, &[], HeaderVisibility::Collapsed);
            let annotations = cache
                .rows
                .iter()
                .filter(|row| {
                    matches!(
                        row,
                        VisualRow::Full {
                            kind: RowKind::Annotation,
                            ..
                        } | VisualRow::Columns {
                            kind: RowKind::Annotation,
                            ..
                        }
                    )
                })
                .collect::<Vec<_>>();
            match preference {
                Preference::Split => {
                    assert_eq!(
                        annotations.len(),
                        usize::from(missing.contains(&true)),
                        "{name}"
                    );
                    if let Some(VisualRow::Columns {
                        left, right, fixed, ..
                    }) = annotations.first()
                    {
                        for (line, missing) in [(left, missing[0]), (right, missing[1])] {
                            if missing {
                                annotation(line);
                                assert_eq!(*fixed, line.width());
                            } else {
                                assert!(line.to_string().is_empty());
                            }
                        }
                    }
                    let source = cache
                        .rows
                        .iter()
                        .filter_map(|row| match row {
                            VisualRow::Columns {
                                left,
                                right,
                                kind: RowKind::Content,
                                ..
                            } => Some((left, right)),
                            _ => None,
                        })
                        .collect::<Vec<_>>();
                    assert_eq!(source.len(), 1, "删除与新增仍应成对排列: {name}");
                    let body = |line: &crate::links::Line| {
                        line.to_string().chars().skip(4).collect::<String>()
                    };
                    assert_eq!(
                        body(source[0].0),
                        before.unwrap_or_default().trim_end_matches('\n')
                    );
                    assert_eq!(body(source[0].1), after.trim_end_matches('\n'));
                }
                Preference::Unified => {
                    assert_eq!(
                        annotations.len(),
                        missing.iter().filter(|missing| **missing).count(),
                        "{name}"
                    );
                    for row in annotations {
                        let VisualRow::Full { line, fixed, .. } = row else {
                            unreachable!()
                        };
                        annotation(line);
                        assert_eq!(*fixed, line.width());
                    }
                }
                Preference::Auto => unreachable!(),
            }
            for (index, row) in cache.rows.iter().enumerate() {
                let anchor = row.anchors().preferred();
                if matches!(
                    row,
                    VisualRow::Full {
                        kind: RowKind::Annotation,
                        ..
                    } | VisualRow::Columns {
                        kind: RowKind::Annotation,
                        ..
                    }
                ) {
                    assert!(cache.locate(anchor) < index);
                }
            }
        }
        assert!(matches!(&view.export, Export::Complete(export) if export.as_ref() == original));
    }
}

#[test]
fn unchanged_eof_context_marks_both_split_sides_and_literal_git_notices_stay_code() {
    let f = Fixture::create();
    f.commit("context.txt", "old\nstable");
    write(f.repo.join("context.txt"), "new\nstable");
    write(f.repo.join("literal.txt"), "\\ No newline at end of file\n");
    let snapshot = files::read(&f.repo).unwrap();
    for change in &snapshot.changes {
        let patch = files::document(&snapshot, change, &Cancel::default()).unwrap();
        let view = prepare("file", patch, change.area, &Cancel::default()).unwrap();
        let cache = view.layout(
            180,
            Preference::Split,
            false,
            &[],
            HeaderVisibility::Collapsed,
        );
        let rows = cache
            .rows
            .iter()
            .filter_map(|row| match row {
                VisualRow::Columns {
                    left, right, kind, ..
                } => Some((left, right, kind)),
                _ => None,
            })
            .collect::<Vec<_>>();
        let (left, right, kind) = rows.last().unwrap();
        if change.location.name() == std::path::Path::new("context.txt") {
            assert_eq!(**kind, RowKind::Annotation);
            annotation(left);
            annotation(right);
            let previous = rows[rows.len() - 2];
            assert!(previous.0.to_string().ends_with("stable"));
            assert!(previous.1.to_string().ends_with("stable"));
        } else {
            assert_eq!(**kind, RowKind::Content);
            assert!(right.to_string().ends_with("\\ No newline at end of file"));
        }
    }
}

#[test]
fn newline_markers_follow_wrapped_source_and_stay_visible_during_horizontal_scroll() {
    let f = Fixture::create();
    f.commit("long.txt", &"before ".repeat(40));
    write(f.repo.join("long.txt"), &"after ".repeat(50));
    let snapshot = files::read(&f.repo).unwrap();
    let patch = files::document(&snapshot, &snapshot.changes[0], &Cancel::default()).unwrap();
    let view = prepare("long.txt", patch, Area::Unstaged, &Cancel::default()).unwrap();
    for layout in [Preference::Unified, Preference::Split] {
        for wrap in [false, true] {
            let cache = view.layout(40, layout, wrap, &[], HeaderVisibility::Collapsed);
            let mut markers = 0;
            for (index, row) in cache.rows.iter().enumerate() {
                let (lines, fixed) = match row {
                    VisualRow::Full {
                        line,
                        fixed,
                        kind: RowKind::Annotation,
                        ..
                    } => (vec![line], fixed),
                    VisualRow::Columns {
                        left,
                        right,
                        fixed,
                        kind: RowKind::Annotation,
                        ..
                    } => (vec![left, right], fixed),
                    _ => continue,
                };
                if wrap {
                    assert!(index > cache.locate(row.anchors().preferred()) + 1);
                }
                for line in lines {
                    if line.width() > 0 {
                        markers += 1;
                        let displayed = crate::horizontal::pinned(&line.text, *fixed, 200, 19);
                        assert_eq!(displayed.to_string().trim(), notice::NO_NEWLINE);
                    }
                }
            }
            assert_eq!(markers, 2);
        }
    }
}
