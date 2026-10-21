//! Dedicated scheduled-task overview and state-dependent management actions.
use super::*;

pub(super) fn render(frame: &mut Frame, screen: &Screen, area: Rect) {
    let panel = block(i18n::tr!(ScheduleSettings));
    let inner = panel.inner(area);
    frame.render_widget(panel, area);
    let parts = Layout::vertical([Constraint::Length(6), Constraint::Min(1)]).split(inner);
    let (status, color) = match screen.snapshot.registration {
        Registration::Loaded => (i18n::tr!(TaskEnabled), GOOD),
        Registration::NotLoaded => match screen.snapshot.schedule {
            Schedule::Missing => (i18n::tr!(TaskMissing), MUTED),
            Schedule::Saved { .. } => (i18n::tr!(TaskDisabled), WARN),
        },
    };
    let label = |label: &str| Span::styled(format!("{label}: "), Style::default().fg(MUTED));
    let application = match screen.snapshot.schedule {
        Schedule::Missing => {
            Span::styled(i18n::tr!(IntervalUninstalled), Style::default().fg(WARN))
        }
        Schedule::Saved { interval_seconds }
            if interval_seconds == screen.config.interval_seconds =>
        {
            Span::styled(i18n::tr!(IntervalApplied), Style::default().fg(GOOD))
        }
        Schedule::Saved { interval_seconds } => Span::styled(
            i18n::tf!(IntervalPending, interval_seconds as f64 / 3600.0),
            Style::default().fg(WARN),
        ),
    };
    let lines = vec![
        Line::from(vec![
            label(i18n::tr!(TaskStatusLabel)),
            Span::styled(status, Style::default().fg(color)),
            Span::raw("   "),
            label(i18n::tr!(CheckProcessLabel)),
            Span::raw(
                if screen.check_task.is_some()
                    || matches!(screen.snapshot.activity, Activity::Running)
                {
                    i18n::tr!(CheckingAltAlt)
                } else {
                    i18n::tr!(Idle)
                },
            ),
        ]),
        Line::from(vec![
            label(i18n::tr!(IntervalLabel)),
            Span::raw(i18n::tf!(
                EveryHours,
                screen.config.interval_seconds as f64 / 3600.0
            )),
        ]),
        Line::from(vec![
            label(i18n::tr!(AutopushLabel)),
            Span::raw(screen.config.push_policy.to_string()),
        ]),
        Line::from(vec![label(i18n::tr!(ApplicationLabel)), application]),
        Line::styled(i18n::tr!(PolicyAppliesNextRun), Style::default().fg(MUTED)),
        Line::from(""),
    ];
    frame.render_widget(Paragraph::new(lines), parts[0]);
    let actions = Configuration::Schedule.actions(screen.snapshot.registration);
    let items = actions
        .iter()
        .map(|action| {
            ListItem::new(match action {
                Action::Interval => i18n::tr!(EditInterval),
                Action::Autopush => i18n::tr!(EditAutopush),
                Action::Apply => i18n::tr!(ActionApply),
                Action::EnableSchedule => i18n::tr!(EnableScheduledTask),
                Action::DisableSchedule => i18n::tr!(DisableScheduledTask),
                _ => unreachable!("schedule action catalog"),
            })
        })
        .collect::<Vec<_>>();
    let mut state = ListState::default().with_selected(Some(screen.setting_selected));
    frame.render_stateful_widget(
        List::new(items)
            .highlight_style(selected_style())
            .highlight_symbol("› "),
        parts[1],
        &mut state,
    );
}
