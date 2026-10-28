//! Workflow composition and contextual controls render in separate fixed panels.
use super::*;
use crate::workflow::{Action, Session};
use engine::workflow::Step;
use unicode_width::UnicodeWidthStr;

fn action_index(action: Action) -> usize {
    Action::ALL
        .iter()
        .position(|item| *item == action)
        .expect("workflow action")
}
fn controls(
    frame: &mut Frame,
    area: Rect,
    session: &Session,
    actions: &[Action],
    pointer: &mut crate::pointer::Map,
) {
    let mut x = 0u16;
    let labels = actions
        .iter()
        .map(|action| {
            let label = format!("[ {} ]", session.action_label(*action));
            let start = x;
            x = x.saturating_add(label.width() as u16 + 2);
            (*action, start, label)
        })
        .collect::<Vec<_>>();
    let shift = labels
        .iter()
        .find(|(action, _, _)| session.action_selected(*action))
        .map_or(0, |(_, start, label)| {
            start
                .saturating_add(label.width() as u16)
                .saturating_sub(area.width)
        });
    for (action, start, label) in labels {
        let width = label.width() as u16;
        if start.saturating_add(width) <= shift {
            continue;
        }
        let clipped = shift.saturating_sub(start) as usize;
        let label = label.chars().skip(clipped).collect::<String>();
        let rect = Rect::new(
            area.x.saturating_add(start.saturating_sub(shift)),
            area.y,
            width.saturating_sub(clipped as u16),
            area.height.min(1),
        )
        .intersection(area);
        let enabled = session.enabled(action);
        let style = if !enabled {
            Style::default().fg(MUTED)
        } else if session.action_selected(action) {
            selected_style().fg(ACCENT).bold()
        } else {
            Style::default()
                .fg(if action == Action::Save { GOOD } else { ACCENT })
                .bold()
        };
        frame.render_widget(Paragraph::new(label).style(style), rect);
        if enabled {
            pointer.item(
                rect,
                crate::pointer::Target::WorkflowActions,
                action_index(action),
            );
        }
    }
}
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
        block(i18n::tr!(WorkflowControls)).padding(ratatui::widgets::Padding::horizontal(1));
    let inner = panel.inner(area);
    frame.render_widget(panel, area);
    if inner.is_empty() {
        session.revealed = None;
        return;
    }
    let footer_height = 1;
    let controls_height = u16::from(inner.height >= 2);
    let heading_height = u16::from(inner.height >= 3);
    let parts = Layout::vertical([
        Constraint::Length(heading_height),
        Constraint::Min(0),
        Constraint::Length(controls_height),
        Constraint::Length(footer_height),
    ])
    .split(inner);
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
    if session.cursor.row().index() < 3 {
        controls(
            frame,
            parts[2],
            session,
            &[Action::Toggle, Action::Up, Action::Down],
            pointer,
        );
    } else {
        frame.render_widget(
            Paragraph::new(i18n::tr!(WorkflowFixedInspect)).style(Style::default().fg(MUTED)),
            parts[2],
        );
    }
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
    controls(
        frame,
        parts[3],
        session,
        &[Action::Save, Action::Cancel],
        pointer,
    );
}
