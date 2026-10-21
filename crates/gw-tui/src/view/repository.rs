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
        let selected = rows.get(screen.selected);
        let title = selected
            .map(|row| name(&row.path))
            .unwrap_or_else(|| "无选中仓库".into());
        let text = selected
            .map(detail_text)
            .unwrap_or_else(|| "当前没有仓库".into());
        frame.render_widget(
            Paragraph::new(text)
                .wrap(Wrap { trim: false })
                .block(block(title)),
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
        frame.render_widget(Paragraph::new("当前筛选没有仓库；/add 添加仓库"), inner);
    }
}
fn field(label: &str, value: impl Into<String>, color: Color) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label}："), Style::default().fg(MUTED)),
        Span::styled(value.into(), Style::default().fg(color)),
    ])
}

pub(super) fn detail_text(row: &Row) -> Text<'static> {
    let mut lines = vec![field(
        "路径",
        safe(&row.path.to_string_lossy()),
        Color::Reset,
    )];
    let Some(result) = &row.result else {
        lines.push(field("状态", "未检查", MUTED));
        lines.push(Line::from("上次报告未包含此仓库，等待定时或显式检查。"));
        return Text::from(lines);
    };
    let color = match result {
        RepositoryResult::Clean { .. } => GOOD,
        RepositoryResult::Attention { .. } => WARN,
        RepositoryResult::Failed { .. } => BAD,
    };
    lines.push(field("状态", result.label(), color));
    match result {
        RepositoryResult::Failed { error, .. } => {
            let summary = match failure_label(error) {
                "TLS失败" => "远程 TLS 连接失败（SSL_ERROR_SYSCALL）",
                "引用不符" => "上游引用不一致：远程更新或 fetch 配置未更新该上游",
                _ => "检查失败，详情见原始错误",
            };
            lines.push(field("失败原因", summary, BAD));
            for (label, value) in [
                ("工作区", "未知"),
                ("远程检查", "未完成"),
                ("分支统计", "未知"),
            ] {
                lines.push(field(label, value, MUTED));
            }
            lines.push(Line::default());
            lines.push(field("原始错误", "", Color::Reset));
            lines.extend(
                multiline(error)
                    .lines()
                    .map(|line| Line::raw(line.to_owned())),
            );
        }
        RepositoryResult::Clean { .. } => {
            lines.push(field("工作区", "干净", GOOD));
            lines.push(field("远程检查", "通过", GOOD));
        }
        RepositoryResult::Attention { issues, .. } => {
            for issue in issues {
                let (label, color) = match issue.kind {
                    IssueKind::Dirty => ("工作区", WARN),
                    IssueKind::DetachedHead => ("HEAD 状态", WARN),
                    IssueKind::NoCommits => ("提交状态", WARN),
                    IssueKind::MissingUpstream => ("远程上游", WARN),
                    IssueKind::OutOfSync => ("远程同步", WARN),
                    IssueKind::PushFailed => ("推送失败", BAD),
                };
                lines.push(field(label, safe(&issue.detail), color));
            }
        }
    }
    if let Some(branches) = result.branches() {
        lines.push(Line::default());
        lines.push(field(
            "未配置远程上游",
            format!("{} 个分支", branches.len()),
            Color::Reset,
        ));
        for branch in branches {
            lines.push(Line::raw(format!(
                "• {}{}",
                safe(&branch.branch),
                if branch.reason == gw_core::domain::UpstreamReason::LocalUpstream {
                    "（跟踪本地分支）"
                } else {
                    ""
                }
            )));
        }
    }
    if !result.pushes().is_empty() {
        lines.push(Line::default());
        lines.push(field("推送记录", "", Color::Reset));
    }
    for push in result.pushes() {
        let target = push.target();
        lines.push(field("分支", safe(&target.branch), Color::Reset));
        lines.push(field(
            "推送目标",
            format!("{}/{}", safe(&target.remote), safe(&target.remote_ref)),
            Color::Reset,
        ));
        match push {
            Publication::Pushed { .. } => lines.push(field(
                "推送结果",
                format!("已推送（{} 个提交）", target.ahead_commits),
                GOOD,
            )),
            Publication::Failed { phase, error, .. } => {
                let phase = match phase {
                    gw_core::domain::Phase::Validate => "校验",
                    gw_core::domain::Phase::Transfer => "传输",
                    gw_core::domain::Phase::Verify => "核验",
                };
                lines.push(field("推送结果", format!("{phase}阶段失败"), BAD));
                lines.push(field("推送错误", "", Color::Reset));
                lines.extend(
                    multiline(error)
                        .lines()
                        .map(|line| Line::raw(line.to_owned())),
                );
            }
        }
    }
    Text::from(lines)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gw_core::domain::{Issue, Issues};

    #[test]
    fn detail_values_have_labels_for_each_report_state() {
        let path = std::path::PathBuf::from("/example/alpha");
        let row = |result| Row {
            path: path.clone(),
            result,
        };
        let missing = detail_text(&row(None)).to_string();
        assert!(missing.starts_with("路径：/example/alpha\n状态：未检查\n"));
        let clean = detail_text(&row(Some(RepositoryResult::Clean {
            path: path.clone(),
            branches_without_upstream: vec![],
            pushes: vec![],
        })))
        .to_string();
        assert!(clean.contains("状态：正常\n工作区：干净\n远程检查：通过"));
        assert!(clean.contains("未配置远程上游：0 个分支"));
        let attention = detail_text(&row(Some(RepositoryResult::Attention {
            path: path.clone(),
            issues: Issues::new(
                Issue {
                    kind: IssueKind::Dirty,
                    detail: "有未提交文件".into(),
                },
                vec![Issue {
                    kind: IssueKind::OutOfSync,
                    detail: "main 落后 2 个提交".into(),
                }],
            ),
            branches_without_upstream: vec![],
            pushes: vec![],
        })))
        .to_string();
        assert!(attention.contains("工作区：有未提交文件"));
        assert!(attention.contains("远程同步：main 落后 2 个提交"));
        let failed = detail_text(&row(Some(RepositoryResult::Failed {
            path: path.clone(),
            error: "SSL_ERROR_SYSCALL".into(),
            pushes: vec![],
        })))
        .to_string();
        assert!(failed.contains("状态：检查失败"));
        assert!(failed.contains("工作区：未知\n远程检查：未完成\n分支统计：未知"));
        assert!(failed.contains("原始错误：\nSSL_ERROR_SYSCALL"));
    }
    #[test]
    fn detail_label_and_value_styles_follow_semantic_state() {
        let path = std::path::PathBuf::from("/example/alpha");
        let results = [
            (None, MUTED),
            (
                Some(RepositoryResult::Clean {
                    path: path.clone(),
                    branches_without_upstream: vec![],
                    pushes: vec![],
                }),
                GOOD,
            ),
            (
                Some(RepositoryResult::Attention {
                    path: path.clone(),
                    issues: Issues::new(
                        Issue {
                            kind: IssueKind::Dirty,
                            detail: "未提交文件".into(),
                        },
                        vec![],
                    ),
                    branches_without_upstream: vec![],
                    pushes: vec![],
                }),
                WARN,
            ),
            (
                Some(RepositoryResult::Failed {
                    path: path.clone(),
                    error: "连接失败\n地址：https://example.invalid".into(),
                    pushes: vec![],
                }),
                BAD,
            ),
        ];
        for (result, expected) in results {
            let text = detail_text(&Row {
                path: path.clone(),
                result,
            });
            assert_eq!(text.lines[0].spans[0].style.fg, Some(MUTED));
            assert_eq!(text.lines[0].spans[1].style.fg, Some(Color::Reset));
            assert_eq!(text.lines[1].spans[0].style.fg, Some(MUTED));
            assert_eq!(text.lines[1].spans[1].style.fg, Some(expected));
            if expected == BAD {
                let error = text
                    .lines
                    .iter()
                    .find(|line| line.to_string().starts_with("地址："))
                    .unwrap();
                assert_eq!(error.spans.len(), 1);
                assert_eq!(error.spans[0].style.fg, None);
            }
        }
    }
}
