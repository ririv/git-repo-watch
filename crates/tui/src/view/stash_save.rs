//! Stash mode and reviewed files stay above the optional message on one page.
use super::*;
use crate::confirmation::Choice;
use crate::stash_save::{Focus, Session};

pub(crate) fn actions(area: Rect) -> Vec<(Rect, Choice)> {
    let padding = u16::from(area.width > 1);
    let mut x = area.x.saturating_add(padding);
    [Choice::Execute, Choice::Return]
        .into_iter()
        .map(|choice| {
            let label = if choice == Choice::Execute {
                i18n::tr!(ActionSave)
            } else {
                i18n::tr!(ActionCancel)
            };
            let width = (unicode_width::UnicodeWidthStr::width(label) as u16 + 2)
                .min(area.right().saturating_sub(x));
            let rect = Rect::new(x.min(area.right()), area.y, width, area.height.min(1));
            x = x.saturating_add(width + 2).min(area.right());
            (rect, choice)
        })
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
    if session.focus == Focus::Message {
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
    if session.focus == Focus::Message {
        frame.set_cursor_position(((inner.x + cursor).min(inner.right() - 1), inner.y));
    }
}
