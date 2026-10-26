//! Draw a shared scroll viewport with fixed version labels and cached aligned rows.
use super::*;
use crate::diff::{Layout as DiffLayout, Preference, VisualRow};

pub(super) fn render(
    frame: &mut Frame,
    area: Rect,
    content: &mut crate::diff::View,
    scroll: &mut usize,
    options: crate::diff::Options,
    hits: &mut crate::pointer::Map,
) {
    let preference = options.layout;
    let width = area.width.saturating_sub(2);
    let mode = content.selected_layout(width, preference);
    let mode_label = if preference == Preference::Auto {
        format!("{}: {}", preference.label(), mode.label())
    } else {
        mode.label().into()
    };
    let border = block(format!("{} · {mode_label}", i18n::tr!(FileDiff)));
    let inner = border.inner(area);
    frame.render_widget(border, area);
    if inner.width < content.minimum_width(mode) {
        frame.render_widget(
            Paragraph::new(i18n::tf!(DiffTooNarrow, content.minimum_width(mode)))
                .wrap(Wrap { trim: false }),
            inner,
        );
        return;
    }
    let (before, after) = content.labels();
    let header = Rect::new(inner.x, inner.y, inner.width, inner.height.min(1));
    let body = Rect::new(
        inner.x,
        inner.y + header.height,
        inner.width,
        inner.height - header.height,
    );
    let left_width = inner.width.saturating_sub(1) / 2;
    let right_width = inner.width.saturating_sub(1) - left_width;
    let header_style = Style::default().fg(Color::LightBlue).bold();
    match mode {
        DiffLayout::Unified => frame.render_widget(
            Line::styled(format!("{before} → {after}"), header_style),
            header,
        ),
        DiffLayout::Split => {
            frame.render_widget(
                Line::styled(i18n::tf!(DiffBefore, before), header_style),
                Rect::new(header.x, header.y, left_width, header.height),
            );
            frame.render_widget(
                Line::styled(i18n::tf!(DiffAfter, after), header_style),
                Rect::new(
                    header.x + left_width + 1,
                    header.y,
                    right_width,
                    header.height,
                ),
            );
            frame.render_widget(
                Line::styled("│", Style::default().fg(MUTED)),
                Rect::new(header.x + left_width, header.y, 1, header.height),
            );
        }
    }
    content.arrange(
        inner.width,
        preference,
        options.wrap,
        &hits.links.paths,
        scroll,
    );
    let horizontal = content.horizontal.clone();
    let gutter = content.gutter_width(mode);
    match mode {
        DiffLayout::Unified => horizontal.render_on_border(
            frame,
            area,
            body.x + gutter..body.right(),
            hits,
            crate::pointer::ScrollTarget::DiffHorizontal,
        ),
        DiffLayout::Split => {
            horizontal.render_on_border(
                frame,
                area,
                body.x + gutter..body.x + left_width,
                hits,
                crate::pointer::ScrollTarget::DiffHorizontal,
            );
            horizontal.render_on_border(
                frame,
                area,
                body.x + left_width + 1 + gutter..body.right(),
                hits,
                crate::pointer::ScrollTarget::DiffHorizontal,
            );
        }
    }
    let offset = horizontal.offset();
    let cache = content.arrange(
        inner.width,
        preference,
        options.wrap,
        &hits.links.paths,
        scroll,
    );
    *scroll = (*scroll).min(cache.rows.len().saturating_sub(body.height as usize));
    cache.settle(*scroll);
    crate::scrollbar::vertical(frame, area, body, cache.rows.len(), *scroll);
    hits.vertical(
        area,
        body,
        cache.rows.len().saturating_sub(body.height as usize),
        *scroll,
        crate::pointer::ScrollTarget::Diff,
    );
    for (index, row) in cache
        .rows
        .iter()
        .skip(*scroll)
        .take(body.height as usize)
        .enumerate()
    {
        let y = body.y + index as u16;
        match row {
            VisualRow::Full { line, fixed, .. } => hits.links.line(
                frame,
                Rect::new(body.x, y, body.width, 1),
                line,
                *fixed,
                offset,
            ),
            VisualRow::Columns {
                left, right, fixed, ..
            } => {
                hits.links.line(
                    frame,
                    Rect::new(body.x, y, left_width, 1),
                    left,
                    *fixed,
                    offset,
                );
                frame.render_widget(
                    Line::styled("│", Style::default().fg(MUTED)),
                    Rect::new(body.x + left_width, y, 1, 1),
                );
                hits.links.line(
                    frame,
                    Rect::new(body.x + left_width + 1, y, right_width, 1),
                    right,
                    *fixed,
                    offset,
                );
            }
        }
    }
}
