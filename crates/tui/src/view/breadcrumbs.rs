//! Width-aware breadcrumb rendering keeps the active view visible and ancestors clickable.
use super::*;
use crate::features::Maturity;
use crate::navigation::breadcrumbs::Crumb;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

pub(super) fn shorten(text: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    if text.width() <= width {
        return text.into();
    }
    let head_width = width.saturating_sub(1) / 2;
    let tail_width = width.saturating_sub(1) - head_width;
    let take = |chars: &mut dyn Iterator<Item = char>, budget: usize| {
        let mut used = 0;
        chars
            .take_while(|c| {
                used += c.width().unwrap_or(0);
                used <= budget
            })
            .collect::<String>()
    };
    let head = take(&mut text.chars(), head_width);
    let tail = take(&mut text.chars().rev(), tail_width)
        .chars()
        .rev()
        .collect::<String>();
    format!("{head}…{tail}")
}

fn width(crumbs: &[Crumb]) -> usize {
    crumbs
        .iter()
        .map(|crumb| crumb.label.width())
        .sum::<usize>()
        + 3 * crumbs.len().saturating_sub(1)
}
pub(super) fn render(screen: &mut Screen, area: Rect) -> Line<'static> {
    let mut crumbs = screen.breadcrumbs();
    for crumb in &mut crumbs {
        crumb.label = safe(&crumb.label);
    }
    let available = usize::from(area.width.saturating_sub(2));
    if width(&crumbs) > available {
        for crumb in &mut crumbs {
            if crumb.label.width() > available / 2 {
                crumb.label = shorten(&crumb.label, available / 2);
            }
        }
    }
    let mut omitted = false;
    while width(&crumbs) + if omitted { 4 } else { 0 } > available && crumbs.len() > 2 {
        crumbs.remove(1);
        omitted = true;
    }
    if omitted {
        crumbs.insert(
            1,
            Crumb {
                label: "…".into(),
                destination: None,
                maturity: Maturity::Stable,
            },
        );
    }
    if width(&crumbs) > available {
        let excess = width(&crumbs) - available;
        if let Some(last) = crumbs.last_mut() {
            last.label = shorten(&last.label, last.label.width().saturating_sub(excess));
        }
    }
    let count = crumbs.len();
    let mut spans = vec![Span::raw(" ")];
    let mut x = area.x + 1;
    for (index, crumb) in crumbs.into_iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled(" › ", Style::default().fg(MUTED)));
            x += 3;
        }
        let width = crumb.label.width() as u16;
        if let Some(destination) = crumb.destination {
            let hit = Rect::new(x, area.y, width, 1).intersection(area);
            if !hit.is_empty() {
                screen.breadcrumb_items.push((hit, destination));
            }
        }
        spans.push(Span::styled(
            crumb.label,
            if crumb.maturity == Maturity::Experimental {
                let style = Style::default().fg(EXPERIMENTAL);
                if index + 1 == count {
                    style.bold()
                } else {
                    style
                }
            } else if index + 1 == count {
                Style::default().fg(ACCENT).bold()
            } else {
                Style::default().fg(MUTED)
            },
        ));
        x += width;
    }
    Line::from(spans)
}
