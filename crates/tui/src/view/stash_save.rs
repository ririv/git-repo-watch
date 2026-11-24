//! Stash mode and reviewed files stay above the optional message on one page.
use super::*;
use crate::confirmation::Choice;
use crate::stash_save::Session;

pub(super) fn action_row(focus: crate::stash_save::Focus) -> input::actions::Row {
    use crate::stash_save::Focus;
    use input::actions::{Item, Row, Spacing, State, Target};

    let items = [
        (Choice::Execute, i18n::tr!(ActionSave)),
        (Choice::Return, i18n::tr!(ActionCancel)),
    ]
    .into_iter()
    .map(|(choice, label)| Item {
        label,
        state: State::new(true, focus == Focus::Actions(choice)),
        target: Target::Button(crate::pointer::Action::StashFooter(choice)),
    })
    .collect();
    Row::new(items, Spacing::Indented)
}

#[cfg(test)]
pub(crate) fn actions(area: Rect) -> Vec<(Rect, Choice)> {
    action_row(crate::stash_save::Focus::Actions(Choice::Execute))
        .cells(area)
        .into_iter()
        .zip([Choice::Execute, Choice::Return])
        .map(|((area, _), choice)| (area, choice))
        .collect()
}

pub(crate) fn message_area(area: Rect) -> Rect {
    let height = area.height.min(3);
    Rect::new(
        area.x,
        area.bottom().saturating_sub(height),
        area.width,
        height,
    )
}
pub(super) fn render(
    frame: &mut Frame,
    area: Rect,
    session: &mut Session,
    hits: &mut crate::pointer::Map,
) {
    let message = message_area(area);
    let review = Rect::new(
        area.x,
        area.y,
        area.width,
        area.height.saturating_sub(message.height),
    );
    text_view(
        frame,
        review,
        &crate::stash_save::review(&session.plan),
        &mut session.scroll,
        i18n::tr!(StashSaveCommand),
        hits,
    );
    let panel =
        block(i18n::tr!(StashMessageOptional)).padding(ratatui::widgets::Padding::horizontal(1));
    let inner = panel.inner(message);
    frame.render_widget(panel, message);
    if session.focus.is_input() {
        highlight_border(frame, message);
    }
    if inner.is_empty() {
        return;
    }
    let (text, cursor) = session.message.viewport(inner.width.into());
    let span = if text.is_empty() {
        Span::styled(
            truncate(i18n::tr!(StashDefaultMessage), inner.width.into()),
            Style::default().fg(MUTED),
        )
    } else {
        Span::raw(text)
    };
    frame.render_widget(Paragraph::new(span), inner);
    if session.focus.is_input() {
        frame.set_cursor_position(((inner.x + cursor).min(inner.right() - 1), inner.y));
    }
}
