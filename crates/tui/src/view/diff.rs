//! Draw a shared scroll viewport with fixed version labels and cached aligned rows.
use super::*;
use crate::diff::{Layout as DiffLayout, Preference, VisualRow};

pub(super) fn render(
    frame: &mut Frame,
    area: Rect,
    content: &mut crate::diff::View,
    scroll: &mut usize,
    preference: Preference,
) {
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
    content.arrange(inner.width, preference, scroll);
    let horizontal = content.horizontal.clone();
    let gutter = content.gutter_width(mode);
    match mode {
        DiffLayout::Unified => {
            horizontal.render_on_border(frame, area, body.x + gutter..body.right())
        }
        DiffLayout::Split => {
            horizontal.render_on_border(frame, area, body.x + gutter..body.x + left_width);
            horizontal.render_on_border(
                frame,
                area,
                body.x + left_width + 1 + gutter..body.right(),
            );
        }
    }
    let offset = horizontal.offset();
    let cache = content.arrange(inner.width, preference, scroll);
    *scroll = (*scroll).min(cache.rows.len().saturating_sub(body.height as usize));
    cache.settle(*scroll);
    for (index, row) in cache
        .rows
        .iter()
        .skip(*scroll)
        .take(body.height as usize)
        .enumerate()
    {
        let y = body.y + index as u16;
        match row {
            VisualRow::Full { line, fixed, .. } => frame.render_widget(
                crate::horizontal::pinned(line, *fixed, offset, body.width as usize),
                Rect::new(body.x, y, body.width, 1),
            ),
            VisualRow::Columns {
                left, right, fixed, ..
            } => {
                frame.render_widget(
                    crate::horizontal::pinned(left, *fixed, offset, left_width as usize),
                    Rect::new(body.x, y, left_width, 1),
                );
                frame.render_widget(
                    Line::styled("│", Style::default().fg(MUTED)),
                    Rect::new(body.x + left_width, y, 1, 1),
                );
                frame.render_widget(
                    crate::horizontal::pinned(right, *fixed, offset, right_width as usize),
                    Rect::new(body.x + left_width + 1, y, right_width, 1),
                );
            }
        }
    }
}
