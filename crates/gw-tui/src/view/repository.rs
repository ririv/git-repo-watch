use super::*;
pub(super) fn repositories(frame: &mut Frame, screen: &mut Screen, area: Rect) {
    let parts = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(3),
    ])
    .split(area);
    let mut filters = Vec::new();
    for filter in [
        Filter::All,
        Filter::Problems,
        Filter::WithoutUpstream,
        Filter::Pushes,
    ]
    .iter()
    {
        let count = if *filter == Filter::All {
            format!(" {}", screen.config.repositories.len())
        } else if *filter == screen.filter {
            format!(" {}库", screen.rows().len())
        } else {
            String::new()
        };
        filters.push(Span::styled(
            format!(" {}{} ", filter.name(), count),
            if *filter == screen.filter {
                selected_style()
            } else {
                Style::default().fg(MUTED)
            },
        ));
    }
    if !screen.query.is_empty() {
        filters.push(Span::raw(format!(" 搜索：{}", safe(&screen.query))))
    }
    frame.render_widget(Paragraph::new(Line::from(filters)), parts[0]);
    frame.render_widget(
        Paragraph::new("* 工作区改动 · ? 未知 · ↑/! 推送成功/失败分支数")
            .style(Style::default().fg(MUTED)),
        parts[1],
    );
    let body = parts[2];
    if area.width >= 120 && area.height >= 19 {
        let columns = Layout::horizontal([Constraint::Percentage(58), Constraint::Percentage(42)])
            .split(body);
        repository_table(frame, screen, columns[0]);
        let rows = screen.rows();
        let text = rows
            .get(screen.selected)
            .map(detail_text)
            .unwrap_or_else(|| "当前没有仓库".into());
        frame.render_widget(
            Paragraph::new(text)
                .wrap(Wrap { trim: false })
                .block(block("选中仓库")),
            columns[1],
        );
    } else {
        repository_table(frame, screen, body)
    }
}
struct Badges {
    overall: &'static str,
    workspace: &'static str,
    remote: &'static str,
    experimental: String,
    push: String,
    color: Color,
}
fn badges(result: Option<&RepositoryResult>) -> Badges {
    let mut b = Badges {
        overall: "·",
        workspace: "?",
        remote: "?",
        experimental: "?".into(),
        push: "—".into(),
        color: MUTED,
    };
    let Some(result) = result else { return b };
    match result {
        RepositoryResult::Clean {
            branches_without_upstream,
            ..
        } => {
            b.overall = "✓";
            b.workspace = "干净";
            b.remote = "通过";
            b.experimental = branches_without_upstream.len().to_string();
            b.color = GOOD
        }
        RepositoryResult::Attention {
            issues,
            branches_without_upstream,
            ..
        } => {
            b.overall = "!";
            b.workspace = "干净";
            b.remote = "通过";
            b.experimental = branches_without_upstream.len().to_string();
            b.color = WARN;
            for issue in issues {
                match issue.kind {
                    IssueKind::Dirty => b.workspace = "*改动",
                    IssueKind::OutOfSync => b.remote = "待同步",
                    IssueKind::MissingUpstream => b.remote = "上游缺失",
                    IssueKind::PushFailed => b.remote = "推送失败",
                    IssueKind::NoCommits => b.remote = "无提交",
                    _ => {}
                }
            }
        }
        RepositoryResult::Failed { error, .. } => {
            b.overall = "×";
            b.remote = failure_label(error);
            b.color = BAD
        }
    }
    let good = result
        .pushes()
        .iter()
        .filter(|p| matches!(p, Publication::Pushed { .. }))
        .count();
    let bad = result.pushes().len() - good;
    if good + bad > 0 {
        b.push = format!("↑{good} !{bad}")
    }
    b
}
fn failure_label(error: &str) -> &'static str {
    if error.contains("SSL_ERROR_SYSCALL") {
        "TLS失败"
    } else if error.contains("远程在 fetch 后发生变化或 fetch refspec 未更新上游") {
        "引用不符"
    } else {
        "检查失败"
    }
}
fn repository_table(frame: &mut Frame, screen: &Screen, area: Rect) {
    let rows = screen.rows();
    let data = rows
        .iter()
        .map(|row| {
            let b = badges(row.result.as_ref());
            TableRow::new(vec![
                Cell::from(format!("{} {}", b.overall, name(&row.path))),
                Cell::from(b.workspace),
                Cell::from(b.remote),
                Cell::from(b.experimental),
                Cell::from(b.push),
            ])
            .style(Style::default().fg(b.color))
        })
        .collect::<Vec<_>>();
    let table = Table::new(
        data,
        [
            Constraint::Min(12),
            Constraint::Length(7),
            Constraint::Length(9),
            Constraint::Length(6),
            Constraint::Length(8),
        ],
    )
    .header(
        TableRow::new(["仓库", "工作区", "远程", "无上游", "推送"])
            .style(Style::default().fg(MUTED)),
    )
    .block(block(if screen.order == Order::Priority {
        "列表 · 问题优先"
    } else {
        "列表 · 路径顺序"
    }))
    .row_highlight_style(selected_style())
    .highlight_symbol("› ")
    .column_spacing(1);
    let mut state = TableState::default().with_selected(if rows.is_empty() {
        None
    } else {
        Some(screen.selected)
    });
    frame.render_stateful_widget(table, area, &mut state);
    if rows.is_empty() {
        let inner = Rect {
            y: area.y + 2,
            height: area.height.saturating_sub(3),
            x: area.x + 1,
            width: area.width.saturating_sub(2),
        };
        frame.render_widget(Paragraph::new("当前筛选没有仓库；: add 添加仓库"), inner);
    }
}
pub(super) fn detail_text(row: &Row) -> String {
    let mut text = format!("{}\n", safe(&row.path.to_string_lossy()));
    match &row.result {
        None => text.push_str("上次报告未包含此仓库，等待定时或显式检查。"),
        Some(result) => {
            if let RepositoryResult::Failed { error, .. } = result {
                let summary = match failure_label(error) {
                    "TLS失败" => "远程 TLS 连接失败（SSL_ERROR_SYSCALL）",
                    "引用不符" => "上游引用不一致：远程更新或 fetch 配置未更新该上游",
                    _ => "检查失败，详情见原始错误",
                };
                text.push_str(&format!(
                    "失败原因：{summary}\n本轮未完成检查，工作区与分支统计未知。\n\n原始错误：{}\n",
                    multiline(error)
                ));
            } else {
                if let RepositoryResult::Attention { issues, .. } = result {
                    for issue in issues {
                        text.push_str(&format!("• {}\n", safe(&issue.detail)));
                    }
                }
                if let Some(branches) = result.branches() {
                    if !branches.is_empty() {
                        text.push_str("\n未配置远程上游\n");
                    }
                    for branch in branches {
                        text.push_str(&format!(
                            "• {}{}\n",
                            safe(&branch.branch),
                            if branch.reason == gw_core::domain::UpstreamReason::LocalUpstream {
                                "（跟踪本地分支）"
                            } else {
                                ""
                            }
                        ));
                    }
                }
                if !result.pushes().is_empty() {
                    text.push_str("\n推送记录\n");
                }
                for push in result.pushes() {
                    let target = push.target();
                    match push {
                        Publication::Pushed { .. } => text.push_str(&format!(
                            "• {} → {}/{}：已推送（{} 个提交）\n",
                            safe(&target.branch),
                            safe(&target.remote),
                            safe(&target.remote_ref),
                            target.ahead_commits
                        )),
                        Publication::Failed { phase, error, .. } => text.push_str(&format!(
                            "• {}：{phase:?} 阶段失败\n{}\n",
                            safe(&target.branch),
                            multiline(error)
                        )),
                    }
                }
            }
            if matches!(result, RepositoryResult::Failed { .. }) {
                for p in result.pushes() {
                    text.push_str(&format!(
                        "\n推送记录：{} / {}",
                        safe(&p.target().branch),
                        if p.error().is_some() {
                            "失败"
                        } else {
                            "成功"
                        }
                    ));
                }
            }
        }
    }
    text
}
