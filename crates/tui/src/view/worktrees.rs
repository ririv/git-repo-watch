//! Show the source checkout and the other linked worktrees in one list.
use super::*;
use crate::worktrees::Session;
use engine::local::worktrees::{Kind, Revision, State, Status};

pub(super) fn render(
    frame: &mut Frame,
    area: Rect,
    session: &mut Session,
    hits: &mut crate::pointer::Map,
) {
    let panel = block(i18n::tr!(WorktreesTitle));
    let inner = panel.inner(area);
    frame.render_widget(panel, area);
    let selected = session.selected();
    let detail = selected.and_then(|entry| match (&entry.state, &entry.kind) {
        (State::Unavailable(error), _) => Some((safe(error), BAD)),
        (_, Kind::Locked(Some(reason))) => Some((safe(reason), MUTED)),
        _ => None,
    });
    let parts = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(u16::from(detail.is_some())),
    ])
    .split(inner);
    if let Some((detail, color)) = detail {
        frame.render_widget(
            Paragraph::new(detail).style(Style::default().fg(color)),
            parts[1],
        );
    }
    let lines = session
        .snapshot
        .entries
        .iter()
        .map(|entry| {
            let (branch, status, color) = match &entry.state {
                State::Available {
                    revision, status, ..
                } => {
                    let branch = match revision {
                        Revision::Branch { name, .. } | Revision::Unborn(name) => safe(name),
                        Revision::Detached(oid) => {
                            format!("{} · {:.8}", i18n::tr!(OverviewDetached), oid.to_string())
                        }
                    };
                    let (text, color) = match status {
                        Status::Clean => (i18n::tr!(CleanAlt), GOOD),
                        Status::Changed => (i18n::tr!(WorktreeChangedFiles), WARN),
                        Status::IgnoredFiles => (i18n::tr!(WorktreeIgnoredFiles), WARN),
                        Status::Operation => (i18n::tr!(WorktreeOperation), WARN),
                    };
                    (branch, text, color)
                }
                State::Unavailable(_) => (String::new(), i18n::tr!(WorktreeUnavailable), BAD),
            };
            let mut title = vec![Span::styled(branch.clone(), Style::default().fg(ACCENT))];
            if !branch.is_empty() {
                title.push(Span::raw(" · "));
            }
            title.push(Span::styled(status, Style::default().fg(color)));
            if entry.current {
                title.push(Span::raw(format!(" · {}", i18n::tr!(WorktreeCurrent))));
            }
            match &entry.kind {
                Kind::Main => title.push(Span::raw(format!(" · {}", i18n::tr!(WorktreeMain)))),
                Kind::Locked(_) => title.push(Span::styled(
                    format!(" · {}", i18n::tr!(WorktreeLocked)),
                    Style::default().fg(WARN),
                )),
                Kind::Linked => {}
            }
            title.push(Span::styled(
                format!("  {}", safe(&entry.path.to_string_lossy())),
                Style::default().fg(MUTED),
            ));
            Line::from(title)
        })
        .collect::<Vec<_>>();
    list_window::Scrolling {
        panel: area,
        inner: parts[0],
        pane: crate::horizontal::Pane::Worktrees,
        viewport: &mut session.horizontal,
    }
    .render(frame, hits, &mut session.state, &lines, None);
    let count = session.snapshot.entries.len();
    crate::scrollbar::vertical(frame, area, parts[0], count, session.state.offset());
    hits.list(
        area,
        parts[0],
        &(0..count).collect::<Vec<_>>(),
        session.state.offset(),
        crate::pointer::Target::Worktrees,
    );
}
