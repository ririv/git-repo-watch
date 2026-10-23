//! Styled quick reference with explicit headings, inline tokens and scroll position.
use super::*;

fn token_style(value: &str) -> Style {
    let color = match value {
        "✓" => GOOD,
        "!" => WARN,
        "×" => BAD,
        "·" | "?" => Color::Gray,
        value if value.starts_with('/') => Color::LightCyan,
        _ => Color::LightYellow,
    };
    Style::default().fg(color).add_modifier(Modifier::BOLD)
}

fn inline(value: &str, body: Style) -> Vec<Span<'static>> {
    value
        .split('`')
        .enumerate()
        .map(|(index, part)| {
            if index % 2 == 0 {
                Span::styled(part.to_owned(), body)
            } else {
                Span::styled(part.to_owned(), token_style(part))
            }
        })
        .collect()
}

fn content() -> Text<'static> {
    let lines = i18n::tr!(TuiHelp)
        .lines()
        .map(|line| {
            if let Some(title) = line.strip_prefix("## ") {
                Line::from(vec![
                    Span::styled("  ━━ ", Style::default().fg(ACCENT)),
                    Span::styled(
                        title.to_owned(),
                        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                    ),
                ])
            } else if let Some(item) = line.strip_prefix("- ") {
                let mut spans = vec![Span::styled("  • ", Style::default().fg(MUTED))];
                spans.extend(inline(item, Style::default()));
                Line::from(spans)
            } else if let Some(note) = line.strip_prefix("> ") {
                let mut spans = vec![Span::styled("  │ ", Style::default().fg(ACCENT))];
                spans.extend(inline(
                    note,
                    Style::default()
                        .fg(Color::Gray)
                        .add_modifier(Modifier::ITALIC),
                ));
                Line::from(spans)
            } else {
                Line::from(inline(line, Style::default()))
            }
        })
        .collect::<Vec<_>>();
    Text::from(lines)
}

pub(super) fn render(
    frame: &mut Frame,
    area: Rect,
    scroll: &mut usize,
    hits: &mut crate::pointer::Map,
) {
    let panel = block(i18n::tr!(Instructions)).padding(ratatui::widgets::Padding::horizontal(1));
    let inner = panel.inner(area);
    let paragraph = Paragraph::new(content()).wrap(Wrap { trim: false });
    let lines = paragraph.line_count(inner.width);
    let max_scroll = lines.saturating_sub(inner.height as usize);
    *scroll = (*scroll).min(max_scroll);
    frame.render_widget(
        paragraph
            .scroll(((*scroll).min(u16::MAX as usize) as u16, 0))
            .block(panel),
        area,
    );
    crate::scrollbar::vertical(frame, area, inner, lines, *scroll);
    hits.vertical(area, inner, max_scroll, crate::pointer::ScrollTarget::Text);
}
