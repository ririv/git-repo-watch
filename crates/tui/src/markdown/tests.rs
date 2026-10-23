use super::*;

fn plain(text: &Text<'_>) -> String {
    text.lines
        .iter()
        .map(|line| {
            line.spans
                .iter()
                .map(|span| span.content.as_ref())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn parses_escaped_markup_nested_styles_links_and_literal_code() {
    let text = render(
        "## 标题\n\n**粗体 *斜体***，\\*原文\\*，``a ` b``，[文档](https://example.com)。",
        80,
    );
    let output = plain(&text);
    assert!(output.contains("标题"));
    assert!(output.contains("*原文*"));
    assert!(output.contains("a ` b"));
    assert!(output.contains("文档 (https://example.com)"));
    let span = text
        .lines
        .iter()
        .flat_map(|line| &line.spans)
        .find(|span| span.content == "斜体")
        .unwrap();
    assert!(
        span.style
            .add_modifier
            .contains(Modifier::BOLD | Modifier::ITALIC)
    );
}

#[test]
fn ordered_nested_and_task_lists_keep_markers_and_hard_breaks() {
    let output = plain(&render(
        "3. first\n   - child\n   - next\n4. second\n\n- [x] done\n- [ ] todo\n\n> quote  \n> next",
        60,
    ));
    assert!(output.contains("3. first"), "{output}");
    assert!(output.contains("   • child"), "{output}");
    assert!(output.contains("4. second"), "{output}");
    assert!(output.contains("☑ done"));
    assert!(output.contains("☐ todo"));
    assert!(output.contains("│ next"));
}

#[test]
fn tables_fit_cjk_cells_and_keep_content_on_narrow_screens() {
    let source = "| 名称 | 说明 |\n|:---|---:|\n| **仓库** | 中文测试 abcdefghijklmnopqrstuvwxyz |";
    for width in [16, 22, 40, 80] {
        let text = render(source, width);
        let output = plain(&text);
        assert!(output.contains("名称"));
        assert!(output.contains("仓库"));
        if width >= 11 {
            assert!(
                text.lines
                    .iter()
                    .all(|line| line.width() <= usize::from(width)),
                "{width}: {output}"
            );
        }
        let condensed = output
            .chars()
            .filter(|c| c.is_ascii_alphabetic())
            .collect::<String>();
        assert_eq!(condensed, "abcdefghijklmnopqrstuvwxyz");
    }
    let text = render(source, 5);
    assert!(plain(&text).contains("名称: 仓库"));
}

#[test]
fn fenced_typescript_reuses_highlighting_and_keeps_backticks() {
    let text = render("```typescript\nconst message: string = `你好`;\n```", 80);
    assert!(plain(&text).contains("`你好`"));
    let colors = text
        .lines
        .iter()
        .flat_map(|line| &line.spans)
        .filter_map(|span| {
            if let Some(Color::Rgb(r, g, b)) = span.style.fg {
                Some((r, g, b))
            } else {
                None
            }
        })
        .collect::<std::collections::BTreeSet<_>>();
    assert!(colors.len() > 1, "{text:?}");
}

#[test]
fn text_and_code_cannot_emit_terminal_controls() {
    let text = render("a\u{1b}[31m\n\n```text\nx\u{1b}[31m\n```", 80);
    assert!(!plain(&text).contains('\u{1b}'));
}
