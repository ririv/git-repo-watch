use super::*;
use engine::local::files::{self, Area, patch::Source};
use ratatui::text::Text;
use std::path::Path;
use test_support::*;

fn styled(title: &str, document: Document, cancel: &Cancel) -> Result<Text<'static>> {
    let view = prepare(title, document, Area::Unstaged, cancel)?;
    let cache = layout::arrange(&view.rows, view.digits, 200, Layout::Unified);
    Ok(Text::from(
        cache
            .rows
            .into_iter()
            .map(|row| match row {
                VisualRow::Full { line, .. } => line,
                VisualRow::Columns { .. } => unreachable!(),
            })
            .collect::<Vec<_>>(),
    ))
}

fn document(f: &Fixture, name: &str, area: Area) -> Document {
    let snapshot = files::read(&f.repo).unwrap();
    let change = snapshot
        .changes
        .iter()
        .find(|change| change.location.name() == Path::new(name) && change.area == area)
        .unwrap();
    files::document(&snapshot, change, &Cancel::default()).unwrap()
}
fn find<'a>(text: &'a Text<'_>, word: &str) -> &'a Line<'a> {
    text.lines
        .iter()
        .find(|line| line.to_string().contains(word))
        .unwrap_or_else(|| panic!("没有找到 {word:?}：{text}"))
}
fn foreground(line: &Line<'_>, word: &str) -> Option<Color> {
    line.spans
        .iter()
        .find(|span| span.content.contains(word))
        .unwrap()
        .style
        .fg
}

#[test]
fn old_and_new_contexts_have_independent_syntax_and_diff_backgrounds() {
    let f = Fixture::create();
    let before = format!(
        "/*\n{}fn old() {{}}\n*/\nfn stable() {{}}\n",
        "// context\n".repeat(20)
    );
    let after = format!(
        "{}fn new() {{}}\nfn stable() {{}}\n",
        "// context\n".repeat(20)
    );
    f.commit("syntax.rs", &before);
    write(f.repo.join("syntax.rs"), &after);
    let text = styled(
        "syntax.rs",
        document(&f, "syntax.rs", Area::Unstaged),
        &Cancel::default(),
    )
    .unwrap();
    let old = find(&text, "fn old");
    let new = find(&text, "fn new");
    assert_eq!(old.style.bg, Some(REMOVED));
    assert_eq!(new.style.bg, Some(ADDED));
    assert_ne!(foreground(old, "fn"), foreground(new, "fn"));
    assert_eq!(
        foreground(new, "fn"),
        foreground(find(&text, "fn stable"), "fn")
    );
    assert!(new.to_string().contains("21 │+"));
    assert!(find(&text, "@@").style.fg.is_some());
}

#[test]
fn crlf_tabs_unicode_and_no_final_newline_keep_the_displayed_content() {
    let f = Fixture::create();
    f.commit("中文 文件.rs", "fn old() {}\r\n");
    write(
        f.repo.join("中文 文件.rs"),
        "fn main() {\r\n\tlet 消息 = \"中文\";\r\n}",
    );
    let text = styled(
        "中文 文件.rs",
        document(&f, "中文 文件.rs", Area::Unstaged),
        &Cancel::default(),
    )
    .unwrap();
    let rendered = text.to_string();
    assert!(rendered.contains("+    let 消息 = \"中文\";"));
    assert!(rendered.contains("No newline at end of file"));
    assert!(!rendered.contains(['\r', '\t', '\u{1b}']));
    assert_ne!(
        foreground(find(&text, "let 消息"), "let"),
        foreground(find(&text, "let 消息"), "中文")
    );
}

#[test]
fn unknown_and_bounded_syntax_modes_are_localized_and_keep_diff_styles() {
    let f = Fixture::create();
    write(f.repo.join("file.unknown_extension"), "unknown content\n");
    write(
        f.repo.join("long.rs"),
        &format!("{}\n", "x".repeat(16 * 1024 + 1)),
    );
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let text = styled(
            "unknown",
            document(&f, "file.unknown_extension", Area::Untracked),
            &Cancel::default(),
        )
        .unwrap();
        assert!(text.to_string().contains(i18n::tr!(SyntaxUnknown)));
        assert_eq!(find(&text, "unknown content").style.bg, Some(ADDED));
        let text = styled(
            "long",
            document(&f, "long.rs", Area::Untracked),
            &Cancel::default(),
        )
        .unwrap();
        assert!(text.to_string().contains(i18n::tr!(SyntaxLineLimit)));
    }
}

#[test]
fn external_changes_between_patch_and_source_are_reported_before_coloring() {
    let f = Fixture::create();
    write(f.repo.join("race.rs"), "fn old() {}\n");
    let mut patch = document(&f, "race.rs", Area::Untracked);
    let Document::Patch { files, .. } = &mut patch else {
        panic!("应该生成 patch")
    };
    files[0].after = Source::Text {
        path: "race.rs".into(),
        text: "fn externally_changed() {}\n".into(),
    };
    let error = styled("race.rs", patch, &Cancel::default()).unwrap_err();
    assert_eq!(error.to_string(), i18n::tr!(DiffSourceChanged));
}

#[test]
fn terminal_control_sequences_are_never_executed_by_styled_diff_spans() {
    let f = Fixture::create();
    write(
        f.repo.join("control.rs"),
        "fn main() { let s = \"\u{1b}[31m中文\u{1b}[0m\"; }\u{7}\n",
    );
    let text = styled(
        "control",
        document(&f, "control.rs", Area::Untracked),
        &Cancel::default(),
    )
    .unwrap();
    assert!(!text.to_string().contains(['\u{1b}', '\u{7}']));
    assert!(text.to_string().contains("中文"));
}

#[test]
fn cancelled_styling_and_truncated_patches_are_explicit() {
    let f = Fixture::create();
    write(f.repo.join("many.rs"), &"fn repeated() {}\n".repeat(10_000));
    let patch = document(&f, "many.rs", Area::Untracked);
    let text = styled("many.rs", patch, &Cancel::default()).unwrap();
    assert!(
        text.to_string()
            .contains(i18n::tr!(DiffExceeds128KibOnlyTheBeginning).trim())
    );
    let cancel = Cancel::default();
    cancel.cancel();
    assert!(styled("cancel", Document::Notice("message".into()), &cancel).is_err());
}
