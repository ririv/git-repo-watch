//! A commit sequence, assigned units, and the selected patch share one review surface.
use super::*;
use crate::composition::{Focus, Session, State};
pub(super) fn waiting(
    frame: &mut Frame,
    area: Rect,
    waiting: &mut crate::composition::waiting::Session,
    hits: &mut crate::pointer::Map,
) {
    use crate::composition::waiting::State as Generation;
    if let crate::Mode::Composition(previous) = waiting.origin.as_mut() {
        // 重新生成时保留上次计划供查看，临时命中图不会开放旧计划的编辑操作。
        render(frame, area, previous, &mut crate::pointer::Map::default());
    } else {
        let (steps, detail) = areas(area);
        let (message, units, preview) = detail_areas(detail);
        for (panel, title) in [
            (steps, i18n::tr!(ComposePending)),
            (message, i18n::tr!(CommitMessageAlt)),
            (units, i18n::tr!(ComposeUnits)),
            (preview, i18n::tr!(CommitPreview)),
        ] {
            frame.render_widget(block(title), panel);
        }
    }
    let (_, detail) = areas(area);
    let (message, _, preview) = detail_areas(detail);
    frame.render_widget(Clear, message);
    let status = match &waiting.state {
        Generation::Running(animation) => {
            format!("{} {}", animation.symbol(), i18n::tr!(ComposeGenerating))
        }
        Generation::Failed { .. } => i18n::tr!(ComposeGenerationFailed).into(),
    };
    let panel = block(i18n::tr!(CommitMessageAlt));
    let inner = panel.inner(message);
    frame.render_widget(panel, message);
    frame.render_widget(
        Paragraph::new(status)
            .style(
                Style::default().fg(if matches!(waiting.state, Generation::Running(_)) {
                    ACCENT
                } else {
                    BAD
                }),
            )
            .wrap(Wrap { trim: false }),
        inner,
    );
    if let Generation::Failed { detail, scroll } = &mut waiting.state {
        frame.render_widget(Clear, preview);
        text_view(
            frame,
            preview,
            detail,
            scroll,
            i18n::tr!(ComposeGenerationFailed),
            hits,
        );
    }
}
pub(crate) fn areas(area: Rect) -> (Rect, Rect) {
    let parts = if area.width >= 90 {
        Layout::horizontal([Constraint::Percentage(35), Constraint::Percentage(65)]).split(area)
    } else {
        Layout::vertical([
            Constraint::Length(6.min(area.height / 3)),
            Constraint::Min(0),
        ])
        .split(area)
    };
    (parts[0], parts[1])
}
pub(crate) fn detail_areas(area: Rect) -> (Rect, Rect, Rect) {
    let parts = Layout::vertical([
        Constraint::Length(6.min(area.height / 3)),
        Constraint::Length(6.min(area.height / 3)),
        Constraint::Min(0),
    ])
    .split(area);
    (parts[0], parts[1], parts[2])
}
pub(super) fn render(
    frame: &mut Frame,
    area: Rect,
    session: &mut Session,
    hits: &mut crate::pointer::Map,
) {
    let (steps, detail) = areas(area);
    let step_panel = block(format!(
        "{} · {}",
        i18n::tr!(ComposePending),
        session.prepared.plan.commits.len()
    ));
    let step_inner = step_panel.inner(steps);
    let selected = match session.state {
        State::Moving { target, .. } => target,
        _ => session.selected,
    };
    session.commits.select(Some(selected));
    let rows = session
        .prepared
        .plan
        .commits
        .iter()
        .enumerate()
        .map(|(index, step)| {
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{} ", index + 1 + session.prepared.completed.len()),
                    Style::default().fg(MUTED),
                ),
                Span::raw(safe(step.message.lines().next().unwrap_or(""))),
                Span::styled(
                    format!(" · {}", step.units.len()),
                    Style::default().fg(ACCENT),
                ),
            ]))
        })
        .collect::<Vec<_>>();
    frame.render_stateful_widget(
        List::new(rows)
            .block(step_panel)
            .highlight_style(selected_style()),
        steps,
        &mut session.commits,
    );
    if session.focus == Focus::Commits && !matches!(session.state, State::Editing(_)) {
        highlight_border(frame, steps);
    }
    crate::scrollbar::vertical(
        frame,
        steps,
        step_inner,
        session.prepared.plan.commits.len(),
        session.commits.offset(),
    );
    if matches!(session.state, State::Browsing) {
        hits.list(
            steps,
            step_inner,
            &(0..session.prepared.plan.commits.len()).collect::<Vec<_>>(),
            session.commits.offset(),
            crate::pointer::Target::CompositionCommits,
        );
    }
    let (message, units, preview) = detail_areas(detail);
    let message_text = session.selected_message().to_owned();
    if let State::Editing(editor) = &mut session.state {
        let panel = block(i18n::tr!(CommitMessageAlt));
        let inner = panel.inner(message);
        frame.render_widget(panel, message);
        highlight_border(frame, message);
        let lines =
            super::text_editor::render(frame, inner, editor, i18n::tr!(DescribeThisCommit), true);
        crate::scrollbar::vertical(frame, message, inner, lines, editor.scroll);
        hits.vertical(
            message,
            inner,
            lines.saturating_sub(inner.height as usize),
            editor.scroll,
            crate::pointer::ScrollTarget::CommitMessage,
        );
    } else {
        review::styled_text_view(
            frame,
            message,
            Text::from(message_text),
            &mut session.message_scroll,
            i18n::tr!(CommitMessageAlt),
            hits,
            crate::pointer::ScrollTarget::CommitMessage,
        );
    }
    let ids = session.ids().to_vec();
    let rows = ids
        .iter()
        .filter_map(|id| {
            session
                .prepared
                .snapshot
                .units
                .iter()
                .find(|unit| unit.id == *id)
        })
        .map(|unit| {
            ListItem::new(Line::from(vec![
                Span::styled(format!("{} ", unit.id), Style::default().fg(ACCENT)),
                Span::raw(safe(&unit.path.to_string_lossy())),
                Span::styled(
                    format!(
                        " · {}",
                        match unit.layer {
                            engine::composition::Layer::Staged => i18n::tr!(ChangesStaged),
                            engine::composition::Layer::Worktree => i18n::tr!(Worktree),
                        }
                    ),
                    Style::default().fg(MUTED),
                ),
            ]))
        })
        .collect::<Vec<_>>();
    let unit_panel = block(i18n::tr!(ComposeUnits));
    let unit_inner = unit_panel.inner(units);
    session.units.select(Some(session.unit));
    frame.render_stateful_widget(
        List::new(rows)
            .block(unit_panel)
            .highlight_style(selected_style()),
        units,
        &mut session.units,
    );
    if session.focus == Focus::Units && !matches!(session.state, State::Editing(_)) {
        highlight_border(frame, units);
    }
    crate::scrollbar::vertical(frame, units, unit_inner, ids.len(), session.units.offset());
    if matches!(session.state, State::Browsing) {
        hits.list(
            units,
            unit_inner,
            &(0..ids.len()).collect::<Vec<_>>(),
            session.units.offset(),
            crate::pointer::Target::CompositionUnits,
        );
    }
    let patch = session
        .selected_unit()
        .map(|unit| unit.preview.clone())
        .unwrap_or_default();
    let title = session
        .selected_unit()
        .map(|unit| format!("{} · {}", unit.id, unit.path.display()))
        .unwrap_or_default();
    let lines = patch
        .lines()
        .map(|line| {
            Line::from(Span::styled(
                safe(line),
                Style::default().fg(if line.starts_with('+') && !line.starts_with("+++") {
                    GOOD
                } else if line.starts_with('-') && !line.starts_with("---") {
                    BAD
                } else if line.starts_with("@@") {
                    ACCENT
                } else {
                    Color::Reset
                }),
            ))
        })
        .collect::<Vec<_>>();
    review::styled_text_view(
        frame,
        preview,
        Text::from(lines),
        &mut session.scroll,
        &title,
        hits,
        crate::pointer::ScrollTarget::Text,
    );
}
