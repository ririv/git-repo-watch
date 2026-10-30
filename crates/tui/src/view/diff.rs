//! Draw a shared scroll viewport with fixed version labels and cached aligned rows.
mod information;
use super::*;
use crate::diff::{Layout as DiffLayout, Preference, VisualRow};

fn separator_style() -> Style {
    Style::default().fg(MUTED).remove_modifier(Modifier::BOLD)
}

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
    let selection = match &content.interaction {
        crate::diff::Interaction::Editable(Some(marking)) => format!(" · {}", marking.label),
        _ => String::new(),
    };
    let border = block(format!("{} · {mode_label}{selection}", i18n::tr!(FileDiff)));
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
    let body = information::render(frame, inner, content, mode, hits);
    let left_width = inner.width.saturating_sub(1) / 2;
    let right_width = inner.width.saturating_sub(1) - left_width;
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
    let ids = (0..content.rows_len())
        .map(|row| content.line_id(row))
        .collect::<Vec<_>>();
    let (editable, marking) = match &mut content.interaction {
        crate::diff::Interaction::ReadOnly => (false, None),
        crate::diff::Interaction::Editable(marking) => (true, marking.clone()),
    };
    if let crate::diff::Interaction::Editable(Some(marking)) = &mut content.interaction {
        marking.reveal = false;
    }
    let cache = content.arrange(
        inner.width,
        preference,
        options.wrap,
        &hits.links.paths,
        scroll,
    );
    if let Some(marking) = &marking
        && marking.reveal
        && let Some(row) = ids.iter().position(|id| *id == Some(marking.cursor))
    {
        let position = cache.locate(crate::diff::Anchor { row });
        if position < *scroll {
            *scroll = position;
        } else if position >= scroll.saturating_add(body.height as usize) {
            *scroll = position
                .saturating_add(1)
                .saturating_sub(body.height as usize);
        }
    }
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
            VisualRow::Full {
                line,
                fixed,
                anchor,
            } => {
                let area = Rect::new(body.x, y, body.width, 1);
                hits.links.line(frame, area, line, *fixed, offset);
                selected_line(
                    frame,
                    hits,
                    area,
                    *fixed,
                    ids[anchor.row],
                    editable,
                    marking.as_ref(),
                );
            }
            VisualRow::Columns {
                left,
                right,
                fixed,
                anchors,
            } => {
                hits.links.line(
                    frame,
                    Rect::new(body.x, y, left_width, 1),
                    left,
                    *fixed,
                    offset,
                );
                frame.render_widget(
                    Line::styled("│", separator_style()),
                    Rect::new(body.x + left_width, y, 1, 1),
                );
                hits.links.line(
                    frame,
                    Rect::new(body.x + left_width + 1, y, right_width, 1),
                    right,
                    *fixed,
                    offset,
                );
                let (old, new) = match anchors {
                    crate::diff::Anchors::Pair(old, new) => (ids[old.row], ids[new.row]),
                    crate::diff::Anchors::One(anchor) => match ids[anchor.row] {
                        id @ Some(engine::local::files::partial::LineId::Old(_)) => (id, None),
                        id @ Some(engine::local::files::partial::LineId::New(_)) => (None, id),
                        None => (None, None),
                    },
                };
                let old = old.filter(|_| left.width() > 0);
                let new = new.filter(|_| right.width() > 0);
                selected_line(
                    frame,
                    hits,
                    Rect::new(body.x, y, left_width, 1),
                    *fixed,
                    old,
                    editable,
                    marking.as_ref(),
                );
                selected_line(
                    frame,
                    hits,
                    Rect::new(body.x + left_width + 1, y, right_width, 1),
                    *fixed,
                    new,
                    editable,
                    marking.as_ref(),
                );
            }
        }
    }
}

fn selected_line(
    frame: &mut Frame,
    hits: &mut crate::pointer::Map,
    area: Rect,
    fixed: usize,
    id: Option<engine::local::files::partial::LineId>,
    editable: bool,
    marking: Option<&crate::diff::Marking>,
) {
    let Some(id) = id else {
        return;
    };
    if editable {
        hits.button(area, crate::pointer::Action::DiffLine(id));
    }
    if marking.is_some_and(|marking| marking.lines.contains(&id))
        && fixed >= 2
        && fixed <= usize::from(area.width)
    {
        let separator = area.x + fixed as u16 - 2;
        let style = selected_style().fg(Color::LightBlue);
        for x in area.x..=separator {
            frame.buffer_mut()[(x, area.y)].set_style(style);
        }
        frame.buffer_mut()[(separator, area.y)]
            .set_symbol("│")
            .set_style(style);
    }
}
