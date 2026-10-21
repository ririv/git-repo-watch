use super::*;
pub(super) fn header(screen: &Screen) -> Text<'static> {
    let title = match &screen.mode {
        Mode::Files(_) | Mode::Diff { .. } => i18n::tr!(FilesCurrentState),
        Mode::CommitMessage { .. } | Mode::CommitConfirm { .. } => {
            i18n::tr!(CommitStagedContents)
        }
        Mode::PushConfirm { .. } => i18n::tr!(PushCurrentBranch),
        Mode::Error { title, .. } => title.as_str(),
        _ => match screen.page {
            Page::Repositories => i18n::tr!(Repositories),
            Page::Detail => i18n::tr!(RepositoryDetails),
            Page::Settings => i18n::tr!(Settings),
            Page::Help => i18n::tr!(Help),
        },
    };
    let mut lines = vec![Line::from(vec![
        Span::styled(
            format!(" {} ", identity::COMMAND),
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(" / {}", safe(title))),
    ])];
    if matches!(screen.mode, Mode::Browsing)
        && matches!(screen.page, Page::Repositories | Page::Detail)
    {
        if matches!(screen.snapshot.activity, Activity::Running) {
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
        match &screen.snapshot.report {
            CachedReport::Missing => lines.push(Line::from(i18n::tr!(NoCompleteReportYet))),
            CachedReport::Available { report, scope } => {
                if screen.page == Page::Repositories {
                    let stats = Stats::of(report);
                    lines.push(Line::from(i18n::tf!(
                        CleanAttentionFailed,
                        stats.clean,
                        stats.attention,
                        stats.failed
                    )))
                }
                lines.push(Line::from(i18n::tf!(
                    LastCheck,
                    local_time(&report.finished_at),
                    if matches!(scope, Scope::Outdated) {
                        i18n::tr!(ReportScopeChanged)
                    } else {
                        ""
                    }
                )))
            }
        }
    }
    Text::from(lines)
}
