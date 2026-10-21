use super::*;

#[test]
fn common_languages_produce_distinct_code_and_string_colors() {
    for (path, text, expected, code, string) in [
        (
            "main.c",
            "const char *message = \"hello\"; // note\n",
            "C",
            "const",
            "hello",
        ),
        (
            "main.cpp",
            "constexpr auto message = \"hello\"; // note\n",
            "C++",
            "constexpr",
            "hello",
        ),
        (
            "Main.java",
            "public class Main { String message = \"hello\"; } // note\n",
            "Java",
            "public",
            "hello",
        ),
        (
            "main.cs",
            "public class Main { string message = \"hello\"; } // note\n",
            "C#",
            "public",
            "hello",
        ),
        (
            "main.go",
            "package main\nconst message = \"hello\" // note\n",
            "Go",
            "const",
            "hello",
        ),
        (
            "main.py",
            "def main(): return \"hello\" # note\n",
            "Python",
            "def",
            "hello",
        ),
        (
            "main.kt",
            "val message: String = \"hello\" // note\n",
            "Kotlin",
            "val",
            "hello",
        ),
        (
            "main.swift",
            "let message: String = \"hello\" // note\n",
            "Swift",
            "let",
            "hello",
        ),
        (
            "main.dart",
            "final message = \"hello\"; // note\n",
            "Dart",
            "final",
            "hello",
        ),
        (
            "main.rb",
            "def main; return \"hello\"; end # note\n",
            "Ruby",
            "def",
            "hello",
        ),
        (
            "main.php",
            "<?php function main() { return \"hello\"; } // note\n",
            "PHP",
            "function",
            "hello",
        ),
        (
            "main.scala",
            "val message = \"hello\" // note\n",
            "Scala",
            "val",
            "hello",
        ),
        (
            "main.lua",
            "local message = \"hello\" -- note\n",
            "Lua",
            "local",
            "hello",
        ),
        (
            "main.r",
            "if (TRUE) message <- \"hello\" # note\n",
            "R",
            "if",
            "hello",
        ),
        (
            "query.sql",
            "SELECT 'hello' AS message; -- note\n",
            "SQL",
            "SELECT",
            "hello",
        ),
        (
            "main.ps1",
            "function Main { return \"hello\" } # note\n",
            "PowerShell",
            "function",
            "hello",
        ),
        (
            "main.sh",
            "if true; then echo \"hello\"; fi # note\n",
            "Bourne Again Shell (bash)",
            "if",
            "hello",
        ),
        (
            "main.fish",
            "if true; echo \"hello\"; end # note\n",
            "Fish",
            "if",
            "hello",
        ),
        (
            "main.ex",
            "def main, do: \"hello\" # note\n",
            "Elixir",
            "def",
            "hello",
        ),
        (
            "main.zig",
            "const message: []const u8 = \"hello\"; // note\n",
            "Zig",
            "const",
            "hello",
        ),
        (
            "index.html",
            "<button title=\"hello\">Save</button>\n",
            "HTML",
            "button",
            "hello",
        ),
        (
            "style.css",
            "button { content: \"hello\"; }\n",
            "CSS",
            "content",
            "hello",
        ),
        (
            "style.scss",
            "$text: \"hello\"; button { content: $text; }\n",
            "SCSS",
            "$text",
            "hello",
        ),
        (
            "main.proto",
            "syntax = \"proto3\";\n",
            "Protocol Buffer",
            "syntax",
            "proto3",
        ),
        (
            "query.graphql",
            "query { user(id: \"hello\") { name } }\n",
            "GraphQL",
            "query",
            "hello",
        ),
        (
            "config.yaml",
            "enabled: true\nmessage: \"hello\"\n",
            "YAML",
            "true",
            "hello",
        ),
        (
            "config.toml",
            "enabled = true\nmessage = \"hello\"\n",
            "TOML",
            "true",
            "hello",
        ),
        (
            "config.json",
            "{\"message\": \"hello\", \"enabled\": true}\n",
            "JSON",
            "true",
            "hello",
        ),
        (
            "flake.nix",
            "{ message = \"hello\"; enabled = true; }\n",
            "Nix",
            "true",
            "hello",
        ),
        (
            "main.tf",
            "variable \"hello\" { default = true }\n",
            "Terraform",
            "variable",
            "hello",
        ),
    ] {
        let highlighted = source(path, text, &[1, 2]);
        let Source::Syntax { language, lines } = &highlighted else {
            panic!("{path}: {highlighted:?}")
        };
        assert_eq!(language, expected, "{path}");
        let find_color = |word| {
            color(
                lines
                    .values()
                    .find(|line| line.text.contains(word))
                    .unwrap(),
                word,
            )
        };
        assert_ne!(find_color(code), find_color(string), "{path}: {text}");
        for line in lines.values() {
            assert_eq!(
                line.tokens
                    .iter()
                    .map(|token| &line.text[token.range.clone()])
                    .collect::<String>(),
                line.text,
                "{path}"
            );
        }
    }
}

