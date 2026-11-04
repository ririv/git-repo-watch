//! Directory candidates share the full-width panel above the persistent input.
use super::*;

fn parameter_title(screen: &Screen) -> &'static str {
    let input = screen.bar.command_input().expect("parameter command input");
    let (argument, _) =
        command::argument_input(&input.query()).expect("parameter candidate command");
    argument.command().summary().text()
}

pub(super) fn render(
    frame: &mut Frame,
    screen: &Screen,
    directories: &command::path::Directories,
    body: Rect,
    viewport: &mut candidates::Viewport,
) -> Vec<(Rect, usize)> {
    let mut items = directories
        .names()
        .iter()
        .map(|name| {
            ListItem::new(Line::from(Span::styled(
                truncate(&format!("{name}/"), body.width.saturating_sub(2) as usize),
                Style::default().fg(ACCENT),
            )))
        })
        .collect::<Vec<_>>();
    if items.is_empty() {
        items.push(ListItem::new(i18n::tr!(NoMatchingDirectories)));
    }
    let areas = candidates::panel(
        frame,
        screen,
        body,
        directories.names().len(),
        Some(i18n::tr!(DirectoryCandidates)),
    );
    candidates::render_list(
        frame,
        areas,
        items,
        Some(directories.selected()),
        candidates::Source::Directories(directories.names().to_vec()),
        viewport,
    )
}

pub(super) fn render_values(
    frame: &mut Frame,
    screen: &Screen,
    values: &command::values::Candidates,
    body: Rect,
    viewport: &mut candidates::Viewport,
) -> Vec<(Rect, usize)> {
    use unicode_width::UnicodeWidthStr;
    let entries = values.entries().collect::<Vec<_>>();
    let areas = candidates::panel(
        frame,
        screen,
        body,
        entries.len(),
        Some(parameter_title(screen)),
    );
    let width = usize::from(areas.1.width);
    let label_width = entries
        .iter()
        .map(|entry| entry.label.width())
        .max()
        .unwrap_or(0)
        .min(width.saturating_sub(8));
    let mut items = entries
        .iter()
        .map(|entry| {
            let label = crate::horizontal::clip(&Line::from(safe(&entry.label)), 0, label_width);
            let gap = label_width.saturating_sub(label.width()) + 2;
            let description = crate::horizontal::clip(
                &Line::from(safe(&entry.description)),
                0,
                width.saturating_sub(label_width + 2),
            );
            let mut spans = label
                .spans
                .into_iter()
                .map(|span| span.style(Style::default().fg(ACCENT).bold()))
                .collect::<Vec<_>>();
            spans.push(Span::raw(
                " ".repeat(gap.min(width.saturating_sub(label_width))),
            ));
            spans.extend(
                description
                    .spans
                    .into_iter()
                    .map(|span| span.style(Style::default().fg(MUTED))),
            );
            ListItem::new(Line::from(spans))
        })
        .collect::<Vec<_>>();
    if items.is_empty() {
        items.push(ListItem::new(i18n::tr!(NoMatchingOptions)));
    }
    candidates::render_list(
        frame,
        areas,
        items,
        values.selected,
        candidates::Source::Values(entries.iter().map(|entry| entry.token.clone()).collect()),
        viewport,
    )
}

pub(super) fn render_options(
    frame: &mut Frame,
    screen: &Screen,
    values: &command::options::Candidates,
    body: Rect,
    viewport: &mut candidates::Viewport,
) -> Vec<(Rect, usize)> {
    use unicode_width::UnicodeWidthStr;
    let crate::bar::InputBar::Command {
        path, selection, ..
    } = &screen.bar
    else {
        unreachable!("option menu has a captured command draft")
    };
    let (area, inner) = candidates::panel(
        frame,
        screen,
        body,
        values.options.len(),
        Some(parameter_title(screen)),
    );
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
            let availability =
                screen.command_availability(option.value.action(), path.as_deref(), selection);
            let enabled = availability.reason().is_none();
            let mut spans = vec![Span::styled(
                truncate(option.token, name_width + 1),
                if enabled {
                    Style::default().fg(ACCENT).bold()
                } else {
                    Style::default().fg(MUTED)
                },
            )];
            let gap = name_width.saturating_sub(option.token.width()) + 2;
            let available = width.saturating_sub(name_width + 2 + 2);
            spans.push(Span::raw(
                " ".repeat(gap.min(width.saturating_sub(name_width))),
            ));
            let description = match availability.label() {
                Some(reason) => reason.to_owned(),
                None => {
                    match screen.command_target(option.value.action(), path.as_deref(), selection) {
                        Some(count) => format!("{} · {count}", option.description.text()),
                        None => option.description.text().into(),
                    }
                }
            };
            let description = crate::horizontal::clip(
                &Line::from(Span::styled(
                    safe(&description),
                    if enabled {
                        Style::default()
                    } else {
                        Style::default().fg(MUTED)
                    },
                )),
                0,
                available,
            );
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
                    Style::default().fg(if enabled { GOOD } else { MUTED }),
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
