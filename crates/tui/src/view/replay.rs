//! Explicit mainline selection for replaying a merge commit.
pub(super) mod controls;
use super::*;
use engine::operations::replay::Candidate;
pub(super) fn parents(
    frame: &mut Frame,
    area: Rect,
    candidate: &Candidate,
    selected: usize,
    viewport: &mut crate::horizontal::Viewport,
    hits: &mut crate::pointer::Map,
) {
    let border = block(i18n::tr!(ReplayParentsTitle));
    let inner = border.inner(area);
    frame.render_widget(border, area);
    let parts = Layout::vertical([Constraint::Length(7), Constraint::Min(0)]).split(inner);
    let introduction = Text::from(vec![
        Line::styled(candidate.kind.label(), Style::default().fg(ACCENT).bold()),
        Line::raw(format!(
            "{}: {}",
            i18n::tr!(ReplayTargetBranch),
            safe(&candidate.branch)
        )),
        Line::raw(format!("{}: {}", i18n::tr!(ReplaySource), candidate.commit)),
        Line::raw(safe(&candidate.summary)),
        Line::raw(""),
        Line::styled(i18n::tr!(ReplayMainlineHelp), Style::default().fg(MUTED)),
        Line::styled(i18n::tr!(ReplayMainline), Style::default().fg(ACCENT)),
    ]);
    frame.render_widget(
        Paragraph::new(introduction).wrap(Wrap { trim: false }),
        parts[0],
    );
    let lines = candidate
        .parents
        .iter()
        .map(|parent| {
            Line::from(vec![
                Span::styled(
                    format!("{}  {}  ", parent.number, &parent.oid.to_string()[..12]),
                    Style::default().fg(ACCENT),
                ),
                Span::raw(safe(&parent.summary)),
            ])
        })
        .collect::<Vec<_>>();
    let mut state = ListState::default().with_selected(Some(selected));
    list_window::Scrolling {
        panel: area,
        inner: parts[1],
        pane: crate::horizontal::Pane::ReplayParents,
        viewport,
    }
    .render(frame, hits, &mut state, &lines, None);
    hits.list(
        area,
        parts[1],
        &(0..candidate.parents.len()).collect::<Vec<_>>(),
        state.offset(),
        crate::pointer::Target::ReplayParent,
    );
}
