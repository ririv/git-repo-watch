//! Render remote selection without consuming the editable target-branch field.
use super::*;
pub(super) fn target(
    frame: &mut Frame,
    area: Rect,
    session: &crate::push::Session,
    pointer: &mut crate::pointer::Map,
) {
    let panel = block(i18n::tr!(PushTargetTitle));
    let inner = panel.inner(area);
    frame.render_widget(panel, area);
    let mut lines = vec![
        Line::from(vec![
            Span::styled(
                format!("{}: ", i18n::tr!(Branch)),
                Style::default().fg(MUTED),
            ),
            Span::raw(safe(&session.request.branch)),
        ]),
        Line::default(),
        Line::styled(i18n::tr!(Remote), Style::default().fg(ACCENT)),
    ];
    let visible = usize::from(inner.height.saturating_sub(4));
    let total = session.request.remotes().count();
    let offset = session
        .selected
        .saturating_sub(visible.saturating_sub(1))
        .min(total.saturating_sub(visible));
    for (index, remote) in session
        .request
        .remotes()
        .enumerate()
        .skip(offset)
        .take(visible)
    {
        let line = Line::from(vec![
            Span::styled(
                format!("{}  ", safe(&remote.name)),
                Style::default().fg(ACCENT),
            ),
            Span::styled(safe(&remote.url), Style::default().fg(MUTED)),
        ]);
        let line = if index == session.selected {
            line.style(selected_style())
        } else {
            line
        };
        let y = inner.y + lines.len() as u16;
        if y < inner.bottom() && session.editing() {
            pointer.button(
                Rect::new(inner.x, y, inner.width, 1),
                crate::pointer::Action::PushRemote(index),
            );
        }
        lines.push(line);
    }
    lines.push(Line::styled(
        i18n::tr!(PushSetUpstream),
        Style::default().fg(MUTED),
    ));
    frame.render_widget(Paragraph::new(lines), inner);
}
