//! Prepare both source sides once, retaining code spans without layout-specific gutters.
use super::*;

pub(super) enum Heading<'a> {
    Full(&'a str),
    Inline,
}

pub(super) struct Prepared {
    pub rows: Vec<Row>,
    pub information: Option<Information>,
    pub header: Vec<String>,
    pub full: Result<Vec<Row>, &'static str>,
}

pub(super) fn rows(
    heading: Heading<'_>,
    document: Document,
    theme: highlight::Theme,
    cancel: &Cancel,
) -> Result<Prepared> {
    cancel.check()?;
    let mut information = match heading {
        Heading::Full(title) => Some(Information {
            title: safe(title),
            syntax: Vec::new(),
            facts: Vec::new(),
        }),
        Heading::Inline => None,
    };
    let mut rows = Vec::new();
    let mut header = Vec::new();
    let mut full_rows = Ok(Vec::new());
    match document {
        Document::Notice(text) => {
            metadata(&mut rows, &text, Style::default());
            full_rows = Ok(rows.clone());
        }
        Document::Patch { files, extent } => {
            for file in files {
                let raw = file
                    .lines
                    .iter()
                    .filter_map(|line| match line {
                        PatchLine::Metadata {
                            kind: Metadata::File,
                            text,
                        } => Some(text),
                        _ => None,
                    })
                    .flat_map(|text| text.lines().map(str::to_owned))
                    .collect::<Vec<_>>();
                let facts = header::facts(&raw)?;
                header.extend(raw);
                let expanded =
                    super::full::expand(&file.before, &file.after, &file.lines, extent, cancel)?;
                let mut old_numbers = BTreeSet::new();
                let mut new_numbers = BTreeSet::new();
                let full_lines = match &expanded {
                    super::full::Expansion::Text(lines) => lines.as_slice(),
                    _ => &[],
                };
                for line in file.lines.iter().chain(full_lines) {
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
                let old = source::prepare_with_theme(
                    &file.before,
                    &old_numbers,
                    &file.lines,
                    source::Side::Old,
                    theme,
                    cancel,
                )?;
                let new = source::prepare_with_theme(
                    &file.after,
                    &new_numbers,
                    &file.lines,
                    source::Side::New,
                    theme,
                    cancel,
                )?;
                let start = rows.len();
                let mut labels = Vec::new();
                for label in [old.label(), new.label()].into_iter().flatten() {
                    if !labels.contains(&label) {
                        labels.push(label);
                    }
                }
                if !labels.is_empty() {
                    let syntax = labels.join(" → ");
                    if let Some(information) = &mut information {
                        if !information.syntax.contains(&syntax) {
                            information.syntax.push(syntax);
                        }
                    } else {
                        rows.push(Row::Information(header::summary(&[header::Fact::Field {
                            label: i18n::tr!(SyntaxLabel),
                            value: syntax,
                        }])));
                    }
                }
                if let Some(information) = &mut information {
                    information.facts.extend(facts);
                } else if !facts.is_empty() {
                    rows.push(Row::Information(header::summary(&facts)));
                }
                let body = body(&file.lines, &old, &new, cancel)?;
                let information = rows[start..].to_vec();
                match expanded {
                    super::full::Expansion::Unavailable(reason) => full_rows = Err(reason),
                    expanded => {
                        if let Ok(full) = &mut full_rows {
                            full.extend(information);
                            match expanded {
                                super::full::Expansion::Text(lines) => {
                                    full.extend(self::body(&lines, &old, &new, cancel)?)
                                }
                                super::full::Expansion::NonText => full.extend(body.clone()),
                                super::full::Expansion::Unavailable(_) => unreachable!(),
                            }
                        }
                    }
                }
                rows.extend(body);
                if matches!(heading, Heading::Full(_)) {
                    rows.push(Row::Metadata(Line::default()));
                }
            }
            if extent == Extent::Truncated {
                metadata(
                    &mut rows,
                    i18n::tr!(DiffExceeds128KibOnlyTheBeginning),
                    Style::default().fg(Color::Yellow),
                );
            }
        }
    }
    cancel.check()?;
    Ok(Prepared {
        rows,
        information,
        header,
        full: full_rows,
    })
}
fn body(
    lines: &[PatchLine],
    old: &source::Paint,
    new: &source::Paint,
    cancel: &Cancel,
) -> Result<Vec<Row>> {
    let mut rows = Vec::new();
    let code = |number, text: &str, paint: &source::Paint| Code {
        number,
        content: Line::from(source::spans(text, paint.line(number), Style::default())),
        ending: if text.ends_with('\n') {
            Ending::Newline
        } else {
            Ending::EndOfFile
        },
    };
    for line in lines {
        cancel.check()?;
        match line {
            PatchLine::Context {
                old: before,
                new: after,
                text,
            } => rows.push(Row::Context {
                old: code(*before, text, old),
                new: code(*after, text, new),
            }),
            PatchLine::Added { new: number, text } => {
                rows.push(Row::Added(code(*number, text, new)))
            }
            PatchLine::Removed { old: number, text } => {
                rows.push(Row::Removed(code(*number, text, old)))
            }
            PatchLine::Metadata { kind, text } => {
                let (text, style) = match kind {
                    Metadata::File | Metadata::NoNewline => continue,
                    Metadata::Hunk => {
                        (text.clone(), Style::default().fg(Color::Rgb(180, 142, 220)))
                    }
                    Metadata::Notice => (text.clone(), Style::default().fg(MUTED).italic()),
                    Metadata::Binary { before, after } => (
                        notice::binary(before, after),
                        Style::default().fg(MUTED).italic(),
                    ),
                };
                metadata(&mut rows, &text, style);
            }
        }
    }
    Ok(rows)
}
fn metadata(rows: &mut Vec<Row>, text: &str, style: Style) {
    rows.extend(
        text.lines()
            .map(|line| Row::Metadata(Line::styled(safe(line), style))),
    );
}
