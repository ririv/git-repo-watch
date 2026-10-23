//! Parse old and new sources independently, retaining tokens only for requested lines.
use crate::{Color, Style, THEMES, syntax};
use anyhow::{Context, Result};
use std::{
    collections::{BTreeMap, BTreeSet},
    num::NonZeroU32,
    ops::Range,
    path::Path,
};
use syntect::{easy::HighlightLines, highlighting::FontStyle, util::LinesWithEndings};

pub const MAX_SOURCE_BYTES: usize = 1024 * 1024;
const MAX_LINE_BYTES: usize = 16 * 1024;
const MAX_LINES: usize = 50_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Limit {
    SourceBytes,
    LineBytes,
    LineCount,
}

#[derive(Debug)]
pub enum Source {
    Syntax {
        language: String,
        lines: BTreeMap<NonZeroU32, Highlighted>,
    },
    UnknownLanguage,
    Limited(Limit),
}

#[derive(Debug, Clone)]
pub struct Token {
    pub range: Range<usize>,
    pub style: Style,
}

#[derive(Debug)]
pub struct Highlighted {
    pub text: String,
    pub tokens: Vec<Token>,
}

/// Highlight selected one-based lines after parsing all preceding source lines.
pub fn highlight(
    path: &Path,
    text: &str,
    requested: &BTreeSet<NonZeroU32>,
    mut checkpoint: impl FnMut() -> Result<()>,
) -> Result<Source> {
    checkpoint()?;
    if let Some(limit) = limit(text) {
        return Ok(Source::Limited(limit));
    }
    let syntaxes = syntax::set();
    let Some(syntax) = syntax::detect(syntaxes, path, text.lines().next())? else {
        return Ok(Source::UnknownLanguage);
    };
    highlight_syntax(syntax, text, requested, checkpoint)
}

/// Highlight a fenced code block using its explicit language token.
pub fn highlight_code(
    language: &str,
    text: &str,
    requested: &BTreeSet<NonZeroU32>,
) -> Result<Source> {
    if let Some(limit) = limit(text) {
        return Ok(Source::Limited(limit));
    }
    let syntaxes = syntax::set();
    let Some(syntax) = syntaxes.find_syntax_by_token(language) else {
        return Ok(Source::UnknownLanguage);
    };
    highlight_syntax(syntax, text, requested, || Ok(()))
}

fn highlight_syntax(
    syntax: &syntect::parsing::SyntaxReference,
    text: &str,
    requested: &BTreeSet<NonZeroU32>,
    mut checkpoint: impl FnMut() -> Result<()>,
) -> Result<Source> {
    let syntaxes = syntax::set();
    let theme = THEMES
        .themes
        .get("base16-ocean.dark")
        .context("Bundled syntax theme is missing")?;
    let mut parser = HighlightLines::new(syntax, theme);
    let mut lines = BTreeMap::new();
    let last = requested.last().map(|number| number.get()).unwrap_or(0);
    for (index, line) in LinesWithEndings::from(text).enumerate() {
        let number = NonZeroU32::new(index as u32 + 1).expect("bounded source line number");
        if number.get() > last {
            break;
        }
        checkpoint()?;
        let ranges = parser.highlight_line(line, syntaxes)?;
        if !requested.contains(&number) {
            continue;
        }
        let mut offset = 0;
        let tokens = ranges
            .into_iter()
            .map(|(style, part)| {
                let range = offset..offset + part.len();
                offset = range.end;
                Token {
                    range,
                    style: Style {
                        foreground: Color(
                            style.foreground.r,
                            style.foreground.g,
                            style.foreground.b,
                        ),
                        bold: style.font_style.contains(FontStyle::BOLD),
                        italic: style.font_style.contains(FontStyle::ITALIC),
                        underline: style.font_style.contains(FontStyle::UNDERLINE),
                    },
                }
            })
            .collect();
        lines.insert(
            number,
            Highlighted {
                text: line.to_owned(),
                tokens,
            },
        );
    }
    checkpoint()?;
    Ok(Source::Syntax {
        language: syntax.name.clone(),
        lines,
    })
}

fn limit(text: &str) -> Option<Limit> {
    if text.len() > MAX_SOURCE_BYTES {
        Some(Limit::SourceBytes)
    } else if text
        .split_inclusive('\n')
        .any(|line| line.len() > MAX_LINE_BYTES)
    {
        Some(Limit::LineBytes)
    } else if text.lines().count() > MAX_LINES {
        Some(Limit::LineCount)
    } else {
        None
    }
}
