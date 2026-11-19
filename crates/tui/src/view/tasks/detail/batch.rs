//! Decode repository blocks from the task journal's batch report format.
use super::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

struct RepositoryFailure<'a> {
    path: &'a str,
    reasons: Vec<String>,
}

struct BatchFailure<'a> {
    repositories: Vec<RepositoryFailure<'a>>,
    trace: Option<&'a str>,
}

fn split_trace<'a>(record: &Record, detail: &'a str) -> Option<(&'a str, &'a str)> {
    let split = |plan: &str| {
        let (failures, trace) = detail.split_once(&format!("\n\n{plan}\n"))?;
        let mut lines = trace.lines();
        let path = lines.next()?;
        let step = lines.next()?;
        (Path::new(path).is_absolute() && step.starts_with(['✓', '–', '✗']))
            .then_some((failures, trace))
    };
    match record.spec.source {
        Source::Scheduled => {
            let (_, plan) = record.spec.title.split_once(" · ")?;
            split(plan)
        }
        Source::Interactive | Source::Startup => [i18n::Language::English, i18n::Language::Chinese]
            .into_iter()
            .find_map(|language| {
                let _language = i18n::using(language);
                split(&engine::workflow::Plan::fetch().summary())
            }),
        Source::Automatic | Source::Imported => None,
    }
}

fn parse_batch<'a>(record: &Record, detail: &'a str) -> Option<BatchFailure<'a>> {
    if !matches!(record.spec.scope, TaskScope::All) {
        return None;
    }
    let (failures, trace) = match split_trace(record, detail) {
        Some((failures, trace)) => (failures, Some(trace)),
        None => (detail, None),
    };
    let mut repositories: Vec<RepositoryFailure<'_>> = Vec::new();
    let mut positions = BTreeMap::new();
    let mut current = None;
    for paragraph in failures
        .split("\n\n")
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let header = paragraph
            .split_once(": ")
            .filter(|(path, _)| !path.contains('\n') && Path::new(path).is_absolute());
        if let Some((path, reason)) = header {
            let position = *positions.entry(path).or_insert_with(|| {
                let position = repositories.len();
                repositories.push(RepositoryFailure {
                    path,
                    reasons: Vec::new(),
                });
                position
            });
            current = Some(position);
            repositories[position]
                .reasons
                .push(reason.trim_start().to_owned());
        } else {
            let reason = repositories[current?].reasons.last_mut()?;
            reason.push_str("\n\n");
            reason.push_str(paragraph);
        }
    }
    (!repositories.is_empty()).then_some(BatchFailure {
        repositories,
        trace,
    })
}

fn has_publication_error(entry: &str, message: &str) -> bool {
    entry.match_indices(" · ").any(|(start, separator)| {
        let label = entry[..start].rsplit('\n').next().unwrap_or_default();
        label.starts_with("✗ ")
            && label.contains(" → ")
            && label.contains(":refs/heads/")
            && entry[start + separator.len()..]
                .strip_prefix(message)
                .is_some_and(|rest| rest.is_empty() || rest.starts_with('\n'))
    })
}

fn append_reasons(lines: &mut Vec<Line<'static>>, reasons: &[String]) {
    // 旧版把同一份 Git 输出同时写进步骤、分支和仓库汇总。以分支记录保留目标和原文。
    let copies = reasons
        .iter()
        .filter(|message| {
            reasons
                .iter()
                .any(|entry| has_publication_error(entry, message))
        })
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let mut first = true;
    for reason in reasons {
        if copies.contains(reason.as_str()) {
            continue;
        }
        if !first {
            lines.push(Line::default());
        }
        first = false;
        let embedded_copy = reason.split_once(": ").and_then(|(step, body)| {
            copies.iter().find_map(|message| {
                let rest = body.strip_prefix(message)?.strip_prefix('\n')?.trim_start();
                rest.starts_with(['✓', '✗']).then_some((step, rest))
            })
        });
        let body = match embedded_copy {
            Some((step, rest)) => {
                lines.push(Line::raw(step.to_owned()));
                rest
            }
            None => reason,
        };
        lines.extend(body.lines().map(|line| Line::raw(line.to_owned())));
    }
}

fn timeline(trace: &str, repositories: BTreeSet<&str>) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let mut rows = trace.lines().peekable();
    let mut visible = false;
    while let Some(row) = rows.next() {
        let step = row.starts_with("– ")
            || row
                .strip_prefix("✓ ")
                .or_else(|| row.strip_prefix("✗ "))
                .and_then(|rest| rest.split_once(' '))
                .is_some_and(|(time, _)| {
                    chrono::NaiveTime::parse_from_str(time, "%H:%M:%S").is_ok()
                });
        if step && visible {
            let status = if row.starts_with("– ") {
                row
            } else {
                row.split_once(" · ").map_or(row, |(status, _)| status)
            };
            lines.push(Line::raw(status.to_owned()));
        } else if Path::new(row).is_absolute()
            && rows
                .peek()
                .is_some_and(|next| next.starts_with(['✓', '–', '✗']))
        {
            visible = repositories.contains(row);
            if visible {
                lines.push(Line::default());
                lines.push(Line::styled(row.to_owned(), Style::default().fg(MUTED)));
            }
        }
    }
    lines
}

pub(super) fn append_lines(
    record: &Record,
    detail: &str,
    width: u16,
    lines: &mut Vec<Line<'static>>,
) -> Option<usize> {
    let batch = parse_batch(record, detail)?;
    let heading = format!(
        "{} · {}",
        i18n::tr!(TasksFailedRepositories),
        batch.repositories.len()
    );
    lines.push(section(&heading, BAD, width));
    let mut first_reason = None;
    for repository in &batch.repositories {
        lines.push(Line::default());
        lines.push(Line::styled(
            format!("× {}", name(Path::new(repository.path))),
            Style::default().fg(BAD).bold(),
        ));
        lines.push(Line::styled(
            repository.path.to_owned(),
            Style::default().fg(MUTED),
        ));
        first_reason.get_or_insert(lines.len());
        append_reasons(lines, &repository.reasons);
    }
    if let Some(trace) = batch.trace {
        let steps = timeline(
            trace,
            batch.repositories.iter().map(|repo| repo.path).collect(),
        );
        if !steps.is_empty() {
            lines.push(Line::default());
            lines.push(section(i18n::tr!(TasksExecutionLog), ACCENT, width));
            lines.extend(steps);
        }
    }
    first_reason
}