#[test]
fn configuration_names_and_longest_compound_suffixes_take_precedence() {
    for (path, expected) in [
        ("Dockerfile.dev", "Dockerfile"),
        ("Containerfile", "Dockerfile"),
        ("Containerfile.production", "Dockerfile"),
        ("service.Dockerfile", "Dockerfile"),
        ("tsconfig.json", "JSONC"),
        ("tsconfig.build.json", "JSONC"),
        ("jsconfig.json", "JSONC"),
        ("project.code-workspace", "JSONC"),
        (".babelrc", "JSON5"),
        (".env.local", "DotENV"),
        ("CMakeLists.txt", "CMake"),
        ("config.h.in", "CMake C Header"),
        ("config.hpp.in", "CMake C++ Header"),
        ("page.html.j2", "HTML (Jinja2)"),
        ("page.html.eex", "HTML (EEx)"),
        ("message.pb.txt", "Protocol Buffer (TEXT)"),
        ("style.css.erb", "CSS"),
        ("go.mod", "Gomod"),
        ("go.work", "Gomod"),
        ("go.sum", "Gosum"),
        ("Cargo.lock", "TOML"),
        ("uv.lock", "TOML"),
        (".gitignore", "Git Ignore"),
        (".editorconfig", "INI"),
    ] {
        let syntax = syntax::detect(syntax::set(), Path::new(path), None)
            .unwrap()
            .unwrap();
        assert_eq!(syntax.name, expected, "{path}");
    }
    for path in [
        "Dockerfile.",
        "Dockerfiles.dev",
        "tsconfiguration.jsonc.unknown",
    ] {
        assert!(
            syntax::detect(syntax::set(), Path::new(path), None)
                .unwrap()
                .is_none(),
            "{path}"
        );
    }
}

#[test]
fn jsonc_comments_strings_constants_and_continuing_context_are_colored() {
    let text = "{\n/* comment starts\nstill inside comment\n*/\n\"message\": \"hello\",\n\"enabled\": true, // note\n}\n";
    for path in [
        "config.jsonc",
        "tsconfig.build.json",
        "project.code-workspace",
    ] {
        let highlighted = source(path, text, &[3, 5, 6]);
        let get = |number| &lines(&highlighted)[&NonZeroU32::new(number).unwrap()];
        assert_ne!(color(get(3), "inside"), color(get(5), "hello"));
        assert_ne!(color(get(6), "true"), color(get(5), "hello"));
        assert_eq!(color(get(3), "inside"), color(get(6), "note"));
    }
}

#[test]
fn json5_handles_unicode_keys_numbers_escapes_comments_and_continued_strings() {
    let text = concat!(
        "{ 中文: 'hello', plus: +1, hex: -0xAF, leading: .5, trailing: 5., exp: 1.e3, special: -Infinity,\n",
        "  escaped: '\\'\\x41\\u4E2D\\v\\0', // note\n",
        "  continued: 'line\\\n",
        "still inside string',\n",
        "/* omitted block\n",
        "comment continues\n",
        "*/ enabled: true, }\n",
    );
    let highlighted = source("config.json5", text, &[1, 2, 4, 6, 7]);
    let get = |number| &lines(&highlighted)[&NonZeroU32::new(number).unwrap()];
    for number in ["+1", "-0xAF", ".5", "5.", "1.e3", "-Infinity"] {
        assert_ne!(color(get(1), number), color(get(1), "hello"), "{number}");
    }
    assert_eq!(color(get(4), "inside"), color(get(1), "hello"));
    assert_ne!(color(get(6), "comment"), color(get(7), "true"));
    assert_ne!(color(get(2), "note"), color(get(1), "hello"));
    let set = syntax::set();
    let mut parser = syntect::parsing::ParseState::new(set.find_syntax_by_name("JSON5").unwrap());
    let mut stack = syntect::parsing::ScopeStack::new();
    for line in syntect::util::LinesWithEndings::from(text) {
        for (_, operation) in parser.parse_line(line, set).unwrap() {
            stack.apply(&operation).unwrap();
            assert!(
                !stack
                    .as_slice()
                    .iter()
                    .any(|scope| scope.to_string().starts_with("invalid.")),
                "不应把合法 JSON5 标成错误：{line}"
            );
        }
    }
}

