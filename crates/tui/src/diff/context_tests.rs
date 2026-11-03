use super::*;
use engine::local::files::{
    self,
    patch::{FilePatch, Source, SourceOmission},
};
use test_support::{Fixture, write};

fn read(f: &Fixture) -> Box<View> {
    let snapshot = files::read(&f.repo).unwrap();
    let change = &snapshot.changes[0];
    let patch = files::document(&snapshot, change, &Cancel::default()).unwrap();
    prepare("source.txt", patch, change.area, &Cancel::default()).unwrap()
}
fn sources(rows: &[Row]) -> (String, String) {
    fn add(text: &mut String, code: &Code) {
        text.push_str(&code.content.to_string());
        if code.ending == Ending::Newline {
            text.push('\n');
        }
    }
    let (mut before, mut after) = (String::new(), String::new());
    for row in rows {
        match row {
            Row::Context { old, new } => {
                add(&mut before, old);
                add(&mut after, new);
            }
            Row::Removed(code) => add(&mut before, code),
            Row::Added(code) => add(&mut after, code),
            _ => {}
        }
    }
    (before, after)
}

#[test]
fn full_context_reconstructs_both_sources_without_changing_patch_or_editable_lines() {
    let before = (1..=120)
        .map(|n| format!("源码 {n}\r\n"))
        .collect::<String>();
    let after = before
        .replace("源码 20\r\n", "新增 A\r\n新增 B\r\n")
        .replace("源码 70\r\n", "")
        .replace("源码 100", "修改 100");
    for (before, after) in [
        (Some(before.as_str()), after.as_str()),
        (None, "新增\n无换行"),
        (Some("旧版\n无换行"), ""),
        (Some("单行"), "替换"),
    ] {
        let f = Fixture::create();
        if let Some(before) = before {
            f.commit("source.txt", before);
        }
        if after.is_empty() {
            std::fs::remove_file(f.repo.join("source.txt")).unwrap();
        } else {
            write(f.repo.join("source.txt"), after);
        }
        let view = read(&f);
        let full = view.full.as_ref().unwrap();
        assert_eq!(
            sources(full),
            (
                before.unwrap_or_default().replace("\r\n", "\n"),
                after.replace("\r\n", "\n")
            )
        );
        let edits = |rows: &[Row]| rows.iter().filter_map(Row::line_id).collect::<Vec<_>>();
        assert_eq!(edits(full), edits(&view.rows));
        let Export::Complete(raw) = &view.export else {
            panic!("完整补丁")
        };
        if before.is_some_and(|s| s.lines().count() == 120) {
            assert!(!raw.contains("源码 1\r\n"));
            assert!(!sources(&view.rows).1.contains("源码 1\n"));
            assert!(full.iter().all(
                |row| !matches!(row, Row::Metadata(line) if line.to_string().starts_with("@@"))
            ));
        }
        for preference in [Preference::Unified, Preference::Split] {
            let cache = view.layout(
                180,
                preference,
                false,
                &[],
                HeaderVisibility::Collapsed,
                Context::Full,
            );
            let ids = cache.ids.iter().flatten().copied().collect::<Vec<_>>();
            assert_eq!(ids, edits(full));
        }
    }
}

#[test]
fn context_switch_and_header_toggle_retain_the_same_source_location() {
    let f = Fixture::create();
    let before = (1..=120).map(|n| format!("line {n}\n")).collect::<String>();
    f.commit("source.txt", &before);
    write(
        f.repo.join("source.txt"),
        &before.replace("line 100\n", "changed 100\n"),
    );
    for preference in [Preference::Unified, Preference::Split] {
        for wrap in [false, true] {
            let mut view = read(&f);
            let mut scroll = 0;
            let cache = view.arrange(160, preference, wrap, &[], &mut scroll, Context::Full);
            scroll = cache
                .rows
                .iter()
                .position(|row| match row {
                    VisualRow::Full { line, .. } => line.to_string().contains("changed 100"),
                    VisualRow::Columns { right, .. } => right.to_string().contains("changed 100"),
                })
                .unwrap();
            let position = view.position(scroll).unwrap();
            for context in [
                Context::Changes,
                Context::Full,
                Context::Changes,
                Context::Full,
            ] {
                let cache = view.arrange(160, preference, wrap, &[], &mut scroll, context);
                assert_eq!(scroll, cache.locate_position(&position).unwrap());
            }
            let old_scroll = scroll;
            view.set_header(HeaderVisibility::Expanded, &mut scroll);
            view.arrange(160, preference, wrap, &[], &mut scroll, Context::Full);
            view.set_header(HeaderVisibility::Collapsed, &mut scroll);
            assert_eq!(scroll, old_scroll);
        }
    }
}

#[test]
fn full_context_reports_explicit_limits_and_preserves_non_text_notices() {
    let _language = i18n::using(i18n::Language::Chinese);
    for (source, extent, expected) in [
        (
            Source::Unavailable(SourceOmission::TooLarge),
            Extent::Complete,
            i18n::tr!(DiffFullUnavailable),
        ),
        (
            Source::Text {
                path: "large.txt".into(),
                text: "\n".repeat(50_001),
            },
            Extent::Complete,
            i18n::tr!(DiffFullLineLimit),
        ),
        (
            Source::Text {
                path: "truncated.txt".into(),
                text: "text\n".into(),
            },
            Extent::Truncated,
            i18n::tr!(DiffFullTruncated),
        ),
    ] {
        let view = prepare(
            "large",
            Document::Patch {
                files: vec![FilePatch {
                    before: Source::Absent,
                    after: source,
                    lines: vec![],
                }],
                extent,
            },
            Area::Unstaged,
            &Cancel::default(),
        )
        .unwrap();
        let cache = view.layout(
            180,
            Preference::Unified,
            false,
            &[],
            HeaderVisibility::Collapsed,
            Context::Full,
        );
        assert!(
            matches!(&cache.rows[0], VisualRow::Full { line, .. } if line.to_string() == expected)
        );
    }
    let view = prepare(
        "binary",
        Document::Notice("Binary files differ".into()),
        Area::Staged,
        &Cancel::default(),
    )
    .unwrap();
    assert_eq!(view.full.as_ref().unwrap().len(), view.rows.len());
}
