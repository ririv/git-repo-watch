use super::*;
pub(super) fn header(screen: &Screen) -> Text<'static> {
    let title = match &screen.mode {
        Mode::Files(_) | Mode::Diff { .. } => "文件 · 当前状态",
        Mode::CommitMessage { .. } | Mode::CommitConfirm { .. } => "提交暂存内容",
        Mode::PushConfirm { .. } => "推送当前分支",
        Mode::Error { title, .. } => title.as_str(),
        _ => match screen.page {
            Page::Repositories => "仓库",
            Page::Detail => "仓库详情",
            Page::Settings => "设置",
            Page::Help => "帮助",
        },
    };
    let mut lines = vec![Line::from(vec![
        Span::styled(
            " gw ",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(" / {}", safe(title))),
    ])];
    if matches!(screen.mode, Mode::Browsing)
        && matches!(screen.page, Page::Repositories | Page::Detail)
    {
        if matches!(screen.snapshot.activity, Activity::Running) {
            lines[0]
                .spans
                .push(Span::styled(" · 正在检查", Style::default().fg(WARN)))
        }
        if screen.snapshot.registration == Registration::NotLoaded
            && screen.page == Page::Repositories
        {
            lines[0]
                .spans
                .push(Span::styled(" · 定时任务未加载", Style::default().fg(WARN)))
        }
        match &screen.snapshot.report {
            CachedReport::Missing => lines.push(Line::from(" 尚无完整报告")),
            CachedReport::Available { report, scope } => {
                if screen.page == Page::Repositories {
                    let stats = Stats::of(report);
                    lines.push(Line::from(format!(
                        " 正常 {} · 需关注 {} · 失败 {}",
                        stats.clean, stats.attention, stats.failed
                    )))
                }
                lines.push(Line::from(format!(
                    " 上次检查：{}{}",
                    local_time(&report.finished_at),
                    if matches!(scope, Scope::Outdated) {
                        " · 报告范围已变化"
                    } else {
                        ""
                    }
                )))
            }
        }
    }
    Text::from(lines)
}
