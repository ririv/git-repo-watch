//! Adapt source tokens to safe terminal spans without mixing lexical states.
use super::*;
use anyhow::ensure;
use engine::local::files::patch::{Source, SourceOmission};
use highlight::{Highlighted, Limit};
use std::num::NonZeroU32;

#[derive(Clone, Copy)]
pub(crate) enum Side {
    Old,
    New,
}

pub(crate) enum Paint {
    Absent,
    Source(highlight::Source),
    Omitted(SourceOmission),
}
impl Paint {
    pub fn line(&self, number: NonZeroU32) -> Option<&Highlighted> {
        match self {
            Self::Source(highlight::Source::Syntax { lines, .. }) => lines.get(&number),
            _ => None,
        }
    }
    pub fn label(&self) -> Option<String> {
        Some(
            match self {
                Self::Absent => return None,
                Self::Source(source) => match source {
                    highlight::Source::Syntax { language, .. } => return Some(language.clone()),
                    highlight::Source::UnknownLanguage => i18n::tr!(SyntaxUnknown),
                    highlight::Source::Limited(Limit::SourceBytes) => i18n::tr!(SyntaxSourceLimit),
                    highlight::Source::Limited(Limit::LineBytes) => i18n::tr!(SyntaxLineLimit),
                    highlight::Source::Limited(Limit::LineCount) => i18n::tr!(SyntaxLineCountLimit),
                },
                Self::Omitted(reason) => match reason {
                    SourceOmission::Binary => i18n::tr!(SyntaxBinary),
                    SourceOmission::TooLarge => i18n::tr!(SyntaxSourceLimit),
                    SourceOmission::NonRegular => i18n::tr!(SyntaxNonRegular),
                },
            }
            .into(),
        )
    }
}

pub(crate) fn prepare_with_theme(
    source: &Source,
    requested: &BTreeSet<NonZeroU32>,
    patch: &[PatchLine],
    side: Side,
    theme: highlight::Theme,
    cancel: &Cancel,
) -> Result<Paint> {
    match source {
        Source::Image(_) => Ok(Paint::Omitted(SourceOmission::Binary)),
        Source::Absent => Ok(Paint::Absent),
        Source::Unavailable(reason) => Ok(Paint::Omitted(*reason)),
        Source::Text { path, text } => {
            let mut lines = text.split_inclusive('\n').enumerate();
            // 行号来自 libgit2；磁盘可能在生成 patch 后被外部编辑，不能把其他内容的颜色套上来。
            for line in patch {
                cancel.check()?;
                let number = match (side, line) {
                    (
                        Side::Old,
                        PatchLine::Context { old, .. } | PatchLine::Removed { old, .. },
                    ) => *old,
                    (Side::New, PatchLine::Context { new, .. } | PatchLine::Added { new, .. }) => {
                        *new
                    }
                    _ => continue,
                };
                ensure!(
                    lines
                        .find(|(index, _)| *index + 1 == number.get() as usize)
                        .is_some_and(|(_, source)| content(source) == content(line.content())),
                    i18n::tr!(DiffSourceChanged)
                );
            }
            Ok(Paint::Source(highlight::highlight_with_theme(
                path,
                text,
                requested,
                theme,
                || cancel.check(),
            )?))
        }
    }
}

fn content(text: &str) -> &str {
    let text = text.strip_suffix('\n').unwrap_or(text);
    text.strip_suffix('\r').unwrap_or(text)
}

pub(crate) fn spans(
    text: &str,
    highlighted: Option<&Highlighted>,
    base: Style,
) -> Vec<Span<'static>> {
    let text = content(text);
    match highlighted {
        None => super::text::spans([(text, base)]),
        Some(line) => super::text::spans(line.tokens.iter().filter_map(|token| {
            let start = token.range.start.min(text.len());
            let end = token.range.end.min(text.len());
            if start == end {
                return None;
            }
            let highlight::Color(r, g, b) = token.style.foreground;
            let mut style = base.fg(Color::Rgb(r, g, b));
            if token.style.bold {
                style = style.add_modifier(Modifier::BOLD);
            }
            if token.style.italic {
                style = style.add_modifier(Modifier::ITALIC);
            }
            if token.style.underline {
                style = style.add_modifier(Modifier::UNDERLINED);
            }
            Some((&text[start..end], style))
        })),
    }
}
