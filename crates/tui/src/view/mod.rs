//! Terminal layout and shared text-safe widgets.
mod body;
mod branches;
mod breadcrumbs;
pub(super) mod browser;
mod candidates;
mod checkbox;
mod management;
mod metadata;
pub(crate) use candidates::Viewport as CandidateViewport;
pub(super) mod changes;
mod commands;
pub(crate) mod commit;
mod completion;
pub(crate) mod composition;
pub(crate) mod conflict;
mod diff;
mod document;
mod feedback;
mod files;
mod help;
pub(crate) mod history;
pub(super) mod input;
pub(crate) mod inspection;
mod layout_summary;
mod overview;
mod prompt;
mod push;
mod replay;
mod repository;
mod repository_list;
mod review;
mod schedule;
mod settings;
mod shell;
pub(crate) mod stash_save;
mod stashes;
mod tasks;
mod text_editor;
mod workflow;
mod workspace;
use super::*;
use body::render_body;
use engine::{
    domain::IssueKind,
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
pub(crate) const ACCENT: Color = Color::LightBlue;
pub(crate) const EXPERIMENTAL: Color = Color::Rgb(95, 201, 181);
pub(crate) const MUTED: Color = Color::DarkGray;
const BORDER: Color = Color::DarkGray;
const GOOD: Color = Color::LightGreen;
const BAD: Color = Color::LightRed;
const WARN: Color = Color::Yellow;
pub(crate) use crate::appearance::SELECTION;
const FOOTER_INSET: u16 = 2;
fn footer_text_area(area: Rect) -> Rect {
    Rect::new(
        area.x + FOOTER_INSET.min(area.width),
        area.y,
        area.width.saturating_sub(FOOTER_INSET * 2),
        area.height,
    )
}
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
// 列表控件会在绘制时收紧越界选择，片段着色必须使用同一个有效位置。
fn row_selection(selected: Option<usize>, count: usize) -> Option<usize> {
    selected
        .zip(count.checked_sub(1))
        .map(|(index, last)| index.min(last))
}
// 行背景先绘制，匹配片段随后叠加，避免整行选中样式覆盖匹配底色。
fn row_style(selected: bool) -> Style {
    if selected {
        selected_style()
    } else {
        Style::default()
    }
}
fn selected_style() -> Style {
    Style::default().bg(SELECTION)
}
fn block(title: impl Into<String>) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER))
        .title(title.into())
        .title_style(Style::default().fg(ACCENT))
}
/// Highlight the base border while retaining embedded indicators' own styles.
fn highlight_border(frame: &mut Frame, area: Rect) {
    highlight_border_with(frame, area, ACCENT);
}
fn highlight_border_with(frame: &mut Frame, area: Rect, accent: Color) {
    for y in area.y..area.bottom() {
        for x in area.x..area.right() {
            if x == area.x || x + 1 == area.right() || y == area.y || y + 1 == area.bottom() {
                let cell = &mut frame.buffer_mut()[(x, y)];
                // 滚动条已覆盖边框并使用独立配色；焦点只改变原始边框色。
                if cell.fg == BORDER {
                    cell.set_fg(accent);
                }
            }
        }
    }
}
/// Keep inactive cursor positions visible without competing with the keyboard target.
fn inactive_selection(frame: &mut Frame, area: Rect) {
    let area = area.intersection(frame.area());
    for y in area.y..area.bottom() {
        for x in area.x..area.right() {
            let cell = &mut frame.buffer_mut()[(x, y)];
            if cell.bg == SELECTION {
                cell.set_bg(crate::appearance::INACTIVE_SELECTION);
            }
            // 子视图可能已绘制焦点；外层失焦时撤销亮边框，保留标题和滚动条。
            if cell.fg == ACCENT
                && matches!(
                    cell.symbol(),
                    "─" | "│"
                        | "┌"
                        | "┐"
                        | "└"
                        | "┘"
                        | "├"
                        | "┤"
                        | "┬"
                        | "┴"
                        | "┼"
                        | "╭"
                        | "╮"
                        | "╰"
                        | "╯"
                )
            {
                cell.set_fg(BORDER);
            }
        }
    }
}
pub(super) fn render(frame: &mut Frame, screen: &mut Screen) {
    render_content(frame, screen);
    if screen.history_preview.area.is_none() {
        screen.history_preview.hide();
    }
    screen.background.apply(frame.buffer_mut());
}
fn render_content(frame: &mut Frame, screen: &mut Screen) {
    if screen.focus_mode == crate::focus::State::Selecting
        && (!screen.pane_navigation_available() || screen.tasks.session.open())
    {
        screen.focus_mode = crate::focus::State::Inactive;
    }
    screen.graphics.begin_frame();
    screen.history_preview.area = None;
    screen.body_area = None;
    screen.input_area = None;
    screen.repository_area = None;
    screen.workspace_areas.clear();
    screen.command_items.clear();
    if !matches!(screen.bar, crate::bar::InputBar::Command { .. }) {
        screen.command_viewport = Default::default();
    }
    screen.pointer.clear();
    screen.pointer.links.reset(
        screen.bar.is_plain()
            && !matches!(
                screen.mode,
                Mode::Confirm { .. } | Mode::PushConfirm { .. } | Mode::Busy(_)
            ),
    );
    screen.pointer.links.paths = screen.config.repositories.clone();
    screen.pointer.links.paths.extend([
        screen.service.store.config_dir(),
        screen.service.store.state_dir(),
        screen.service.store.log_dir(),
    ]);
    screen
        .pointer
        .links
        .paths
        .push(screen.service.store.config_path());
    screen
        .pointer
        .links
        .paths
        .push(screen.service.store.database_path());
    if let Some(path) = screen.scope_path() {
        screen.pointer.links.paths.push(path);
    }
    screen.breadcrumb_items.clear();
    let area = frame.area();
    if area.is_empty() {
        return;
    }
    if screen.tasks.session.open() {
        tasks::render(frame, screen, area);
        return;
    }
    screen.normalize_input_page();
    let mut header = if area.height >= 6 {
        header(screen, area)
    } else {
        Text::default()
    };
    let footer = input::Footer::for_screen(screen, area);
    let input_height = footer.height();
    if area.height < 14 {
        header.lines.truncate(1);
    }
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(header.lines.len() as u16),
            Constraint::Min(0),
            Constraint::Length(input_height),
        ])
        .split(area);
    frame.render_widget(Paragraph::new(header), layout[0]);
    let body = layout[1];
    screen.body_area = Some(body);
    footer.render(frame, screen, layout[2]);
    if !body.is_empty() {
        render_body(frame, screen, body);
    }
    commands::render(frame, screen, body);
    history::menu::render(frame, screen, body);
}
fn truncate(text: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    #[test]
    fn pane_focus_preserves_both_scrollbar_tracks_and_thumbs() {
        let area = Rect::new(0, 0, 20, 10);
        let mut terminal = Terminal::new(TestBackend::new(area.width, area.height)).unwrap();
        for focused in [false, true] {
            terminal
                .draw(|frame| {
                    frame.render_widget(block(""), area);
                    crate::scrollbar::vertical(frame, area, Rect::new(1, 2, 18, 7), 40, 0);
                    crate::scrollbar::render(
                        frame,
                        Rect::new(4, 9, 15, 1),
                        crate::scrollbar::Axis::Horizontal,
                        0,
                        50,
                    );
                    let indicators: Vec<_> = [(19, 2), (19, 8), (4, 9), (18, 9)]
                        .into_iter()
                        .map(|pos| (pos, frame.buffer_mut()[pos].clone()))
                        .collect();
                    highlight_border(frame, area);
                    if !focused {
                        inactive_selection(frame, area);
                    }
                    for (pos, original) in indicators {
                        assert_eq!(frame.buffer_mut()[pos], original, "indicator at {pos:?}");
                    }
                })
                .unwrap();
            let buffer = terminal.backend().buffer();
            assert_eq!(buffer[(0, 0)].symbol(), "┌");
            assert_eq!(buffer[(0, 0)].fg, if focused { ACCENT } else { BORDER });
            assert_eq!(buffer[(19, 2)].fg, crate::scrollbar::THUMB_COLOR);
            assert_eq!(buffer[(4, 9)].fg, crate::scrollbar::THUMB_COLOR);
            assert_ne!(buffer[(19, 2)].fg, ACCENT);
        }
    }

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
