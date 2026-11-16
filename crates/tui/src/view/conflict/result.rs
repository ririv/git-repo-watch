//! Render the actual working source independently from the immutable input comparison.
use super::*;
use crate::conflict::result::{Content, View};

pub(super) struct Review<'a> {
    pub view: &'a mut View,
    pub selected: &'a std::collections::BTreeSet<usize>,
    pub label: &'a str,
    pub resolution: &'a crate::conflict::resolution::Session,
}
pub(super) fn render(
    frame: &mut Frame,
    area: Rect,
    review: Review<'_>,
    scroll: &mut usize,
    options: crate::diff::Options,
    graphics: &mut crate::graphics::Display,
    hits: &mut crate::pointer::Map,
) {
    let Review {
        view,
        selected,
        label,
        resolution,
    } = review;
    if let Content::Document(document) = &mut view.content {
        super::super::document::render(frame, area, document, scroll, options, graphics, hits);
        return;
    }
    let border = block(format!("{} · {}", i18n::tr!(ConflictResult), label));
    let inner = border.inner(area);
    frame.render_widget(border, area);
    if inner.is_empty() {
        return;
    }
    let Content::Source { syntax, .. } = &view.content else {
        unreachable!()
    };
    frame.render_widget(
        Paragraph::new(syntax.clone()).style(Style::default().fg(MUTED)),
        Rect::new(inner.x, inner.y, inner.width, 1),
    );
    let body = Rect::new(
        inner.x,
        inner.y + 1,
        inner.width,
        inner.height.saturating_sub(1),
    );
    view.arrange(body.width, options.wrap, body.height as usize, scroll);
    let gutter = view.gutter();
    let cache = view.cache.as_ref().expect("arranged working result");
    let maximum = cache.rows.len().saturating_sub(body.height as usize);
    *scroll = (*scroll).min(maximum);
    for (index, row) in cache
        .rows
        .iter()
        .skip(*scroll)
        .take(body.height as usize)
        .enumerate()
    {
        let fixed = if row.kind == crate::diff::RowKind::Annotation {
            row.line.width()
        } else {
            gutter
        };
        hits.links.line(
            frame,
            Rect::new(body.x, body.y + index as u16, body.width, 1),
            &row.line,
            fixed.min(body.width as usize),
            view.horizontal.offset(),
        );
    }
    for (row, line) in cache
        .anchors
        .iter()
        .skip(*scroll)
        .take(body.height as usize)
        .enumerate()
    {
        if let Some(index) = resolution.block_index(*line) {
            hits.button(
                Rect::new(body.x, body.y + row as u16, body.width, 1),
                crate::pointer::Action::ConflictBlock {
                    index,
                    focus: Focus::Result,
                },
            );
        }
        if selected.contains(line) {
            frame.buffer_mut().set_style(
                Rect::new(
                    body.x,
                    body.y + row as u16,
                    gutter.min(body.width as usize) as u16,
                    1,
                ),
                Style::default().bg(SELECTION).fg(Color::White).bold(),
            );
        }
    }
    crate::scrollbar::vertical(frame, area, body, cache.rows.len(), *scroll);
    hits.vertical(
        area,
        body,
        maximum,
        *scroll,
        ScrollTarget::Conflict(Focus::Result),
    );
    view.horizontal.render_on_border(
        frame,
        area,
        body.x + gutter.min(body.width as usize) as u16..body.right(),
        hits,
        ScrollTarget::ConflictHorizontal(Focus::Result),
    );
}
