//! Conflict display modes filter source panes within the existing workspace.
mod result;
use super::*;
use crate::{
    conflict::{DisplayMode, Focus, View, layout},
    pointer::ScrollTarget,
};
use engine::local::files::conflict::Role;

pub(crate) struct Areas {
    pub comparison: Rect,
    pub result: Option<Rect>,
}
pub(crate) fn areas(area: Rect, mode: DisplayMode) -> Areas {
    if matches!(mode, DisplayMode::Pair(_)) {
        Areas {
            comparison: area,
            result: None,
        }
    } else {
        let parts =
            Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)]).split(area);
        Areas {
            comparison: parts[0],
            result: Some(parts[1]),
        }
    }
}
pub(crate) fn focus_at(
    area: Rect,
    mode: DisplayMode,
    point: ratatui::layout::Position,
) -> Option<Focus> {
    let areas = areas(area, mode);
    if areas.comparison.contains(point) {
        Some(Focus::Compare)
    } else if areas.result.is_some_and(|r| r.contains(point)) {
        Some(Focus::Result)
    } else {
        None
    }
}
pub(crate) fn highlight(frame: &mut Frame, area: Rect, mode: DisplayMode, focus: Focus) {
    let areas = areas(area, mode);
    match focus {
        Focus::Compare => highlight_border(frame, areas.comparison),
        Focus::Result => {
            if let Some(result) = areas.result {
                highlight_border(frame, result)
            }
        }
    }
}

pub(super) enum ResultTarget {
    Pane(Rect),
    Heading(Rect),
}

impl ResultTarget {
    pub(super) fn highlight(self, frame: &mut Frame, accent: Color) {
        match self {
            Self::Pane(area) => highlight_border_with(frame, area, accent),
            Self::Heading(area) => frame
                .buffer_mut()
                .set_style(area, Style::default().fg(accent)),
        }
    }
}

pub(super) fn result_target(area: Rect, mode: DisplayMode) -> Option<ResultTarget> {
    let areas = areas(area, mode);
    if let Some(result) = areas.result {
        return (!result.is_empty()).then_some(ResultTarget::Pane(result));
    }
    let content = block("").inner(areas.comparison);
    let (panel, _) = panels(content, layout::Arrangement::Columns, mode)
        .into_iter()
        .find(|(panel, index)| *index == 3 && !panel.is_empty())?;
    Some(ResultTarget::Heading(Rect::new(
        panel.x,
        panel.y,
        panel.width,
        1,
    )))
}

pub(super) fn render(
    frame: &mut Frame,
    area: Rect,
    view: &mut View,
    scroll: &mut usize,
    options: crate::diff::Options,
    graphics: &mut crate::graphics::Display,
    hits: &mut crate::pointer::Map,
) {
    let areas = areas(area, view.mode);
    let mut positions = view.scrolls(*scroll);
    let compare_slot = view.mode.comparison_slot();
    comparison(
        frame,
        areas.comparison,
        view,
        &mut positions[compare_slot],
        options,
        hits,
    );
    if let Some(area) = areas.result {
        let label = view.resolution.label();
        let (working, selected, resolution) = view.result_review();
        result::render(
            frame,
            area,
            result::Review {
                view: working,
                selected,
                label: &label,
                resolution,
            },
            &mut positions[1],
            options,
            graphics,
            hits,
        );
    }
    view.settle(positions, scroll);
}

