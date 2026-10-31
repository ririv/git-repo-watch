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
}

pub(super) fn rows(heading: Heading<'_>, document: Document, cancel: &Cancel) -> Result<Prepared> {
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
    match document {
        Document::Notice(text) => metadata(&mut rows, &text, Style::default()),
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
                let code = |number, text: &str, paint: &source::Paint| Code {
                    number,
                    content: Line::from(source::spans(text, paint.line(number), Style::default())),
                };
                for line in file.lines {
                    cancel.check()?;
                    match line {
                        PatchLine::Context {
                            old: before,
                            new: after,
                            text,
                        } => rows.push(Row::Context {
                            old: code(before, &text, &old),
                            new: code(after, &text, &new),
                        }),
                        PatchLine::Added { new: number, text } => {
                            rows.push(Row::Added(code(number, &text, &new)))
                        }
                        PatchLine::Removed { old: number, text } => {
                            rows.push(Row::Removed(code(number, &text, &old)))
                        }
                        PatchLine::Metadata { kind, text } => {
                            if kind == Metadata::File {
                                continue;
                            }
                            let style = match kind {
                                Metadata::File => Style::default().fg(MUTED),
                                Metadata::Hunk => Style::default().fg(Color::Rgb(180, 142, 220)),
                                Metadata::Notice => Style::default().fg(MUTED).italic(),
                            };
                            metadata(&mut rows, &text, style);
                        }
                    }
                }
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
    })
}
fn metadata(rows: &mut Vec<Row>, text: &str, style: Style) {
    rows.extend(
        text.lines()
            .map(|line| Row::Metadata(Line::styled(safe(line), style))),
    );
}
