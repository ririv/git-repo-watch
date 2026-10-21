use super::*;
pub(super) fn render_body(frame: &mut Frame, screen: &mut Screen, area: Rect) {
    match &mut screen.mode {
        Mode::Files(s) => {
            let title = format!("{} · {}", name(&s.snapshot.path), safe(s.snapshot.branch()));
            let frame_block = block(title);
            let inner = frame_block.inner(area);
            frame.render_widget(frame_block, area);
            let parts = Layout::vertical([Constraint::Length(2), Constraint::Min(1)]).split(inner);
            let rows = s.rows();
            frame.render_widget(
                Paragraph::new(format!(
                    "{} · {} 个文件 · 已选 {}",
                    if s.area == FileArea::Index {
                        "暂存区"
                    } else {
                        "工作区"
                    },
                    rows.len(),
                    s.selected.len()
                )),
                parts[0],
            );
            let items = rows
                .iter()
                .map(|c| {
                    ListItem::new(format!(
                        "[{}] {} {}",
                        if s.selected.contains(&s.change_index(c)) {
                            "✓"
                        } else {
                            " "
                        },
                        c.kind,
                        safe(&c.location.display())
                    ))
                })
                .collect::<Vec<_>>();
            let mut state = ListState::default().with_selected(if items.is_empty() {
                None
            } else {
                Some(s.cursor)
            });
            frame.render_stateful_widget(
                List::new(if items.is_empty() {
                    vec![ListItem::new("此区域没有变更")]
                } else {
                    items
                })
                .highlight_style(selected_style())
                .highlight_symbol("› "),
                parts[1],
                &mut state,
            );
        }
        Mode::Diff {
            content, scroll, ..
        } => text_view(frame, area, &multiline(content), scroll, "文件差异"),
        Mode::CommitMessage { plan, scroll, .. } => {
            text_view(frame, area, &commit_review(plan), scroll, "提交预览")
        }
        Mode::CommitConfirm {
            plan,
            message,
            scroll,
        } => text_view(
            frame,
            area,
            &format!("提交说明：{}\n\n{}", safe(message), commit_review(plan)),
            scroll,
            "确认提交",
        ),
        Mode::PushConfirm { plan, scroll } => text_view(
            frame,
            area,
            &format!(
                "仓库：{}\n分支：{}\n远程：{}\n上游：{}\n地址：{}\n待推送：{} 个提交\n\n仅发布预览时的提交快照；普通推送，保留 pre-push 钩子。",
                safe(&plan.path.to_string_lossy()),
                safe(&plan.target.branch),
                safe(&plan.target.remote),
                safe(&plan.target.remote_ref),
                safe(&plan.url),
                plan.target.ahead_commits
            ),
            scroll,
            "确认推送",
        ),
        Mode::Error { detail, scroll, .. } => {
            text_view(frame, area, &multiline(detail), scroll, "操作结果")
        }
        Mode::Input { action, .. } => frame.render_widget(
            Paragraph::new(command::description(*action))
                .wrap(Wrap { trim: false })
                .block(block("输入配置")),
            area,
        ),
        Mode::Confirm(confirmation) => {
            let text = match confirmation {
                Confirmation::CheckAll => format!(
                    "检查全部 {} 个注册仓库\n并发上限：{}\n自动推送策略：{}\n\n共享 Git 存储的工作区串行检查；仅领先且不落后的已提交内容可自动推送。",
                    screen.config.repositories.len(),
                    screen.config.max_concurrency,
                    screen.config.push_policy
                ),
                Confirmation::Apply => {
                    "应用已保存的检查间隔，重新加载系统任务。不会立即检查或推送。".into()
                }
                Confirmation::Remove(path) => format!(
                    "移除仓库注册项：{}\n\n保留磁盘文件。",
                    safe(&path.to_string_lossy())
                ),
                Confirmation::Configure { action, value } => {
                    format!("{}\n\n{}", command::description(*action), safe(value))
                }
            };
            frame.render_widget(
                Paragraph::new(text)
                    .wrap(Wrap { trim: false })
                    .block(block("确认操作")),
                area,
            );
        }
        _ => match screen.page {
            Page::Repositories => repositories(frame, screen, area),
            Page::Detail => {
                let rows = screen.rows();
                let text = rows
                    .get(screen.selected)
                    .map(detail_text)
                    .unwrap_or_else(|| "当前仓库不在筛选结果中；Esc 返回。".into());
                review::styled_text_view(frame, area, text, &mut screen.scroll, "仓库详情")
            }
            Page::Settings => settings(frame, screen, area),
            Page::Help => text_view(frame, area, HELP, &mut screen.scroll, "操作说明"),
        },
    }
}
