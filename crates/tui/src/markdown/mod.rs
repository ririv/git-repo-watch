//! Markdown events rendered as owned Ratatui text, independent of pages and input state.
mod code;
mod table;
#[cfg(test)]
mod tests;

use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
};

const ACCENT: Color = Color::LightBlue;
const MUTED: Color = Color::DarkGray;

/// Render CommonMark with tables, strikethrough and task lists at the available width.
pub(super) fn render(source: &str, width: u16) -> Text<'static> {
    let options =
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
    let mut events = Parser::new_ext(source, options);
    let mut renderer = Renderer::default();
    while let Some(event) = events.next() {
        match event {
            Event::Start(Tag::CodeBlock(kind)) => {
                renderer.flush();
                let mut source = String::new();
                for event in events.by_ref() {
                    match event {
                        Event::End(TagEnd::CodeBlock) => break,
                        Event::Text(text) => source.push_str(&text),
                        _ => {}
                    }
                }
                let language = match &kind {
                    CodeBlockKind::Fenced(info) => info.split_whitespace().next().unwrap_or(""),
                    CodeBlockKind::Indented => "",
                };
                for line in code::render(language, &source) {
                    renderer.block_line(line);
                }
                renderer.blank();
            }
            Event::Start(Tag::Table(alignment)) => {
                renderer.flush();
                let available = width.saturating_sub(renderer.prefix_width());
                for line in table::render(&mut events, &alignment, available) {
                    renderer.block_line(line);
                }
                renderer.blank();
            }
            other => renderer.event(other),
        }
    }
    renderer.flush();
    while renderer
        .lines
        .last()
        .is_some_and(|line| line.spans.is_empty())
    {
        renderer.lines.pop();
    }
    Text::from(renderer.lines)
}

#[derive(Default)]
struct Renderer {
    lines: Vec<Line<'static>>,
    current: Vec<Span<'static>>,
    styles: Vec<Style>,
    containers: Vec<Container>,
    lists: Vec<List>,
    links: Vec<String>,
}

enum List {
    Unordered,
    Ordered(u64),
}
enum Container {
    Quote,
    Item { marker: String, first: bool },
}

impl Renderer {
    fn style(&self) -> Style {
        self.styles.last().copied().unwrap_or_default()
    }
    fn push_style(&mut self, style: Style) {
        self.styles.push(self.style().patch(style));
    }
    fn text(&mut self, text: impl Into<String>, style: Style) {
        // Markdown 文本不可把控制字符传给终端；换行由结构事件负责。
        let text = text
            .into()
            .chars()
            .map(|c| if c.is_control() { ' ' } else { c })
            .collect::<String>();
        self.current.push(Span::styled(text, style));
    }
    fn prefix(&mut self) -> Vec<Span<'static>> {
        self.containers
            .iter_mut()
            .map(|container| match container {
                Container::Quote => Span::styled("│ ", Style::default().fg(MUTED)),
                Container::Item { marker, first } => {
                    let text = if *first {
                        marker.clone()
                    } else {
                        " ".repeat(unicode_width::UnicodeWidthStr::width(marker.as_str()))
                    };
                    *first = false;
                    Span::styled(text, Style::default().fg(MUTED))
                }
            })
            .collect()
    }
    fn prefix_width(&self) -> u16 {
        self.containers
            .iter()
            .map(|c| match c {
                Container::Quote => 2,
                Container::Item { marker, .. } => {
                    unicode_width::UnicodeWidthStr::width(marker.as_str())
                }
            })
            .sum::<usize>()
            .min(u16::MAX as usize) as u16
    }
    fn flush(&mut self) {
        if !self.current.is_empty() {
            let mut spans = self.prefix();
            spans.append(&mut self.current);
            self.lines.push(Line::from(spans));
        }
    }
    fn block_line(&mut self, mut line: Line<'static>) {
        let mut spans = self.prefix();
        spans.append(&mut line.spans);
        self.lines.push(Line::from(spans));
    }
    fn blank(&mut self) {
        self.flush();
        if self.lines.last().is_some_and(|line| !line.spans.is_empty()) {
            self.lines.push(Line::default());
        }
    }
    fn event(&mut self, event: Event<'_>) {
        match event {
            Event::Start(tag) => match tag {
                Tag::Heading { .. } => {
                    self.blank();
                    self.push_style(Style::default().fg(ACCENT).add_modifier(Modifier::BOLD));
                }
                Tag::Paragraph => self.flush(),
                Tag::Strong => self.push_style(Style::default().add_modifier(Modifier::BOLD)),
                Tag::Emphasis => self.push_style(Style::default().add_modifier(Modifier::ITALIC)),
                Tag::Strikethrough => {
                    self.push_style(Style::default().add_modifier(Modifier::CROSSED_OUT))
                }
                Tag::BlockQuote(_) => {
                    self.flush();
                    self.containers.push(Container::Quote);
                }
                Tag::List(start) => {
                    self.flush();
                    self.lists
                        .push(start.map_or(List::Unordered, List::Ordered));
                }
                Tag::Item => {
                    self.flush();
                    let marker = match self.lists.last_mut() {
                        Some(List::Ordered(number)) => {
                            let marker = format!("{number}. ");
                            *number = number.saturating_add(1);
                            marker
                        }
                        _ => "• ".to_owned(),
                    };
                    self.containers.push(Container::Item {
                        marker,
                        first: true,
                    });
                }
                Tag::Link { dest_url, .. } | Tag::Image { dest_url, .. } => {
                    self.links.push(dest_url.into_string());
                    self.push_style(
                        Style::default()
                            .fg(ACCENT)
                            .add_modifier(Modifier::UNDERLINED),
                    );
                }
                _ => {}
            },
            Event::End(tag) => match tag {
                TagEnd::Heading(_) => {
                    self.flush();
                    self.styles.pop();
                }
                TagEnd::Strong | TagEnd::Emphasis | TagEnd::Strikethrough => {
                    self.styles.pop();
                }
                TagEnd::Paragraph => {
                    self.flush();
                    if self.lists.is_empty() {
                        self.blank();
                    }
                }
                TagEnd::Item | TagEnd::BlockQuote(_) => {
                    self.flush();
                    self.containers.pop();
                }
                TagEnd::List(_) => {
                    self.flush();
                    self.lists.pop();
                    if self.lists.is_empty() {
                        self.blank();
                    }
                }
                TagEnd::Link | TagEnd::Image => {
                    self.styles.pop();
                    if let Some(destination) = self.links.pop() {
                        self.text(format!(" ({destination})"), Style::default().fg(MUTED));
                    }
                }
                _ => {}
            },
            Event::Text(text) | Event::Html(text) | Event::InlineHtml(text) => {
                self.text(text.into_string(), self.style())
            }
            Event::Code(text) => {
                self.text(text.to_string(), self.style().patch(token_style(&text)))
            }
            Event::SoftBreak => self.text(" ", self.style()),
            Event::HardBreak => self.flush(),
            Event::Rule => {
                self.blank();
                self.text("────────", Style::default().fg(MUTED));
                self.blank();
            }
            Event::TaskListMarker(checked) => self.text(
                if checked { "☑ " } else { "☐ " },
                Style::default().fg(ACCENT),
            ),
            _ => {}
        }
    }
}

fn token_style(value: &str) -> Style {
    let color = match value {
        "✓" => Color::LightGreen,
        "!" => Color::Yellow,
        "×" => Color::LightRed,
        "·" | "?" => Color::Gray,
        value if value.starts_with('/') => Color::LightCyan,
        _ => Color::LightYellow,
    };
    Style::default().fg(color).add_modifier(Modifier::BOLD)
}
