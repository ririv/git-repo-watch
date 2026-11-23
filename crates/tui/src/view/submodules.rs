//! Display the declared submodule hierarchy without opening any nested repository view.
use super::*;
use engine::local::submodules::{Entry, Link, Snapshot, State};

pub(super) fn render(
    frame: &mut Frame,
    area: Rect,
    snapshot: &Snapshot,
    scroll: &mut usize,
    hits: &mut crate::pointer::Map,
) {
    let title = i18n::tr!(SubmodulesTitle);
    let width = block(title).inner(area).width as usize;
    let mut lines = Vec::new();
    if snapshot.entries.is_empty() {
        lines.push(Line::styled(
            i18n::tr!(NoSubmodules),
            Style::default().fg(MUTED),
        ));
    } else {
        lines.push(Line::styled(
            safe(&name(&snapshot.path)),
            Style::default().fg(ACCENT),
        ));
        append(&mut lines, &snapshot.entries, "", width);
    }
    review::styled_text_view(
        frame,
        area,
        Text::from(lines),
        scroll,
        title,
        hits,
        crate::pointer::ScrollTarget::Text,
    );
}

fn append(lines: &mut Vec<Line<'static>>, entries: &[Entry], prefix: &str, width: usize) {
    for (index, entry) in entries.iter().enumerate() {
        let last = index + 1 == entries.len();
        let connector = if last { "└─ " } else { "├─ " };
        let mut row = Line::from(vec![
            Span::styled(format!("{prefix}{connector}"), Style::default().fg(MUTED)),
            Span::raw(safe(&entry.path.to_string_lossy())),
        ]);
        let mut details = Vec::new();
        match entry.link {
            Link::Unrecorded => details.push(Span::styled(
                format!(" · {}", i18n::tr!(SubmoduleUnrecorded)),
                Style::default().fg(MUTED),
            )),
            Link::Recorded(oid) => {
                details.push(commit(i18n::tr!(SubmoduleRecorded), oid, ACCENT));
            }
            Link::Staged { recorded, indexed } => {
                if let Some(oid) = recorded {
                    details.push(commit(i18n::tr!(SubmoduleRecorded), oid, ACCENT));
                }
                details.push(commit(i18n::tr!(SubmoduleStaged), indexed, GOOD));
            }
            Link::StagedRemoval(oid) => {
                details.push(commit(i18n::tr!(SubmoduleRecorded), oid, ACCENT));
                details.push(Span::styled(
                    format!(" · {}", i18n::tr!(SubmoduleStagedRemoval)),
                    Style::default().fg(WARN),
                ));
            }
        }
        match &entry.state {
            State::Initialized {
                current: Some(oid), ..
            } if Some(*oid) != entry.link.indexed() => {
                details.push(commit(i18n::tr!(SubmoduleCheckedOut), *oid, WARN));
            }
            State::Initialized { current: None, .. } => details.push(Span::styled(
                format!(" · {}", i18n::tr!(OverviewUnborn)),
                Style::default().fg(MUTED),
            )),
            State::Initialized { .. } => {}
            State::Uninitialized if matches!(entry.link, Link::StagedRemoval(_)) => {}
            State::Uninitialized => details.push(Span::styled(
                format!(" · {}", i18n::tr!(SubmoduleUninitialized)),
                Style::default().fg(WARN),
            )),
            State::Unavailable(error) => details.push(Span::styled(
                format!(" · {} · {}", i18n::tr!(SubmoduleUnavailable), safe(error)),
                Style::default().fg(BAD),
            )),
        }
        let continuation = if last { "   " } else { "│  " };
        let indent = format!("{prefix}{continuation}");
        if row.width() + details.iter().map(Span::width).sum::<usize>() <= width {
            row.spans.extend(details);
            lines.push(row);
        } else {
            lines.push(row);
            lines.extend(details.into_iter().map(|detail| {
                Line::from(vec![
                    Span::styled(indent.clone(), Style::default().fg(MUTED)),
                    detail,
                ])
            }));
        }
        if let State::Initialized { children, .. } = &entry.state {
            append(lines, children, &indent, width);
        }
    }
}

fn commit(label: &str, oid: engine::local::history::Oid, color: Color) -> Span<'static> {
    Span::styled(
        format!(" · {label} {:.12}", oid.to_string()),
        Style::default().fg(color),
    )
}
