//! Bordered input with explicit editable and locked states.
use super::*;

const INPUT_ACCENT: Color = Color::Rgb(180, 145, 220);

enum Content<'a> {
    Editable {
        editor: &'a Editor,
        placeholder: &'static str,
    },
    Locked(String),
}

pub(super) fn render(frame: &mut Frame, screen: &Screen, area: Rect) {
    let (title, content, hint) = match &screen.mode {
        Mode::Browsing | Mode::Files(_) | Mode::Diff { .. } => {
            let command = matches!(screen.bar, crate::bar::InputBar::Command { .. });
            let files = screen.mode.file_session().is_some();
            let hint = if command {
                "Tab 补全 · ↑↓候选 · Enter 执行完整命令 · Esc 清空"
            } else {
                match screen.mode {
                    Mode::Files(_) => "↑↓选择 · Space/Ins勾选 · Ctrl+A全选 · Tab切换区域",
                    Mode::Diff { .. } => "↑↓滚动 · Esc返回文件 · /命令",
                    _ => match screen.page {
                        Page::Repositories => "↑↓选择 · Enter详情 · Ctrl+F筛选 · Tab设置 · F1帮助",
                        Page::Settings => "↑↓选择 · Enter修改 · Tab仓库 · /命令",
                        _ => "↑↓/PgDn滚动 · Esc返回 · /命令",
                    },
                }
            };
            (
                if command {
                    " 命令 "
                } else if files {
                    " 搜索文件 "
                } else {
                    " 搜索仓库 "
                },
                Content::Editable {
                    editor: screen.bar.editor(),
                    placeholder: "输入关键词搜索，或输入 / 查看命令",
                },
                hint,
            )
        }
        Mode::Input { editor, .. } => (
            " 配置输入 ",
            Content::Editable {
                editor,
                placeholder: "请输入配置值",
            },
            "Enter 下一步 · Esc 取消",
        ),
        Mode::CommitMessage { editor, .. } => (
            " 提交说明 ",
            Content::Editable {
                editor,
                placeholder: "请输入本次提交的说明",
            },
            "Enter 确认 · Esc 取消 · PgUp/PgDn 查看",
        ),
        Mode::Confirm(_) => (
            " 确认操作 ",
            Content::Locked("等待确认".into()),
            "y 确认执行 · n/Esc 取消",
        ),
        Mode::CommitConfirm { .. } => (
            " 确认提交 ",
            Content::Locked("等待确认提交".into()),
            "y 确认提交 · n/Esc 取消 · PgUp/PgDn 查看",
        ),
        Mode::PushConfirm { .. } => (
            " 确认推送 ",
            Content::Locked("等待确认推送".into()),
            "y 确认推送 · n/Esc 取消 · PgUp/PgDn 查看",
        ),
        Mode::Error { .. } => (
            " 操作未完成 ",
            Content::Locked("请查看错误详情".into()),
            "↑↓/PgDn 查看完整错误 · Enter/Esc 返回",
        ),
        Mode::Busy(p) => (
            " 执行中 ",
            Content::Locked(if p.state == TaskState::Cancelling {
                "正在取消…".into()
            } else {
                safe(&p.title)
            }),
            "Esc/Ctrl+C 取消并等待子进程退出",
        ),
    };
    let parts = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Length(1),
    ])
    .split(area);
    frame.render_widget(
        Paragraph::new(truncate(&safe(&screen.message), area.width as usize))
            .style(Style::default().fg(WARN)),
        parts[0],
    );
    let editable = matches!(content, Content::Editable { .. });
    let input_style = Style::default().bg(Color::Reset).fg(Color::Reset);
    let block = Block::bordered()
        .border_type(ratatui::widgets::BorderType::Rounded)
        .title(title)
        .title_style(Style::default().fg(if editable { INPUT_ACCENT } else { MUTED }))
        .border_style(Style::default().fg(if editable { INPUT_ACCENT } else { MUTED }))
        .style(input_style);
    let inner = block.inner(parts[1]);
    frame.render_widget(block, parts[1]);
    match content {
        Content::Editable {
            editor,
            placeholder,
        } => {
            let width = inner.width.saturating_sub(2) as usize;
            let (text, cursor) = editor.viewport(width);
            let value = if editor.text.is_empty() {
                Span::styled(
                    truncate(placeholder, width),
                    Style::default().fg(MUTED).add_modifier(Modifier::ITALIC),
                )
            } else {
                Span::styled(text, input_style)
            };
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("› ", Style::default().fg(INPUT_ACCENT)),
                    value,
                ]))
                .style(input_style),
                inner,
            );
            frame.set_cursor_position((inner.x + 2 + cursor, inner.y));
        }
        Content::Locked(text) => frame.render_widget(
            Paragraph::new(truncate(&text, inner.width as usize)).style(Style::default().fg(MUTED)),
            inner,
        ),
    }
    frame.render_widget(
        Paragraph::new(truncate(hint, area.width as usize)).style(Style::default().fg(MUTED)),
        parts[2],
    );
}
