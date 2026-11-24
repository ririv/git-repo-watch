use super::*;
use engine::local::files::{self, Area, patch::Source};
use ratatui::text::Text;
use std::path::Path;
use test_support::*;

fn styled(title: &str, document: Document, cancel: &Cancel) -> Result<Text<'static>> {
    let view = prepare(title, document, Area::Unstaged, cancel)?;
    Ok(styled_rows(&view))
}
fn styled_rows(view: &View) -> Text<'static> {
    let cache = layout::arrange(&view.rows, view.digits, 200, Layout::Unified, false, &[]);
    Text::from(
        cache
            .rows
            .into_iter()
            .map(|row| match row {
                VisualRow::Full { line, .. } => line.text,
                VisualRow::Columns { .. } => unreachable!(),
            })
            .collect::<Vec<_>>(),
    )
}

fn document(f: &Fixture, name: &str, area: Area) -> Document {
    let snapshot = files::read(&f.repo).unwrap();
    let change = snapshot
        .changes
        .iter()
        .find(|change| change.location.name() == Path::new(name) && change.area == area)
        .unwrap_or_else(|| panic!("找不到 {name:?} {area:?}: {:?}", snapshot.changes));
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
    assert!(new.to_string().contains("21 +│"));
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
    assert!(rendered.contains("+│    let 消息 = \"中文\";"));
    assert!(rendered.contains(notice::NO_NEWLINE));
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
        let view = prepare(
            "unknown",
            document(&f, "file.unknown_extension", Area::Untracked),
            Area::Untracked,
            &Cancel::default(),
        )
        .unwrap();
        assert_eq!(
            view.information.as_ref().unwrap().syntax,
            [i18n::tr!(SyntaxUnknown)]
        );
        assert_eq!(
            find(&styled_rows(&view), "unknown content").style.bg,
            Some(ADDED)
        );
        let view = prepare(
            "long",
            document(&f, "long.rs", Area::Untracked),
            Area::Untracked,
            &Cancel::default(),
        )
        .unwrap();
        assert_eq!(
            view.information.as_ref().unwrap().syntax,
            [i18n::tr!(SyntaxLineLimit)]
        );
    }
}

#[test]
fn file_information_is_separate_from_patch_rows_and_the_complete_export() {
    let f = Fixture::create();
    f.commit("中文 文件.rs", "fn before() {}\n");
    write(f.repo.join("中文 文件.rs"), "fn after() {}\n");
    let patch = document(&f, "中文 文件.rs", Area::Unstaged);
    let raw = patch.text();
    let view = prepare("中文 文件.rs", patch, Area::Unstaged, &Cancel::default()).unwrap();
    let information = view.information.as_ref().unwrap();
    assert_eq!(information.title, "中文 文件.rs");
    assert_eq!(information.syntax, ["Rust"]);
    let rows = styled_rows(&view);
    assert!(rows.lines[0].to_string().starts_with("@@"));
    assert!(!rows.to_string().contains("diff --git "));
    assert!(!rows.to_string().contains(i18n::tr!(SyntaxLabel)));
    assert!(rows.to_string().contains("@@"));
    assert!(matches!(&view.export, Export::Complete(export) if export.as_ref() == raw));
}

#[test]
fn binary_notices_use_readable_paths_and_keep_original_exports_for_each_change_kind() {
    let f = Fixture::create();
    std::fs::create_dir(f.repo.join("元信息")).unwrap();
    #[cfg(unix)]
    let modified = r#"元信息/中文 空格 "引号" \345.dat"#;
    #[cfg(windows)]
    let modified = "元信息/中文 空格 '引号' [345].dat";
    let deleted = "元信息/删除.dat";
    let added = "元信息/新增.dat";
    f.commit(modified, "before\0bytes");
    f.commit(deleted, "\0removed-one\n\0removed-two\n");
    write(f.repo.join(modified), "after\0bytes");
    std::fs::remove_file(f.repo.join(deleted)).unwrap();
    write(f.repo.join(added), "\0new-alpha\n\0new-beta\n\0new-gamma\n");
    git(&f.repo, &["add", "-A"]);
    for (name, expected) in [
        (
            modified,
            format!("Binary files a/{modified} and b/{modified} differ"),
        ),
        (
            deleted,
            format!("Binary files a/{deleted} and /dev/null differ"),
        ),
        (
            added,
            format!("Binary files /dev/null and b/{added} differ"),
        ),
    ] {
        let patch = document(&f, name, Area::Staged);
        let raw = patch.text();
        assert!(raw.contains(r#"\345"#) || raw.contains(r#"\344"#));
        let mut view = prepare(name, patch, Area::Staged, &Cancel::default()).unwrap();
        assert!(styled_rows(&view).to_string().contains(&expected));
        view.set_header(HeaderVisibility::Expanded, &mut 0);
        assert!(styled_rows(&view).to_string().contains(&expected));
        assert!(matches!(&view.export, Export::Complete(export) if export.as_ref() == raw));
    }
}

#[cfg(unix)]
#[test]
fn binary_path_control_characters_cannot_create_extra_rows_or_terminal_commands() {
    let f = Fixture::create();
    let name = "中文\n\t\u{1b}[31m路径.dat";
    f.commit(name, "before\0bytes");
    write(f.repo.join(name), "after\0bytes");
    let patch = document(&f, name, Area::Unstaged);
    let view = prepare(name, patch, Area::Unstaged, &Cancel::default()).unwrap();
    let rows = styled_rows(&view);
    let notice = find(&rows, "Binary files");
    assert_eq!(
        notice.to_string(),
        "Binary files a/中文 路径.dat and b/中文 路径.dat differ"
    );
    assert!(!notice.to_string().contains(['\n', '\t', '\u{1b}']));
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