fn comparison(
    frame: &mut Frame,
    area: Rect,
    view: &mut View,
    scroll: &mut usize,
    options: crate::diff::Options,
    hits: &mut crate::pointer::Map,
) {
    let border = block(format!(
        "{} · {} · {}",
        i18n::tr!(ConflictCompare),
        view.resolution.label(),
        safe(&view.title)
    ));
    let content = border.inner(area);
    frame.render_widget(border, area);
    if content.is_empty() {
        return;
    }
    let arrangement = if matches!(view.mode, DisplayMode::Pair(_))
        || content.width >= 70
        || content.height < (view.mode.indices().len() as u16 * 4)
    {
        layout::Arrangement::Columns
    } else {
        layout::Arrangement::Rows
    };
    let panels = panels(content, arrangement, view.mode);
    let visible = panels
        .iter()
        .map(|(p, _)| p.height.saturating_sub(2) as usize)
        .min()
        .unwrap_or(0);
    view.arrange(content.width, options.wrap, arrangement, visible, scroll);
    let cache = view.cache.as_ref().expect("arranged stage rows");
    let maximum = cache.rows.len().saturating_sub(visible);
    *scroll = (*scroll).min(maximum);
    for (panel, _) in panels.iter().skip(1) {
        if panel.x > content.x {
            for y in panel.y..panel.bottom() {
                frame.render_widget(
                    Line::styled("│", Style::default().fg(MUTED)),
                    Rect::new(panel.x - 1, y, 1, 1),
                );
            }
        } else if panel.y > content.y {
            frame.render_widget(
                Paragraph::new("─".repeat(content.width as usize))
                    .style(Style::default().fg(MUTED)),
                Rect::new(content.x, panel.y - 1, content.width, 1),
            );
        }
    }
    for (panel, n) in panels.iter().copied() {
        let body = Rect::new(
            panel.x,
            panel.y + panel.height.min(2),
            panel.width,
            panel.height.saturating_sub(2),
        );
        let (label, color) = if n < 3 {
            let column = &view.columns[n];
            (
                column.role.label(),
                match column.role {
                    Role::Base => Color::White,
                    Role::Current => Color::LightBlue,
                    Role::Incoming => Color::LightCyan,
                },
            )
        } else {
            (i18n::tr!(ConflictResult), Color::White)
        };
        if panel.height > 0 {
            frame.render_widget(
                Paragraph::new(label)
                    .style(Style::default().fg(color).add_modifier(Modifier::BOLD)),
                Rect::new(panel.x, panel.y, panel.width, 1),
            );
        }
        if panel.height > 1 {
            frame.render_widget(
                Paragraph::new(safe(view.description(n))).style(Style::default().fg(MUTED)),
                Rect::new(panel.x, panel.y + 1, panel.width, 1),
            );
        }
        for (row, cells) in cache
            .rows
            .iter()
            .skip(*scroll)
            .take(body.height as usize)
            .enumerate()
        {
            let row = Rect::new(body.x, body.y + row as u16, body.width, 1);
            let line = &cells.cells[n];
            frame.buffer_mut().set_style(row, line.text.style);
            hits.links.line(
                frame,
                row,
                line,
                if cells.kind == crate::diff::RowKind::Annotation {
                    line.width().min(body.width as usize)
                } else {
                    view.gutter(n).min(body.width as usize)
                },
                view.horizontal.offset().min(cache.maxima[n]),
            );
        }
        for (row, at) in cache
            .anchors
            .iter()
            .skip(*scroll)
            .take(body.height as usize)
            .enumerate()
        {
            if let Some(index) = view
                .coordinate(n, *at)
                .line()
                .and_then(|line| view.block_at(n, line))
            {
                hits.button(
                    Rect::new(body.x, body.y + row as u16, body.width, 1),
                    crate::pointer::Action::ConflictBlock {
                        index,
                        focus: Focus::Compare,
                    },
                );
            }
            if view
                .coordinate(n, *at)
                .line()
                .is_some_and(|line| view.selected_line(n, line))
            {
                frame.buffer_mut().set_style(
                    Rect::new(
                        body.x,
                        body.y + row as u16,
                        view.gutter(n).min(body.width as usize) as u16,
                        1,
                    ),
                    Style::default().bg(SELECTION).fg(Color::White).bold(),
                );
            }
        }
        let x = if panel.right() < content.right() {
            panel.right()
        } else {
            area.right().saturating_sub(1)
        };
        let vertical = Rect::new(x, body.y, 1, body.height);
        crate::scrollbar::render(
            frame,
            vertical,
            crate::scrollbar::Axis::Vertical,
            *scroll,
            maximum,
        );
        hits.track(
            vertical,
            crate::scrollbar::Axis::Vertical,
            maximum,
            *scroll,
            ScrollTarget::Conflict(Focus::Compare),
        );
        let y = if panel.bottom() < content.bottom() {
            panel.bottom()
        } else {
            area.bottom().saturating_sub(1)
        };
        let start = panel.x + view.gutter(n).min(panel.width as usize) as u16;
        let horizontal = Rect::new(start, y, panel.right().saturating_sub(start), 1);
        crate::scrollbar::render(
            frame,
            horizontal,
            crate::scrollbar::Axis::Horizontal,
            view.horizontal.offset().min(cache.maxima[n]),
            cache.maxima[n],
        );
        hits.track(
            horizontal,
            crate::scrollbar::Axis::Horizontal,
            cache.maxima[n],
            view.horizontal.offset().min(cache.maxima[n]),
            ScrollTarget::ConflictHorizontal(Focus::Compare),
        );
    }
}
fn panels(area: Rect, arrangement: layout::Arrangement, mode: DisplayMode) -> Vec<(Rect, usize)> {
    let mut result = Vec::new();
    if let DisplayMode::Pair(pair) = mode {
        let [left, right] = *pair.indices();
        let first = area.width.saturating_sub(1) / 2;
        result.push((Rect::new(area.x, area.y, first, area.height), left));
        let next = Rect::new(
            area.x.saturating_add(first).saturating_add(1),
            area.y,
            area.width.saturating_sub(first).saturating_sub(1),
            area.height,
        )
        .intersection(area);
        result.push((next, right));
        return result;
    }
    let inputs = if mode == DisplayMode::IncludeBase {
        let height = area.height.saturating_sub(1) * 2 / 5;
        result.push((Rect::new(area.x, area.y, area.width, height), 0));
        Rect::new(
            area.x,
            area.y.saturating_add(height).saturating_add(1),
            area.width,
            area.height.saturating_sub(height).saturating_sub(1),
        )
        .intersection(area)
    } else {
        area
    };
    let sizes = layout::widths(
        if arrangement == layout::Arrangement::Columns {
            inputs.width
        } else {
            inputs.height
        },
        layout::Arrangement::Columns,
        DisplayMode::Overview,
    );
    let mut offset = 0;
    for n in [1, 2] {
        let panel = if arrangement == layout::Arrangement::Columns {
            Rect::new(
                inputs.x.saturating_add(offset),
                inputs.y,
                sizes[n],
                inputs.height,
            )
        } else {
            Rect::new(
                inputs.x,
                inputs.y.saturating_add(offset),
                inputs.width,
                sizes[n],
            )
        }
        .intersection(inputs);
        offset = offset.saturating_add(sizes[n]).saturating_add(1);
        result.push((panel, n));
    }
    result
}
