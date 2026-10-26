//! A preview slot draws its own text, image, loading state and scrollbars.
use super::*;
use crate::{
    browser::{Preview, preview::Document},
    pointer::ScrollTarget,
};
pub(super) struct View<'a> {
    pub preview: &'a Preview,
    pub position: &'a mut crate::browser::opening::Position,
    pub layout: &'a mut crate::flow::Cache,
}
pub(super) struct Options<'a> {
    pub spinner: &'a str,
    pub notice: &'a str,
    pub visible: bool,
    pub wrap: bool,
}
pub(super) fn render(
    frame: &mut Frame,
    layout: &layout::Pane,
    view: View<'_>,
    options: Options<'_>,
    graphics: &mut crate::graphics::Display,
    hits: &mut crate::pointer::Map,
) {
    let View {
        preview,
        position,
        layout: text_layout,
    } = view;
    let area = layout.frame;
    let inner = layout.content;
    let mode = layout.mode;
    let Options {
        spinner,
        notice,
        visible: loading_visible,
        wrap,
    } = options;
    match preview {
        Preview::Ready { document, .. } => match document.as_ref() {
            Document::Text {
                text,
                width,
                gutter,
            } => {
                position.horizontal.constrain(if wrap {
                    0
                } else {
                    width.saturating_sub(inner.width as usize)
                });
                let layout = text_layout.arrange(
                    text,
                    inner.width as usize,
                    *gutter,
                    wrap,
                    &mut position.scroll,
                );
                position.scroll = position
                    .scroll
                    .min(layout.rows.len().saturating_sub(inner.height as usize));
                for (index, row) in layout
                    .rows
                    .iter()
                    .skip(position.scroll)
                    .take(inner.height as usize)
                    .enumerate()
                {
                    hits.links.line(
                        frame,
                        Rect::new(inner.x, inner.y + index as u16, inner.width, 1),
                        &row.line,
                        (*gutter).min(inner.width as usize),
                        position.horizontal.offset(),
                    );
                }
                crate::scrollbar::vertical(frame, area, inner, layout.rows.len(), position.scroll);
                hits.vertical(
                    area,
                    inner,
                    layout.rows.len().saturating_sub(inner.height as usize),
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
            Document::Image(image) => {
                super::super::document::content(
                    frame,
                    inner,
                    &image.pairs[0].after,
                    image.source,
                    0,
                    graphics,
                );
            }
        },
        Preview::Loading(_) if loading_visible => frame.render_widget(
            Paragraph::new(format!("{} {}", spinner, i18n::tr!(BrowserReading)))
                .style(Style::default().fg(MUTED)),
            inner,
        ),
        Preview::Loading(_) => {}
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
