//! Source previews share bundled syntax definitions and the raster decoder.
mod markdown;
use super::opening::{Opening, Renderer, TextMode};
use engine::{cancel::Cancel, local::browser::Content};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
};
use std::{collections::BTreeSet, num::NonZeroU32};

#[derive(Debug)]
pub(crate) enum Document {
    Text {
        text: Text<'static>,
        width: usize,
        gutter: usize,
    },
    Image(Box<crate::preview::View>),
}
fn document(text: Text<'static>, gutter: usize) -> Document {
    let width = text.width();
    Document::Text {
        text,
        width,
        gutter,
    }
}
fn plain(text: &str) -> String {
    text.chars()
        .flat_map(|c| match c {
            '\t' => "    ".chars().collect::<Vec<_>>(),
            c if c.is_control() => vec!['�'],
            c => vec![c],
        })
        .collect()
}
fn plain_source(text: &str, cancel: &Cancel) -> anyhow::Result<Vec<Line<'static>>> {
    text.lines()
        .enumerate()
        .map(|(i, line)| {
            cancel.check()?;
            Ok(Line::from(vec![
                Span::styled(
                    format!("{:>4}  ", i + 1),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::raw(plain(line)),
            ]))
        })
        .collect()
}
pub(super) fn prepare(
    content: Content,
    opening: Opening,
    width: u16,
    cancel: &Cancel,
) -> anyhow::Result<Document> {
    let mut gutter = 0;
    let text = match content {
        Content::Text { path, text } => {
            if let Opening::Text {
                renderer,
                mode: TextMode::Preview,
            } = opening
            {
                return match renderer {
                    Renderer::Markdown => Ok(document(markdown::prepare(&text, width, cancel)?, 0)),
                    Renderer::Svg => Ok(Document::Image(Box::new(crate::preview::svg::prepare(
                        &path, &text, cancel,
                    )?))),
                };
            }
            gutter = text.lines().count().to_string().len().max(4) + 2;
            let requested: BTreeSet<_> = (1..=text.lines().count())
                .filter_map(|i| NonZeroU32::new(i as u32))
                .collect();
            match highlight::highlight(&path, &text, &requested, || cancel.check())? {
                highlight::Source::Syntax { lines, .. } => {
                    let rows = lines
                        .into_values()
                        .enumerate()
                        .map(|(index, line)| {
                            let end = line.text.trim_end_matches(['\n', '\r']).len();
                            let mut spans = vec![Span::styled(
                                format!("{:>4}  ", index + 1),
                                Style::default().fg(Color::DarkGray),
                            )];
                            for token in line.tokens {
                                let start = token.range.start.min(end);
                                let end = token.range.end.min(end);
                                if start == end {
                                    continue;
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
                                spans.push(Span::styled(plain(&line.text[start..end]), style));
                            }
                            Line::from(spans)
                        })
                        .collect::<Vec<_>>();
                    return Ok(document(Text::from(rows), gutter));
                }
                highlight::Source::UnknownLanguage => Text::from(plain_source(&text, cancel)?),
                highlight::Source::Limited(limit) => {
                    let reason = match limit {
                        highlight::Limit::SourceBytes => i18n::tr!(SyntaxSourceLimit),
                        highlight::Limit::LineBytes => i18n::tr!(SyntaxLineLimit),
                        highlight::Limit::LineCount => i18n::tr!(SyntaxLineCountLimit),
                    };
                    let mut lines = vec![Line::styled(reason, Style::default().fg(Color::Yellow))];
                    lines.extend(plain_source(&text, cancel)?);
                    Text::from(lines)
                }
            }
        }
        Content::Image(image) => {
            return Ok(Document::Image(Box::new(crate::preview::single(
                image, cancel,
            )?)));
        }
        Content::Symlink(target) => Text::from(format!(
            "{} → {}",
            i18n::tr!(BrowserSymlink),
            crate::view::safe(&target.to_string_lossy())
        )),
        Content::Missing => Text::from(i18n::tr!(BrowserMissing)),
        Content::Binary => Text::from(i18n::tr!(BrowserBinary)),
        Content::TooLarge => Text::from(i18n::tr!(BrowserTooLarge)),
        Content::Special => Text::from(i18n::tr!(BrowserSpecialFile)),
        Content::Directory => Text::from(i18n::tr!(BrowserDirectory)),
    };
    Ok(document(text, gutter))
}
