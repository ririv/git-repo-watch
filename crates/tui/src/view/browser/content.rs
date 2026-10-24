//! A preview slot draws its own text, image, loading state and scrollbars.
use super::*;
use crate::{
    browser::{Preview, preview::Document},
    pointer::ScrollTarget,
};
pub(super) fn render(
    frame: &mut Frame,
    layout: &layout::Pane,
    pane: &mut crate::browser::panes::Pane,
    feedback: (&str, &str),
    graphics: &mut crate::graphics::Display,
    hits: &mut crate::pointer::Map,
) {
    let area = layout.frame;
    let inner = layout.content;
    let mode = layout.mode;
    let (spinner, notice) = feedback;
    let position = &mut pane.position;
    match &mut pane.preview {
        Preview::Ready {
            document:
                Document::Text {
                    text,
                    width,
                    gutter,
                },
            ..
        } => {
            position
                .horizontal
                .constrain(width.saturating_sub(inner.width as usize));
            position.scroll = position
                .scroll
                .min(text.lines.len().saturating_sub(inner.height as usize));
            for (index, line) in text
                .lines
                .iter()
                .skip(position.scroll)
                .take(inner.height as usize)
                .enumerate()
            {
                frame.render_widget(
                    crate::horizontal::pinned(
                        line,
                        (*gutter).min(inner.width as usize),
                        position.horizontal.offset(),
                        inner.width as usize,
                    ),
                    Rect::new(inner.x, inner.y + index as u16, inner.width, 1),
                );
            }
            crate::scrollbar::vertical(frame, area, inner, text.lines.len(), position.scroll);
            hits.vertical(
                area,
                inner,
                text.lines.len().saturating_sub(inner.height as usize),
                position.scroll,
                ScrollTarget::BrowserPreview(mode),
            );
            position.horizontal.render_on_border(
                frame,
                area,
                inner.x + (*gutter as u16).min(inner.width)..inner.right(),
                hits,
                ScrollTarget::BrowserHorizontal(mode),
            );
        }
        Preview::Ready {
            document: Document::Image(image),
            ..
        } => {
            super::super::document::content(
                frame,
                inner,
                &image.pairs[0].after,
                image.source,
                0,
                graphics,
            );
        }
        Preview::Loading(_) => frame.render_widget(
            Paragraph::new(format!("{} {}", spinner, i18n::tr!(BrowserReading)))
                .style(Style::default().fg(MUTED)),
            inner,
        ),
        Preview::Failed { error, .. } => frame.render_widget(
            Paragraph::new(multiline(error))
                .wrap(Wrap { trim: false })
                .style(Style::default().fg(BAD)),
            inner,
        ),
        Preview::Empty => {
            frame.render_widget(
                Paragraph::new(multiline(notice))
                    .wrap(Wrap { trim: false })
                    .style(Style::default().fg(MUTED)),
                inner,
            );
        }
    }
}
