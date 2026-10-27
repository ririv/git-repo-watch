//! Directory candidates share the full-width panel above the persistent input.
use super::*;

pub(super) fn render(
    frame: &mut Frame,
    screen: &Screen,
    directories: &command::path::Directories,
    body: Rect,
    viewport: &mut candidates::Viewport,
) {
    let items = directories
        .names()
        .iter()
        .map(|name| {
            ListItem::new(Line::from(Span::styled(
                truncate(&format!("{name}/"), body.width.saturating_sub(2) as usize),
                Style::default().fg(ACCENT),
            )))
        })
        .collect::<Vec<_>>();
    let areas = candidates::panel(frame, screen, body, directories.names().len());
    candidates::render_list(
        frame,
        areas,
        items,
        Some(directories.selected()),
        candidates::Source::Directories(directories.names().to_vec()),
        viewport,
    );
}

pub(super) fn render_options(
    frame: &mut Frame,
    screen: &Screen,
    values: &command::options::Candidates,
    body: Rect,
    viewport: &mut candidates::Viewport,
) -> Vec<(Rect, usize)> {
    use unicode_width::UnicodeWidthStr;
    let (area, inner) = candidates::panel(frame, screen, body, values.options.len());
    let width = usize::from(inner.width);
    let name_width = values
        .options
        .iter()
        .map(|option| option.token.width())
        .max()
        .unwrap_or(0)
        .min(width);
    let items = values
        .options
        .iter()
        .map(|option| {
            let mut spans = vec![Span::styled(
                truncate(option.token, name_width + 1),
                Style::default().fg(ACCENT).bold(),
            )];
            let gap = name_width.saturating_sub(option.token.width()) + 2;
            let available = width.saturating_sub(name_width + 2 + 2);
            spans.push(Span::raw(
                " ".repeat(gap.min(width.saturating_sub(name_width))),
            ));
            let description =
                crate::horizontal::clip(&Line::from(safe(option.description.text())), 0, available);
            let used = option.token.width() + gap + description.width();
            spans.extend(description.spans);
            if width >= 2 {
                spans.push(Span::raw(" ".repeat(width.saturating_sub(used + 1))));
                spans.push(Span::styled(
                    if Some(option.value) == values.current {
                        "✓"
                    } else {
                        " "
                    },
                    Style::default().fg(GOOD),
                ));
            }
            ListItem::new(Line::from(spans))
        })
        .collect::<Vec<_>>();
    let selected = (!items.is_empty()).then_some(values.selected);
    candidates::render_list(
        frame,
        (area, inner),
        if items.is_empty() {
            vec![ListItem::new(i18n::tr!(NoMatchingOptions))]
        } else {
            items
        },
        selected,
        candidates::Source::Options(values.options.iter().map(|option| option.value).collect()),
        viewport,
    )
}
