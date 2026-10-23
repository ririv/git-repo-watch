use super::*;
pub(super) fn header(screen: &mut Screen, area: Rect) -> Text<'static> {
    let mut lines = vec![super::breadcrumbs::render(screen, area)];
    if screen.page == Page::Detail && crate::workspace::Workspace::is_root(&screen.mode) {
        let size_hint = match screen
            .workspace
            .session()
            .map(|session| session.arrangement)
        {
            Some(crate::workspace::Arrangement::Maximized) => i18n::tr!(WorkspaceRestoreHint),
            _ => i18n::tr!(WorkspaceMaximizeHint),
        };
        lines[0].spans.push(Span::styled(
            format!("  · {} · {size_hint}", i18n::tr!(WorkspaceFocusHint)),
            Style::default().fg(MUTED),
        ));
    }
    if let Some(task) = screen
        .scope_path()
        .and_then(|path| screen.repository_tasks.get(&path))
    {
        lines[0].spans.push(Span::styled(
            format!("  {} {}", task.animation.symbol(), safe(&task.title)),
            Style::default().fg(ACCENT),
        ));
    } else if !screen.repository_tasks.is_empty() {
        lines[0].spans.push(Span::styled(
            i18n::tf!(
                RepositoryTasksRunning,
                screen
                    .repository_tasks
                    .values()
                    .next()
                    .expect("active task")
                    .animation
                    .symbol(),
                screen.repository_tasks.len()
            ),
            Style::default().fg(ACCENT),
        ));
    }
    if let Some(task) = &screen.check_task {
        lines[0]
            .spans
            .push(Span::styled(task.summary(), Style::default().fg(ACCENT)));
    }
    if matches!(screen.mode, Mode::Browsing) && screen.page == Page::Repositories {
        if screen.check_task.is_none() && matches!(screen.snapshot.activity, Activity::Running) {
            lines[0].spans.push(Span::styled(
                i18n::tr!(CheckingAlt),
                Style::default().fg(WARN),
            ))
        }
        if screen.snapshot.registration == Registration::NotLoaded
            && screen.page == Page::Repositories
        {
            lines[0].spans.push(Span::styled(
                i18n::tr!(ScheduleNotLoaded),
                Style::default().fg(WARN),
            ))
        }
        if screen.page == Page::Repositories {
            let known = screen
                .config
                .repositories
                .iter()
                .filter_map(|path| screen.result_for(path))
                .collect::<Vec<_>>();
            if !known.is_empty() {
                let stats = Stats::repositories(known);
                lines.push(Line::from(i18n::tf!(
                    CleanAttentionFailed,
                    stats.clean,
                    stats.attention,
                    stats.failed
                )));
            }
        }
        match &screen.snapshot.report {
            CachedReport::Missing => lines.push(Line::from(i18n::tr!(NoCompleteReportYet))),
            CachedReport::Available { report, scope } => lines.push(Line::from(i18n::tf!(
                LastCheck,
                local_time(&report.finished_at),
                if matches!(scope, Scope::Outdated) {
                    i18n::tr!(ReportScopeChanged)
                } else {
                    ""
                }
            ))),
        }
    }
    Text::from(lines)
}
