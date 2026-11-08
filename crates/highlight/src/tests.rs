use super::*;
use std::{collections::BTreeSet, num::NonZeroU32, path::Path};
mod coverage;

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
fn light_and_dark_themes_preserve_source_and_distinguish_syntax() {
    let text = "export const message = \"中文\"; // comment\n";
    let requested = numbers(&[1]);
    let mut palettes = Vec::new();
    for theme in [Theme::Dark, Theme::Light] {
        let highlighted =
            highlight_with_theme(Path::new("theme.ts"), text, &requested, theme, || Ok(()))
                .unwrap();
        let line = &lines(&highlighted)[&NonZeroU32::new(1).unwrap()];
        let reconstructed: String = line
            .tokens
            .iter()
            .map(|token| &line.text[token.range.clone()])
            .collect();
        assert_eq!(reconstructed, text);
        let colors = [
            color(line, "export"),
            color(line, "中文"),
            color(line, "comment"),
        ];
        assert_ne!(colors[0], colors[1]);
        assert_ne!(colors[0], colors[2]);
        palettes.push(colors);
    }
    assert_ne!(palettes[0], palettes[1]);
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
        ("app.js", "const x = 1;\n", "JavaScript (Babel)"),
        ("module.mjs", "export const x = 1;\n", "JavaScript (Babel)"),
        ("module.cjs", "module.exports = 1;\n", "JavaScript (Babel)"),
        (
            "view.jsx",
            "const View = () => <button />;\n",
            "JavaScript (Babel)",
        ),
        ("data.json", "{\"x\": 1}\n", "JSON"),
        ("Makefile", "all:\n\techo hi\n", "Makefile"),
        ("Dockerfile", "FROM alpine\n", "Dockerfile"),
        ("config.toml", "enabled = true\n", "TOML"),
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
fn typescript_and_tsx_use_their_own_grammars() {
    for (path, text, expected) in [
        (
            "types.ts",
            "export type Location = \"toolbar\" | \"record\";\n",
            "TypeScript",
        ),
        ("types.mts", "export type Mode = string;\n", "TypeScript"),
        ("types.cts", "export type Mode = string;\n", "TypeScript"),
        (
            "component.tsx",
            "export const View = () => <button title=\"保存\">Save</button>;\n",
            "TypeScriptReact",
        ),
    ] {
        assert!(
            matches!(source(path, text, &[1]), Source::Syntax { language, .. } if language == expected),
            "{path} 未识别为 {expected}"
        );
    }
}

#[test]
fn typescript_keywords_generic_constraints_strings_and_comments_have_distinct_colors() {
    let text = concat!(
        "export type Mode = \"global\" | \"item\"; // available modes\n",
        "export interface Command<T extends string> {\n",
        "  readonly location: Mode;\n",
        "  readonly id: string;\n",
        "}\n",
    );
    let highlighted = source("types.d.ts", text, &[1, 2, 3, 4, 5]);
    let get = |number| &lines(&highlighted)[&NonZeroU32::new(number).unwrap()];
    assert_ne!(color(get(1), "export"), color(get(1), "global"));
    assert_ne!(color(get(1), "export"), color(get(1), "available"));
    assert_ne!(color(get(3), "readonly"), color(get(3), "location"));
    assert_ne!(color(get(2), "extends"), color(get(1), "global"));
    assert_ne!(color(get(4), "string"), color(get(1), "global"));
    for line in lines(&highlighted).values() {
        assert_eq!(
            line.tokens
                .iter()
                .map(|token| &line.text[token.range.clone()])
                .collect::<String>(),
            line.text
        );
    }
}

#[test]
fn tsx_tags_attributes_strings_and_embedded_typescript_keep_their_colors() {
    let text = "export const View = ({ label }: { label: string }) => <button title=\"保存\">{label}</button>; // end\n";
    let highlighted = source("component.tsx", text, &[1]);
    let line = &lines(&highlighted)[&NonZeroU32::new(1).unwrap()];
    assert_ne!(color(line, "export"), color(line, "保存"));
    assert_ne!(color(line, "button"), color(line, "保存"));
    assert_ne!(color(line, "title"), color(line, "保存"));
    assert_ne!(color(line, "end"), color(line, "保存"));
}

#[test]
fn typescript_skipped_context_retains_template_string_and_comment_state() {
    let text = format!(
        "/*\n{}*/\nexport const template = `\n{}end`;\nexport type Mode = \"global\";\n",
        "context\n".repeat(20),
        "template\n".repeat(20)
    );
    let highlighted = source("context.ts", &text, &[15, 40, 45]);
    let get = |number| &lines(&highlighted)[&NonZeroU32::new(number).unwrap()];
    assert_ne!(color(get(15), "context"), color(get(40), "template"));
    assert_ne!(color(get(40), "template"), color(get(45), "export"));
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
