//! Recovery choices render inside the existing input surface.
use super::super::*;
use crate::replay::controls;
use engine::operations::replay::Control;
use unicode_width::UnicodeWidthStr;

fn options(area: Rect, selected: Option<Control>) -> Vec<(Rect, Control)> {
    let selected = selected.unwrap_or(Control::Continue);
    let target = controls::ACTIONS
        .iter()
        .position(|value| *value == selected)
        .unwrap();
    let widths = controls::ACTIONS.map(|control| controls::label(control).width() as u16 + 2);
    let mut first = 0;
    while first < target
        && widths[first..=target].iter().sum::<u16>() + (target - first) as u16 > area.width
    {
        first += 1;
    }
    let mut x = area.x;
    controls::ACTIONS
        .iter()
        .copied()
        .enumerate()
        .skip(first)
        .filter_map(|(index, control)| {
            let width = widths[index].min(area.right().saturating_sub(x));
            if width == 0 {
                return None;
            }
            let button = Rect::new(x, area.y, width, area.height.min(1));
            x = x.saturating_add(width + 1);
            Some((button, control))
        })
        .collect()
}
pub(in crate::view) fn render(
    frame: &mut Frame,
    inner: Rect,
    selected: Control,
    enabled: [bool; 3],
    hits: &mut crate::pointer::Map,
    accent: Color,
) {
    let buttons = options(inner, Some(selected));
    for (button, control) in &buttons {
        let index = controls::ACTIONS
            .iter()
            .position(|value| value == control)
            .unwrap();
        let style = if selected == *control {
            Style::default()
                .fg(if enabled[index] { Color::Black } else { MUTED })
                .bg(accent)
                .bold()
        } else {
            Style::default().fg(if enabled[index] { Color::Gray } else { MUTED })
        };
        frame.render_widget(
            Paragraph::new(format!(" {} ", controls::label(*control))).style(style),
            *button,
        );
        if enabled[index] {
            hits.button(*button, crate::pointer::Action::ReplayControl(*control));
        }
    }
    if let Some((last, _)) = buttons.last() {
        let start = last.right().saturating_add(2);
        if selected == Control::Continue && !enabled[0] {
            frame.render_widget(
                Paragraph::new(i18n::tr!(ReplayResolveFirst)).style(Style::default().fg(MUTED)),
                Rect::new(
                    start.min(inner.right()),
                    inner.y,
                    inner.right().saturating_sub(start),
                    inner.height.min(1),
                ),
            );
        }
    }
}
