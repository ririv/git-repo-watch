//! Prepare immutable diff styling in the file worker; redraws reuse the paragraph.
mod source;
#[cfg(test)]
mod tests;
mod text;
use crate::view::safe;
use anyhow::Result;
use engine::{
    cancel::Cancel,
    local::files::patch::{Document, Extent, Line as PatchLine, Metadata},
};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Paragraph, Wrap},
};
use std::collections::BTreeSet;

pub(super) const ADDED: Color = Color::Rgb(28, 51, 43);
pub(super) const REMOVED: Color = Color::Rgb(58, 32, 41);
const MUTED: Color = Color::Rgb(125, 140, 155);

#[derive(Debug)]
pub(super) struct View {
    pub paragraph: Paragraph<'static>,
    measurement: Option<(u16, usize)>,
}
impl View {
    pub fn line_count(&mut self, width: u16) -> usize {
        if let Some((measured_width, lines)) = self.measurement
            && measured_width == width
        {
            return lines;
        }
        let lines = self.paragraph.line_count(width);
        self.measurement = Some((width, lines));
        lines
    }
}

pub(super) fn prepare(title: &str, document: Document, cancel: &Cancel) -> Result<Box<View>> {
    let text = styled(title, document, cancel)?;
    Ok(Box::new(View {
        paragraph: Paragraph::new(text).wrap(Wrap { trim: false }),
        measurement: None,
    }))
}

fn styled(title: &str, document: Document, cancel: &Cancel) -> Result<Text<'static>> {
    cancel.check()?;
    let mut lines = vec![
        Line::styled(safe(title), Style::default().fg(Color::LightBlue).bold()),
        Line::default(),
    ];
    match document {
        Document::Notice(text) => lines.extend(text.lines().map(|line| Line::raw(safe(line)))),
        Document::Patch { files, extent } => {
            for file in files {
                let mut old_numbers = BTreeSet::new();
                let mut new_numbers = BTreeSet::new();
                for line in &file.lines {
                    match line {
                        PatchLine::Context { old, new, .. } => {
                            old_numbers.insert(*old);
                            new_numbers.insert(*new);
                        }
                        PatchLine::Added { new, .. } => {
                            new_numbers.insert(*new);
                        }
                        PatchLine::Removed { old, .. } => {
                            old_numbers.insert(*old);
                        }
                        _ => {}
                    }
                }
                let old = source::prepare(
                    &file.before,
                    &old_numbers,
                    &file.lines,
                    source::Side::Old,
                    cancel,
                )?;
                let new = source::prepare(
                    &file.after,
                    &new_numbers,
                    &file.lines,
                    source::Side::New,
                    cancel,
                )?;
                let labels = [old.label(), new.label()]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>();
                let mut unique = Vec::new();
                for label in labels {
                    if !unique.contains(&label) {
                        unique.push(label);
                    }
                }
                if !unique.is_empty() {
                    lines.push(Line::styled(
                        format!("{}: {}", i18n::tr!(SyntaxLabel), unique.join(" → ")),
                        Style::default().fg(MUTED),
                    ));
                }
                let max_number = old_numbers
                    .iter()
                    .chain(&new_numbers)
                    .copied()
                    .map(|number| number.get())
                    .max()
                    .unwrap_or(1);
                let width = max_number.to_string().len();
                for line in file.lines {
                    cancel.check()?;
                    let (before, after, prefix, background, tokens) = match &line {
                        PatchLine::Context {
                            old: before,
                            new: after,
                            ..
                        } => (
                            format!("{before:>width$}"),
                            format!("{after:>width$}"),
                            " ",
                            None,
                            new.line(*after),
                        ),
                        PatchLine::Added { new: number, .. } => (
                            " ".repeat(width),
                            format!("{number:>width$}"),
                            "+",
                            Some(ADDED),
                            new.line(*number),
                        ),
                        PatchLine::Removed { old: number, .. } => (
                            format!("{number:>width$}"),
                            " ".repeat(width),
                            "-",
                            Some(REMOVED),
                            old.line(*number),
                        ),
                        PatchLine::Metadata { kind, text } => {
                            let style = match kind {
                                Metadata::File => Style::default().fg(Color::LightBlue).bold(),
                                Metadata::Hunk => Style::default().fg(Color::Rgb(180, 142, 220)),
                                Metadata::Notice => Style::default().fg(MUTED).italic(),
                            };
                            lines.extend(text.lines().map(|text| Line::styled(safe(text), style)));
                            continue;
                        }
                    };
                    let base = background
                        .map(|color| Style::default().bg(color))
                        .unwrap_or_default();
                    let marker = match line {
                        PatchLine::Added { .. } => Color::LightGreen,
                        PatchLine::Removed { .. } => Color::LightRed,
                        _ => MUTED,
                    };
                    let mut spans = vec![
                        Span::styled(format!("{before} {after} │"), base.fg(MUTED)),
                        Span::styled(prefix, base.fg(marker)),
                    ];
                    spans.extend(source::spans(line.content(), tokens, base));
                    lines.push(Line::from(spans).style(base));
                }
                lines.push(Line::default());
            }
            if extent == Extent::Truncated {
                lines.extend(
                    i18n::tr!(DiffExceeds128KibOnlyTheBeginning)
                        .lines()
                        .map(|text| Line::styled(safe(text), Style::default().fg(Color::Yellow))),
                );
            }
        }
    }
    cancel.check()?;
    Ok(Text::from(lines))
}
