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
    let panel = block(i18n::tr!(SubmodulesTitle));
    let inner = panel.inner(area);
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
        append(&mut lines, &snapshot.entries, "");
    }
    let max_scroll = lines.len().saturating_sub(inner.height as usize);
    *scroll = (*scroll).min(max_scroll);
    frame.render_widget(panel, area);
    let visible = lines
        .iter()
        .skip(*scroll)
        .take(inner.height as usize)
        .cloned()
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(visible), inner);
    crate::scrollbar::vertical(frame, area, inner, lines.len(), *scroll);
    hits.vertical(
        area,
        inner,
        max_scroll,
        *scroll,
        crate::pointer::ScrollTarget::Text,
    );
}

fn append(lines: &mut Vec<Line<'static>>, entries: &[Entry], prefix: &str) {
    for (index, entry) in entries.iter().enumerate() {
        let last = index + 1 == entries.len();
        let connector = if last { "└─ " } else { "├─ " };
        let mut row = vec![
            Span::styled(format!("{prefix}{connector}"), Style::default().fg(MUTED)),
            Span::raw(safe(&entry.path.to_string_lossy())),
        ];
        match entry.link {
            Link::Unrecorded => row.push(Span::styled(
                format!(" · {}", i18n::tr!(SubmoduleUnrecorded)),
                Style::default().fg(MUTED),
            )),
            Link::Recorded(oid) => {
                row.push(commit(i18n::tr!(SubmoduleRecorded), oid, ACCENT));
            }
            Link::Staged { recorded, indexed } => {
                if let Some(oid) = recorded {
                    row.push(commit(i18n::tr!(SubmoduleRecorded), oid, ACCENT));
                }
                row.push(commit(i18n::tr!(SubmoduleStaged), indexed, GOOD));
            }
            Link::StagedRemoval(oid) => {
                row.push(commit(i18n::tr!(SubmoduleRecorded), oid, ACCENT));
                row.push(Span::styled(
                    format!(" · {}", i18n::tr!(SubmoduleStagedRemoval)),
                    Style::default().fg(WARN),
                ));
            }
        }
        match &entry.state {
            State::Initialized {
                current: Some(oid), ..
            } if Some(*oid) != entry.link.indexed() => {
                row.push(commit(i18n::tr!(SubmoduleCheckedOut), *oid, WARN));
            }
            State::Initialized { current: None, .. } => row.push(Span::styled(
                format!(" · {}", i18n::tr!(OverviewUnborn)),
                Style::default().fg(MUTED),
            )),
            State::Initialized { .. } => {}
            State::Uninitialized if matches!(entry.link, Link::StagedRemoval(_)) => {}
            State::Uninitialized => row.push(Span::styled(
                format!(" · {}", i18n::tr!(SubmoduleUninitialized)),
                Style::default().fg(WARN),
            )),
            State::Unavailable(error) => row.push(Span::styled(
                format!(" · {} · {}", i18n::tr!(SubmoduleUnavailable), safe(error)),
                Style::default().fg(BAD),
            )),
        }
        lines.push(Line::from(row));
        if let State::Initialized { children, .. } = &entry.state {
            let continuation = if last { "   " } else { "│  " };
            append(lines, children, &format!("{prefix}{continuation}"));
        }
    }
}

fn commit(label: &str, oid: engine::local::history::Oid, color: Color) -> Span<'static> {
    Span::styled(
        format!(" · {label} {:.12}", oid.to_string()),
        Style::default().fg(color),
    )
}
