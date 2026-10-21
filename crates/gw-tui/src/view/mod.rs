//! Terminal layout and shared text-safe widgets.
mod body;
mod help;
mod input;
mod repository;
mod review;
mod settings;
mod shell;
use super::*;
use body::render_body;
use gw_core::{
    domain::{IssueKind, Stats},
    scheduler::{Registration, Schedule},
    service::{Activity, CachedReport, Scope},
};
use help::HELP;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Cell, Clear, List, ListItem, ListState, Paragraph, Row as TableRow, Table,
        TableState, Wrap,
    },
};
use repository::{detail_text, repositories};
use review::{commit_review, text_view};
use settings::settings;
use shell::header;
const ACCENT: Color = Color::LightBlue;
const MUTED: Color = Color::DarkGray;
const GOOD: Color = Color::LightGreen;
const BAD: Color = Color::LightRed;
const WARN: Color = Color::Yellow;
fn safe(text: &str) -> String {
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
fn name(path: &std::path::Path) -> String {
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
    let area = frame.area();
    if area.width < 60 || area.height < 18 {
        frame.render_widget(
            Paragraph::new(
                "gw：请将终端调整到至少 60 列 × 18 行。\nCtrl+C 退出；进行中的操作可按 Esc 取消。",
            ),
            area,
        );
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
    input::render(frame, screen, layout[2]);
    render_body(frame, screen, layout[1]);
    render_commands(frame, screen, layout[1]);
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
        Err(_) => format!("报告时间格式无效：{value}"),
    }
}

fn render_commands(frame: &mut Frame, screen: &Screen, body: Rect) {
    let crate::bar::InputBar::Command { selected, path, .. } = &screen.bar else {
        return;
    };
    if !matches!(
        screen.mode,
        Mode::Browsing | Mode::Files(_) | Mode::Diff { .. }
    ) {
        return;
    }
    let matches = screen
        .bar
        .commands()
        .expect("command bar provides candidates");
    let items = matches
        .iter()
        .map(|c| {
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("/{:<13} ", c.name),
                    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
                ),
                Span::raw(c.description),
                Span::styled(
                    format!(
                        "  · {}",
                        match c.scope {
                            command::Scope::Repository => "当前仓库",
                            command::Scope::Global => "全局",
                            command::Scope::View => "当前视图",
                        }
                    ),
                    Style::default().fg(MUTED),
                ),
            ]))
        })
        .collect::<Vec<_>>();
    let height = (items.len().clamp(1, 5) + 2).min(body.height as usize) as u16;
    let area = Rect::new(
        body.x,
        body.bottom().saturating_sub(height),
        body.width,
        height,
    );
    let title = format!(
        "命令 · {}",
        path.as_deref()
            .map(name)
            .unwrap_or_else(|| "未选择仓库".into())
    );
    let mut state = ListState::default().with_selected(if items.is_empty() {
        None
    } else {
        Some(*selected)
    });
    frame.render_widget(Clear, area);
    frame.render_stateful_widget(
        List::new(if items.is_empty() {
            vec![ListItem::new("没有匹配命令")]
        } else {
            items
        })
        .block(block(title))
        .highlight_style(selected_style())
        .highlight_symbol("› "),
        area,
        &mut state,
    );
}

#[cfg(test)]
mod tests {
    use super::local_time;
    #[test]
    fn report_time_uses_system_local_time_without_zone_suffix() {
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
