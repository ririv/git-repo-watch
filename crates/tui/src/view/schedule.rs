//! Each schedule value shares a row with its corresponding edit or control button.
use super::*;
use engine::scheduler::duration;
use unicode_width::UnicodeWidthStr;

mod entries;
use entries::{Interaction, entries};

pub(super) fn render(frame: &mut Frame, screen: &mut Screen, area: Rect) {
    let panel =
        block(i18n::tr!(ScheduleSettings)).padding(ratatui::widgets::Padding::horizontal(1));
    let inner = panel.inner(area);
    frame.render_widget(panel, area);
    if inner.is_empty() {
        return;
    }
    let entries = entries(screen);
    let label_width = entries
        .iter()
        .map(|entry| entry.label.width())
        .max()
        .expect("schedule has rows") as u16;
    let button_width = entries
        .iter()
        .filter_map(|entry| match entry.interaction {
            Interaction::Button { label, .. } => Some(label.width() as u16 + 4),
            Interaction::ReadOnly => None,
        })
        .max()
        .expect("schedule has actions")
        .min(inner.width);
    let inline = inner.width >= label_width + button_width + 14;
    let value_x = if inline { label_width + 2 } else { 0 };
    let value_width = if inline {
        inner.width.saturating_sub(value_x + button_width + 2)
    } else {
        inner.width
    };
    let value_offset = usize::from(!inline);
    let mut row = 0;
    let rows = entries
        .iter()
        .map(|entry| {
            let height = Paragraph::new(entry.value.as_str())
                .wrap(Wrap { trim: false })
                .line_count(value_width)
                .max(1)
                + value_offset;
            let first = row;
            row += height;
            (first, height)
        })
        .collect::<Vec<_>>();
    let count = Configuration::Schedule.actions(&screen.snapshot).len();
    screen.setting_selected = screen.setting_selected.min(count.saturating_sub(1));
    let focus = crate::settings::Focus {
        selected: screen.setting_selected,
        area: inner,
    };
    let mut start = screen.scroll;
    if screen.settings_focus != Some(focus)
        && let Some((first, height)) = entries.iter().zip(&rows).find_map(|(entry, row)| {
            matches!(entry.interaction, Interaction::Button { index, .. } if index == screen.setting_selected).then_some(*row)
        })
    {
        // 值超出整个视口时，优先露出同行按钮，避免选中项的操作停在屏外。
        if first < start || height > usize::from(inner.height) { start = first; }
        else if first + height > start + usize::from(inner.height) {
            start = (first + height).saturating_sub(usize::from(inner.height));
        }
    }
    start = start.min(row.saturating_sub(usize::from(inner.height)));
    screen.scroll = start;
    screen.settings_focus = Some(focus);
    let end = start + usize::from(inner.height);
    for (entry, (first, height)) in entries.iter().zip(rows) {
        let visible_first = first.max(start);
        let visible_end = (first + height).min(end);
        if visible_first >= visible_end {
            continue;
        }
        let selected = matches!(entry.interaction, Interaction::Button { index, .. } if index == screen.setting_selected);
        let base = if selected {
            selected_style()
        } else {
            Style::default()
        };
        let visible = Rect::new(
            inner.x,
            inner.y + (visible_first - start) as u16,
            inner.width,
            (visible_end - visible_first) as u16,
        );
        frame.render_widget(Block::default().style(base), visible);
        if let Interaction::Button { index, .. } = entry.interaction {
            screen
                .pointer
                .item(visible, crate::pointer::Target::Settings, index);
        }
        let value_first = first + value_offset;
        let value_visible_first = value_first.max(start);
        if value_visible_first < visible_end {
            frame.render_widget(
                Paragraph::new(entry.value.as_str())
                    .style(base.fg(entry.color))
                    .wrap(Wrap { trim: false })
                    .scroll(((value_visible_first - value_first) as u16, 0)),
                Rect::new(
                    inner.x + value_x,
                    inner.y + (value_visible_first - start) as u16,
                    value_width,
                    (visible_end - value_visible_first) as u16,
                ),
            );
        }
        if first < start || first >= end {
            continue;
        }
        let y = inner.y + (first - start) as u16;
        frame.render_widget(
            Paragraph::new(entry.label).style(base.fg(MUTED)),
            Rect::new(
                inner.x,
                y,
                if inline {
                    label_width
                } else {
                    inner.width.saturating_sub(button_width + 1)
                },
                1,
            ),
        );
        if let Interaction::Button { index, label } = entry.interaction {
            let width = (label.width() as u16 + 4).min(button_width);
            let button = Rect::new(inner.right() - button_width, y, width, 1);
            frame.render_widget(
                Paragraph::new(format!("[ {label} ]")).style(base.fg(ACCENT).bold()),
                button,
            );
            screen
                .pointer
                .item(button, crate::pointer::Target::ScheduleButton, index);
        }
    }
    screen.pointer.vertical(
        area,
        inner,
        row.saturating_sub(usize::from(inner.height)),
        start,
        crate::pointer::ScrollTarget::Text,
    );
    crate::scrollbar::vertical(frame, area, inner, row, start);
}
