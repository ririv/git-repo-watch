//! Prepare both source sides once, retaining code spans without layout-specific gutters.
use super::*;

pub(super) enum Heading<'a> {
    Full(&'a str),
    Inline,
}

pub(super) fn rows(heading: Heading<'_>, document: Document, cancel: &Cancel) -> Result<Vec<Row>> {
    cancel.check()?;
    let mut rows = match heading {
        Heading::Full(title) => vec![
            Row::Metadata(Line::styled(
                safe(title),
                Style::default().fg(Color::LightBlue).bold(),
            )),
            Row::Metadata(Line::default()),
        ],
        Heading::Inline => Vec::new(),
    };
    match document {
        Document::Notice(text) => metadata(&mut rows, &text, Style::default()),
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
                let mut labels = Vec::new();
                for label in [old.label(), new.label()].into_iter().flatten() {
                    if !labels.contains(&label) {
                        labels.push(label);
                    }
                }
                if !labels.is_empty() {
                    metadata(
                        &mut rows,
                        &format!("{}: {}", i18n::tr!(SyntaxLabel), labels.join(" → ")),
                        Style::default().fg(MUTED),
                    );
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
                            if matches!(heading, Heading::Inline) && kind == Metadata::File {
                                // 连续视图已绘制路径标题；权限、重命名与二进制等元数据仍需显示。
                                for line in text.lines().filter(|line| {
                                    !["diff --git ", "index ", "--- ", "+++ "]
                                        .iter()
                                        .any(|prefix| line.starts_with(prefix))
                                }) {
                                    metadata(
                                        &mut rows,
                                        line,
                                        Style::default().fg(Color::LightBlue).bold(),
                                    );
                                }
                                continue;
                            }
                            let style = match kind {
                                Metadata::File => Style::default().fg(Color::LightBlue).bold(),
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
    Ok(rows)
}
fn metadata(rows: &mut Vec<Row>, text: &str, style: Style) {
    rows.extend(
        text.lines()
            .map(|line| Row::Metadata(Line::styled(safe(line), style))),
    );
}
