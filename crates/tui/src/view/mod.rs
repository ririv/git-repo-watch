//! Terminal layout and shared text-safe widgets.
mod body;
mod branches;
pub(super) mod changes;
mod completion;
mod diff;
mod document;
mod files;
mod help;
mod history;
mod input;
pub(super) mod layout_picker;
mod overview;
mod repository;
mod repository_list;
mod review;
mod schedule;
mod settings;
mod shell;
mod workspace;
use super::*;
use body::render_body;
use engine::{
    domain::{IssueKind, Stats},
    scheduler::{Registration, Schedule},
    service::{Activity, CachedReport, Scope},
};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Cell, Clear, List, ListItem, ListState, Paragraph, Row as TableRow, Table,
        Wrap,
    },
};
use repository::repositories;
use review::{commit_review, text_view};
use settings::settings;
use shell::header;
const ACCENT: Color = Color::LightBlue;
const MUTED: Color = Color::DarkGray;
const GOOD: Color = Color::LightGreen;
const BAD: Color = Color::LightRed;
const WARN: Color = Color::Yellow;
pub(crate) fn safe(text: &str) -> String {
    String::from_utf8_lossy(&strip_ansi_escapes::strip(text))
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}
fn multiline(text: &str) -> String {
    String::from_utf8_lossy(&strip_ansi_escapes::strip(text))
        .chars()
        .map(|c| {
            if c.is_control() && c != '\n' && c != '\t' {
                ' '
            } else {
                c
            }
        })
        .collect()
}
pub(super) fn name(path: &std::path::Path) -> String {
    safe(
        &path
            .file_name()
            .unwrap_or(path.as_os_str())
            .to_string_lossy(),
    )
}
fn selected_style() -> Style {
    Style::default().bg(Color::Rgb(25, 65, 95)).fg(Color::White)
}
fn block(title: impl Into<String>) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(MUTED))
        .title(title.into())
        .title_style(Style::default().fg(ACCENT))
}
pub(super) fn render(frame: &mut Frame, screen: &mut Screen) {
    screen.graphics.begin_frame();
    screen.body_area = None;
    screen.repository_area = None;
    screen.workspace_areas.clear();
    screen.command_items.clear();
    let area = frame.area();
    if area.width < 60 || area.height < 18 {
        frame.render_widget(Paragraph::new(i18n::tr!(TerminalSizeTooSmall)), area);
        return;
    }
    let header = header(screen);
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(header.lines.len() as u16),
            Constraint::Min(3),
            Constraint::Length(5),
        ])
        .split(area);
    frame.render_widget(Paragraph::new(header), layout[0]);
    let body = layout[1];
    screen.body_area = Some(body);
    input::render(frame, screen, layout[2]);
    render_body(frame, screen, body);
    render_commands(frame, screen, body);
}
fn truncate(text: &str, width: usize) -> String {
    let mut out = String::new();
    let mut count = 0;
    for c in text.chars() {
        let w = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
        if count + w > width.saturating_sub(1) {
            out.push('…');
            break;
        }
        count += w;
        out.push(c)
    }
    out
}

fn local_time(value: &str) -> String {
    match chrono::DateTime::parse_from_rfc3339(value) {
        Ok(time) => time
            .with_timezone(&chrono::Local)
            .format("%Y-%m-%d %H:%M:%S")
            .to_string(),
        Err(_) => i18n::tf!(InvalidReportTimestampValue, value = value),
    }
}

