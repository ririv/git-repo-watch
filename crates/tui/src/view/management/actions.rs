//! Scan controls share their layout with pointer hit regions.
use super::*;
use crate::management::{Action, Focus};

fn color(action: Action) -> Color {
    match action {
        Action::Back => Color::Reset,
        Action::Apply => ACCENT,
    }
}

pub(super) fn render(
    frame: &mut Frame,
    content: Rect,
    session: &Session,
    hits: &mut crate::pointer::Map,
) {
    if content.is_empty() {
        return;
    }
    let selected = match session.focus {
        Focus::List => 0,
        Focus::Actions(index) => index,
    };
    let widths = session
        .actions()
        .iter()
        .map(|action| action.label().width() + 4)
        .collect::<Vec<_>>();
    let start = (0..=selected)
        .find(|&start| {
            widths[start..=selected].iter().sum::<usize>() + 2 * (selected - start)
                <= usize::from(content.width)
        })
        .unwrap_or(selected);
    let mut x = content.x;
    for (index, action) in session.actions().iter().copied().enumerate().skip(start) {
        let available = content.right().saturating_sub(x);
        if widths[index] > usize::from(available) && index != selected {
            break;
        }
        let width = (widths[index] as u16).min(available);
        if width == 0 {
            break;
        }
        let rect = Rect::new(x, content.y, width, 1);
        let focused = session.focus == Focus::Actions(index);
        let style = if focused {
            selected_style().fg(color(action)).bold()
        } else {
            Style::default().fg(color(action))
        };
        let bracket = if focused {
            style
        } else {
            Style::default().fg(MUTED)
        };
        let line = Line::from(vec![
            Span::styled("[ ", bracket),
            Span::styled(action.label(), style),
            Span::styled(" ]", bracket),
        ]);
        frame.render_widget(Paragraph::new(line).style(style), rect);
        hits.button(rect, PointerAction::Management(action));
        x = x.saturating_add(width + 2).min(content.right());
    }
}
