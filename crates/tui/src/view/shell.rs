use super::*;
pub(super) fn header(screen: &Screen) -> Text<'static> {
    let repository_title = screen.workspace.path().map(name);
    let title = if screen.page == Page::Detail
        && crate::workspace::Workspace::is_root(&screen.mode)
        && repository_title.is_some()
    {
        repository_title
            .as_deref()
            .expect("open repository has a name")
    } else {
        match &screen.mode {
            Mode::Stashes(_) => i18n::tr!(StashTitle),
            Mode::Branches(_) => i18n::tr!(BranchTitle),
            Mode::History(_) => i18n::tr!(HistoryTitle),
            Mode::Busy(pending) if matches!(pending.origin.as_ref(), Mode::History(_)) => {
                i18n::tr!(HistoryTitle)
            }
            Mode::Files(_) | Mode::Changes(_) => i18n::tr!(ChangesTitle),
            Mode::CommitMessage { .. } | Mode::CommitConfirm { .. } => {
                i18n::tr!(CommitStagedContents)
            }
            Mode::PushConfirm { .. } => i18n::tr!(PushCurrentBranch),
            Mode::Error { title, .. } => title.as_str(),
            _ => match screen.page {
                Page::View(view) => view.title(),
                Page::Repositories => i18n::tr!(Repositories),
                Page::Detail => i18n::tr!(OverviewTitle),
                Page::Configuration(section) => section.title(),
                Page::Help => i18n::tr!(Help),
            },
        }
    };
    let mut lines = vec![Line::from(vec![
        Span::styled(
            format!(" {} ", identity::COMMAND),
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(" / {}", safe(title))),
    ])];
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