fn render_commands(frame: &mut Frame, screen: &mut Screen, body: Rect) {
    let crate::bar::InputBar::Command {
        input,
        selected,
        path,
        ..
    } = &screen.bar
    else {
        return;
    };
    if !matches!(
        screen.mode,
        Mode::Browsing
            | Mode::Panel(_)
            | Mode::Branches(_)
            | Mode::Files(_)
            | Mode::Changes(_)
            | Mode::History(_)
    ) {
        return;
    }
    if let crate::bar::InputBar::Command {
        menu: command::path::Menu::Directories(directories),
        ..
    } = &screen.bar
    {
        completion::render(frame, directories, body);
        return;
    }
    let matches = screen
        .bar
        .commands()
        .expect("command bar provides candidates");
    let name_width = matches
        .iter()
        .map(|command| input.usage(command).len())
        .max()
        .unwrap_or(0)
        .max(13);
    let items = matches
        .iter()
        .map(|c| {
            let scope = match c.scope {
                command::Scope::Repository => i18n::tr!(CurrentRepo),
                command::Scope::Global => i18n::tr!(Global),
                command::Scope::View => i18n::tr!(CurrentView),
            };
            let annotation = repository_list::annotation(c.action, screen)
                .or_else(|| layout_picker::annotation(c.action, screen, body));
            let annotation_width = annotation
                .as_deref()
                .map_or(0, |text| unicode_width::UnicodeWidthStr::width(text) + 3);
            let description_width = (body.width as usize).saturating_sub(
                4 + name_width
                    + 2
                    + 4
                    + unicode_width::UnicodeWidthStr::width(scope)
                    + annotation_width,
            );
            let description = truncate(c.description.text(), description_width);
            let padding = description_width
                .saturating_sub(unicode_width::UnicodeWidthStr::width(description.as_str()));
            let mut spans = vec![Span::styled(
                format!(
                    "{}{}",
                    if input.context().is_some() { "" } else { "/" },
                    input.name(c)
                ),
                Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
            )];
            if let Some(argument) = c.action.argument() {
                spans.push(Span::styled(
                    format!(" {}", argument.placeholder()),
                    Style::default()
                        .fg(Color::LightCyan)
                        .add_modifier(Modifier::ITALIC),
                ));
            }
            let usage_width = input.usage(c).len();
            spans.push(Span::raw(" ".repeat(name_width - usage_width + 1)));
            if let Some(annotation) = annotation {
                spans.push(Span::styled(
                    format!("[{annotation}] "),
                    Style::default().fg(GOOD),
                ));
            }
            spans.push(Span::raw(format!("{description}{}", " ".repeat(padding))));
            spans.push(Span::styled(
                format!("  · {scope}"),
                Style::default().fg(MUTED),
            ));
            ListItem::new(Line::from(spans))
        })
        .collect::<Vec<_>>();
    // 留出筛选栏和列表标题，避免矮窗口中的命令面板遮住列表标题。
    let available_height = body.height.saturating_sub(3) as usize;
    let height = (items.len().clamp(1, 8) + 2).min(available_height) as u16;
    let area = Rect::new(
        body.x,
        body.bottom().saturating_sub(height),
        body.width,
        height,
    );
    let title = input
        .context()
        .map(|context| format!("/{} · {}", context.name(), i18n::tr!(ContextCommands)))
        .or_else(|| repository_list::menu_title(screen).map(str::to_owned))
        .unwrap_or_else(|| {
            i18n::tf!(
                Commands,
                path.as_deref()
                    .map(name)
                    .unwrap_or_else(|| i18n::tr!(NoRepositorySelected).into())
            )
        });
    let mut state = ListState::default().with_selected(if items.is_empty() {
        None
    } else {
        Some(*selected)
    });
    let count = items.len();
    frame.render_widget(Clear, area);
    frame.render_stateful_widget(
        List::new(if items.is_empty() {
            vec![ListItem::new(i18n::tr!(NoMatchingCommands))]
        } else {
            items
        })
        .block(block(title))
        .highlight_style(selected_style())
        .highlight_symbol("› "),
        area,
        &mut state,
    );
    screen.command_items = (state.offset()..count)
        .take(area.height.saturating_sub(2) as usize)
        .enumerate()
        .map(|(row, index)| {
            (
                Rect::new(
                    area.x + 1,
                    area.y + 1 + row as u16,
                    area.width.saturating_sub(2),
                    1,
                ),
                index,
            )
        })
        .collect();
}

#[cfg(test)]
mod tests {
    use super::local_time;
    #[test]
    fn report_time_uses_system_local_time_without_zone_suffix() {
        let _language = i18n::using(i18n::Language::Chinese);
        let utc = "2026-10-21T04:31:16Z";
        let expected = chrono::DateTime::parse_from_rfc3339(utc)
            .unwrap()
            .with_timezone(&chrono::Local)
            .format("%Y-%m-%d %H:%M:%S")
            .to_string();
        assert_eq!(local_time(utc), expected);
        assert_eq!(local_time("2026-10-21T12:31:16+08:00"), expected);
        assert_eq!(expected.len(), 19);
    }
}
