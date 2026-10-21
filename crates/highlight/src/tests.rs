use super::*;
use std::{collections::BTreeSet, num::NonZeroU32, path::Path};

fn numbers(values: &[u32]) -> BTreeSet<NonZeroU32> {
    values
        .iter()
        .map(|value| NonZeroU32::new(*value).unwrap())
        .collect()
}
fn source(path: &str, text: &str, requested: &[u32]) -> Source {
    highlight(Path::new(path), text, &numbers(requested), || Ok(())).unwrap()
}
fn lines(source: &Source) -> &std::collections::BTreeMap<NonZeroU32, Highlighted> {
    let Source::Syntax { lines, .. } = source else {
        panic!("应该识别语法：{source:?}")
    };
    lines
}
fn color(line: &Highlighted, word: &str) -> Color {
    let at = line.text.find(word).unwrap();
    line.tokens
        .iter()
        .find(|token| token.range.contains(&at))
        .unwrap()
        .style
        .foreground
}

#[test]
fn rust_tokens_preserve_unicode_and_distinguish_keywords_strings_and_comments() {
    let text = "fn main() { let 消息 = \"中文\"; } // comment\n";
    let highlighted = source("中文 文件.rs", text, &[1]);
    let line = &lines(&highlighted)[&NonZeroU32::new(1).unwrap()];
    assert_ne!(color(line, "fn"), color(line, "中文"));
    assert_ne!(color(line, "fn"), color(line, "comment"));
    let reconstructed: String = line
        .tokens
        .iter()
        .map(|token| &line.text[token.range.clone()])
        .collect();
    assert_eq!(reconstructed, text);
}

#[test]
fn omitted_hunk_ranges_still_advance_multiline_comment_and_string_states() {
    let mut text = "/* opening outside the diff\n".to_owned();
    text.push_str(&"inside comment\n".repeat(20));
    text.push_str("*/\nfn real() {}\nlet s = r#\"\n");
    text.push_str(&"inside string\n".repeat(20));
    text.push_str("\"#;\nfn last() {}\n");
    let highlighted = source("context.rs", &text, &[15, 23, 40, 46]);
    let lines = lines(&highlighted);
    assert_eq!(lines.len(), 4);
    let get = |n| &lines[&NonZeroU32::new(n).unwrap()];
    assert_ne!(color(get(15), "inside"), color(get(23), "fn"));
    assert_ne!(color(get(15), "inside"), color(get(40), "inside"));
    assert_eq!(color(get(23), "fn"), color(get(46), "fn"));
}

#[test]
fn language_detection_uses_names_extensions_and_shebangs() {
    for (path, text, expected) in [
        ("main.go", "package main\n", "Go"),
        ("main.py", "print('hi')\n", "Python"),
        ("app.js", "const x = 1;\n", "JavaScript"),
        ("data.json", "{\"x\": 1}\n", "JSON"),
        ("Makefile", "all:\n\techo hi\n", "Makefile"),
        ("script", "#!/usr/bin/env python3\nprint('hi')\n", "Python"),
    ] {
        assert!(
            matches!(source(path, text, &[1]), Source::Syntax { language, .. } if language == expected),
            "{path}"
        );
    }
    assert!(matches!(
        source("file.unrecognized_extension", "opaque\n", &[1]),
        Source::UnknownLanguage
    ));
}

#[test]
fn source_line_and_line_count_limits_are_explicit() {
    for (text, expected) in [
        ("x".repeat(1024 * 1024 + 1), Limit::SourceBytes),
        ("x".repeat(16 * 1024 + 1), Limit::LineBytes),
        ("x\n".repeat(50_001), Limit::LineCount),
    ] {
        assert!(
            matches!(source("large.rs", &text, &[1]), Source::Limited(limit) if limit == expected)
        );
    }
}

#[test]
fn cancellation_interrupts_parsing_of_preceding_context() {
    let mut checkpoints = 0;
    let error = highlight(
        Path::new("cancel.rs"),
        &"// context\n".repeat(100),
        &numbers(&[100]),
        || {
            checkpoints += 1;
            anyhow::ensure!(checkpoints < 5, "取消测试");
            Ok(())
        },
    )
    .unwrap_err();
    assert_eq!(error.to_string(), "取消测试");
    assert_eq!(checkpoints, 5);
}
