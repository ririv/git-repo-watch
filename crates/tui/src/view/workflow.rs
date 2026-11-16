//! Workflow composition and contextual controls render in separate fixed panels.
use super::*;
use crate::workflow::Session;
use engine::workflow::Step;

pub(super) fn render(
    frame: &mut Frame,
    area: Rect,
    session: &mut Session,
    pointer: &mut crate::pointer::Map,
) {
    let actions_height = area.height.saturating_sub(9).clamp(3, 8).min(area.height);
    let parts =
        Layout::vertical([Constraint::Min(0), Constraint::Length(actions_height)]).split(area);
    render_plan(frame, parts[0], session, pointer);
    render_actions(frame, parts[1], session, pointer);
}
fn render_plan(
    frame: &mut Frame,
    area: Rect,
    session: &mut Session,
    pointer: &mut crate::pointer::Map,
) {
    let panel = block(session.title()).padding(ratatui::widgets::Padding::horizontal(1));
    let inner = panel.inner(area);
    frame.render_widget(panel, area);
    if inner.is_empty() {
        return;
    }
    let steps = session.steps();
    let order = steps
        .iter()
        .map(|step| step.token())
        .collect::<Vec<_>>()
        .join(" → ");
    let mut header = vec![
        Line::styled(i18n::tr!(WorkflowScope), Style::default().fg(MUTED)),
        Line::from(vec![
            Span::styled(
                format!("{}: ", i18n::tr!(WorkflowSequence)),
                Style::default().fg(MUTED),
            ),
            Span::styled(order, Style::default().fg(ACCENT).bold()),
        ]),
    ];
    header.push(match session.plan() {
        Ok(_) => Line::styled(i18n::tr!(WorkflowEditing), Style::default().fg(MUTED)),
        Err(error) => Line::styled(error.to_string(), Style::default().fg(BAD)),
    });
    let header = Paragraph::new(header);
    let header_height = 3.min(inner.height.saturating_sub(4));
    let parts =
        Layout::vertical([Constraint::Length(header_height), Constraint::Min(0)]).split(inner);
    frame.render_widget(header, parts[0]);
    let selected = session.cursor.row().index();
    let mut lines = Vec::new();
    let mut ordinal = 0;
    for index in 0..4 {
        let (step, enabled) = if index < 3 {
            let entry = session.entries[index];
            (entry.operation.step(), entry.enabled())
        } else {
            (Step::Inspect, true)
        };
        if enabled {
            ordinal += 1;
        }
        let base = if selected == index {
            selected_style()
        } else {
            Style::default()
        };
        let mut spans = vec![Span::styled(
            if enabled {
                format!("{ordinal} ")
            } else {
                "  ".into()
            },
            Style::default().fg(MUTED),
        )];
        spans.push(if index == 3 {
            Span::styled(
                format!("[{}] ", i18n::tr!(WorkflowRequired)),
                Style::default().fg(MUTED),
            )
        } else {
            Span::styled(
                if enabled { "[✓] " } else { "[ ] " },
                Style::default()
                    .fg(if enabled { GOOD } else { MUTED })
                    .bold(),
            )
        });
        spans.push(Span::styled(
            format!("{} · {} ", step.token(), step.label()),
            Style::default()
                .fg(if enabled { ACCENT } else { MUTED })
                .bold(),
        ));
        lines.push(Line::from(spans).style(base));
    }
    if selected < session.offset {
        session.offset = selected;
    } else if selected >= session.offset + usize::from(parts[1].height) {
        session.offset = (selected + 1).saturating_sub(usize::from(parts[1].height));
    }
    session.offset = session
        .offset
        .min(lines.len().saturating_sub(usize::from(parts[1].height)));
    frame.render_widget(
        Paragraph::new(lines).scroll((session.offset as u16, 0)),
        parts[1],
    );
    for index in 0..4 {
        if index < session.offset || index >= session.offset + usize::from(parts[1].height) {
            continue;
        }
        pointer.item(
            Rect::new(
                parts[1].x,
                parts[1].y + (index - session.offset) as u16,
                parts[1].width,
                1,
            ),
            if index < 3 {
                crate::pointer::Target::WorkflowToggle
            } else {
                crate::pointer::Target::WorkflowSteps
            },
            index,
        );
    }
}
fn render_actions(
    frame: &mut Frame,
    area: Rect,
    session: &mut Session,
    pointer: &mut crate::pointer::Map,
) {
    let panel =
        block(i18n::tr!(WorkflowSelectedStep)).padding(ratatui::widgets::Padding::horizontal(1));
    let inner = panel.inner(area);
    frame.render_widget(panel, area);
    if inner.is_empty() {
        session.revealed = None;
        return;
    }
    let heading_height = u16::from(inner.height >= 2);
    let parts =
        Layout::vertical([Constraint::Length(heading_height), Constraint::Min(0)]).split(inner);
    let step = session.selected_step();
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!("{}: ", i18n::tr!(WorkflowSelectedStep)),
                Style::default().fg(MUTED),
            ),
            Span::styled(
                format!("{} · {}", step.token(), step.label()),
                Style::default().fg(ACCENT).bold(),
            ),
        ])),
        parts[0],
    );
    let description = Paragraph::new(step.description())
        .style(Style::default().fg(MUTED))
        .wrap(Wrap { trim: false });
    let line_count = description.line_count(parts[1].width);
    session.scroll = session
        .scroll
        .min(line_count.saturating_sub(usize::from(parts[1].height)));
    session.revealed = Some((session.cursor.row(), parts[1]));
    frame.render_widget(
        description.scroll((session.scroll.min(u16::MAX as usize) as u16, 0)),
        parts[1],
    );
    pointer.vertical(
        area,
        parts[1],
        line_count.saturating_sub(usize::from(parts[1].height)),
        session.scroll,
        crate::pointer::ScrollTarget::Text,
    );
    crate::scrollbar::vertical(frame, area, parts[1], line_count, session.scroll);
}
