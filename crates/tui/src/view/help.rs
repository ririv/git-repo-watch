//! Styled quick reference with explicit headings, inline tokens and scroll position.
use super::*;
use ratatui::widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState};

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

pub(super) fn render(frame: &mut Frame, area: Rect, scroll: &mut usize) {
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
    if lines > inner.height as usize {
        // 滚动条的位置对应视口起点，最后一屏应让滑块贴底。
        let mut state = ScrollbarState::new(max_scroll + 1)
            .position(*scroll)
            .viewport_content_length(inner.height as usize);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None)
                .thumb_style(Style::default().fg(ACCENT))
                .track_style(Style::default().fg(MUTED)),
            Rect::new(area.x, inner.y, area.width, inner.height),
            &mut state,
        );
    }
}
