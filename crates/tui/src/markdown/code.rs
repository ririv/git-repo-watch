//! Code blocks reuse the application's bundled syntax definitions and theme.
use super::*;
use std::{collections::BTreeSet, num::NonZeroU32};

pub(super) fn render(language: &str, source: &str) -> Vec<Line<'static>> {
    let mut lines = vec![Line::styled(
        format!("┌ {language}"),
        Style::default().fg(MUTED),
    )];
    if language.is_empty() || matches!(language, "text" | "plaintext" | "txt") {
        lines.extend(source.lines().map(|line| Line::raw(line.to_owned())));
    } else {
        let requested: BTreeSet<_> = (1..=source.lines().count())
            .filter_map(|i| NonZeroU32::new(i as u32))
            .collect();
        match highlight::highlight_code(language, source, &requested) {
            Ok(highlight::Source::Syntax {
                lines: highlighted, ..
            }) => {
                for line in highlighted.into_values() {
                    let text = line.text.trim_end_matches(['\n', '\r']);
                    let spans = line
                        .tokens
                        .iter()
                        .filter_map(|token| {
                            let start = token.range.start.min(text.len());
                            let end = token.range.end.min(text.len());
                            if start == end {
                                return None;
                            }
                            let highlight::Color(r, g, b) = token.style.foreground;
                            let mut style = Style::default().fg(Color::Rgb(r, g, b));
                            for (enabled, modifier) in [
                                (token.style.bold, Modifier::BOLD),
                                (token.style.italic, Modifier::ITALIC),
                                (token.style.underline, Modifier::UNDERLINED),
                            ] {
                                if enabled {
                                    style = style.add_modifier(modifier);
                                }
                            }
                            Some(Span::styled(text[start..end].to_owned(), style))
                        })
                        .collect::<Vec<_>>();
                    lines.push(Line::from(spans));
                }
            }
            Ok(highlight::Source::UnknownLanguage) => {
                lines.extend(source.lines().map(|line| Line::raw(line.to_owned())))
            }
            Ok(highlight::Source::Limited(limit)) => lines.push(Line::styled(
                format!("Highlight limit: {limit:?}"),
                Style::default().fg(Color::Red),
            )),
            Err(error) => lines.push(Line::styled(
                format!("Highlight error: {error:#}"),
                Style::default().fg(Color::Red),
            )),
        }
    }
    // 代码块可含制表符，但不可向终端发送原始控制序列。
    for line in &mut lines {
        for span in &mut line.spans {
            span.content = span
                .content
                .chars()
                .flat_map(|c| match c {
                    '\t' => "    ".chars().collect::<Vec<_>>(),
                    c if c.is_control() => vec!['�'],
                    c => vec![c],
                })
                .collect::<String>()
                .into();
        }
    }
    lines.push(Line::styled("└", Style::default().fg(MUTED)));
    lines
}