#[test]
fn component_grammars_color_embedded_typescript_tags_strings_and_styles() {
    for (path, expected, text, script_line, tag_line, style_line) in [
        (
            "App.vue",
            "Vue Component",
            "<script lang=\"ts\">\nconst message: string = \"hello\"; // note\n</script>\n<template><button title=\"save\">{{ message }}</button></template>\n<style>\nbutton { content: \"styled\"; }\n</style>\n",
            2,
            4,
            6,
        ),
        (
            "App.svelte",
            "Svelte",
            "<script lang=\"ts\">\nconst message: string = \"hello\"; // note\n</script>\n<button title=\"save\">{message}</button>\n<style>\nbutton { content: \"styled\"; }\n</style>\n",
            2,
            4,
            6,
        ),
        (
            "Page.astro",
            "Astro",
            "---\nconst message: string = \"hello\"; // note\n---\n<button title=\"save\">{message}</button>\n<style>\nbutton { content: \"styled\"; }\n</style>\n",
            2,
            4,
            6,
        ),
    ] {
        let highlighted = source(path, text, &[script_line, tag_line, style_line]);
        let Source::Syntax { language, .. } = &highlighted else {
            panic!("{path}: {highlighted:?}")
        };
        assert_eq!(language, expected);
        let get = |number| &lines(&highlighted)[&NonZeroU32::new(number).unwrap()];
        assert_ne!(
            color(get(script_line), "const"),
            color(get(script_line), "hello"),
            "{path} script"
        );
        assert_ne!(
            color(get(script_line), "note"),
            color(get(script_line), "hello"),
            "{path} comment"
        );
        assert_ne!(
            color(get(tag_line), "button"),
            color(get(tag_line), "save"),
            "{path} tag"
        );
        assert_ne!(
            color(get(style_line), "content"),
            color(get(style_line), "styled"),
            "{path} style"
        );
    }
}

#[test]
fn astro_interpolations_keep_nested_objects_and_strings_before_returning_to_markup() {
    let text = "{JSON.stringify({ inner: { message: \"brace } stays in string\" } })}<button title=\"save\">Done</button>\n";
    let highlighted = source("Page.astro", text, &[1]);
    let line = &lines(&highlighted)[&NonZeroU32::new(1).unwrap()];
    assert_eq!(color(line, "brace"), color(line, "stays"));
    assert_eq!(color(line, "brace"), color(line, "save"));
    assert_ne!(color(line, "button"), color(line, "save"));
}

#[test]
fn astro_frontmatter_is_only_at_the_start_and_closes_after_nested_typescript() {
    for text in [
        "---\nconst data = { nested: { message: \"hello\" } };\n/* note\ncontinues */\n---\n---\n<button title=\"save\">Done</button>\n",
        "Text before separator\n---\n<button title=\"save\">Done</button>\n",
    ] {
        let number = text.lines().count() as u32;
        let highlighted = source("Page.astro", text, &[number]);
        let line = &lines(&highlighted)[&NonZeroU32::new(number).unwrap()];
        assert_ne!(color(line, "button"), color(line, "save"));
    }
}

#[test]
fn astro_style_languages_link_to_the_bundled_definitions() {
    for (language, body, code, string) in [
        (
            "scss",
            "$text: \"styled\"; button { content: $text; }",
            "$text",
            "styled",
        ),
        (
            "less",
            "@text: \"styled\"; button { content: @text; }",
            "@text",
            "styled",
        ),
        (
            "stylus",
            "button\n  content \"styled\"",
            "content",
            "styled",
        ),
    ] {
        let text = format!("<style lang=\"{language}\">\n{body}\n</style>\n");
        let highlighted = source("Page.astro", &text, &[2, 3]);
        let line = lines(&highlighted)
            .values()
            .find(|line| line.text.contains(string))
            .unwrap();
        assert_ne!(color(line, code), color(line, string), "{language}");
    }
}
