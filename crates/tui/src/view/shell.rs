mod summary;
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
            if crate::workspace::Section::focused(&screen.mode).is_some() {
                format!("  · {} · {size_hint}", i18n::tr!(WorkspaceFocusHint))
            } else {
                format!("  · {}", i18n::tr!(WorkspaceFocusHint))
            },
            Style::default().fg(MUTED),
        ));
    }
    if !matches!(screen.mode, Mode::CompositionWaiting(_))
        && let Some(task) = screen
            .scope_path()
            .and_then(|path| screen.repository_tasks.get(&path))
    {
        lines[0].spans.push(Span::styled(
            format!("  {} {}", task.animation.symbol(), safe(&task.title)),
            Style::default().fg(ACCENT),
        ));
    } else if !matches!(screen.mode, Mode::CompositionWaiting(_))
        && !screen.repository_tasks.is_empty()
    {
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
        lines.extend(summary::lines(screen, area.width));
    }
    Text::from(lines)
}
