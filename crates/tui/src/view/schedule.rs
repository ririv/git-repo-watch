//! Configured values and the applied task's next occurrence remain visibly distinct.
use super::*;
use engine::scheduler::{duration, timing::Anchor};

pub(super) fn render(frame: &mut Frame, screen: &mut Screen, area: Rect) {
    let panel = block(i18n::tr!(ScheduleSettings));
    let inner = panel.inner(area);
    frame.render_widget(panel, area);
    let parts = Layout::vertical([
        Constraint::Length(8.min(inner.height.saturating_sub(3))),
        Constraint::Min(1),
    ])
    .split(inner);
    let (status, color) = match screen.snapshot.registration {
        Registration::Loaded => (i18n::tr!(TaskEnabled), GOOD),
        Registration::NotLoaded => match screen.snapshot.schedule {
            Schedule::Missing => (i18n::tr!(TaskMissing), MUTED),
            Schedule::Saved { .. } | Schedule::Anchored { .. } => (i18n::tr!(TaskDisabled), WARN),
        },
    };
    let label = |label: &str| Span::styled(format!("{label}: "), Style::default().fg(MUTED));
    let application = match &screen.snapshot.schedule {
        Schedule::Missing => {
            Span::styled(i18n::tr!(IntervalUninstalled), Style::default().fg(WARN))
        }
        Schedule::Anchored { plan }
            if plan.interval_seconds() == screen.config.interval_seconds
                && screen.config.schedule_anchor == Anchor::Set(plan.basis().clone()) =>
        {
            Span::styled(i18n::tr!(IntervalApplied), Style::default().fg(GOOD))
        }
        Schedule::Saved { interval_seconds } => Span::styled(
            i18n::tf!(IntervalPending, duration::compact(*interval_seconds)),
            Style::default().fg(WARN),
        ),
        Schedule::Anchored { plan } => Span::styled(
            i18n::tf!(IntervalPending, duration::compact(plan.interval_seconds())),
            Style::default().fg(WARN),
        ),
    };
    let next = match screen
        .snapshot
        .schedule
        .next_trigger(screen.snapshot.registration, chrono::Local::now())
    {
        Ok(next) => next.display(),
        Err(error) => format!("{error:#}"),
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
            Span::raw(duration::compact(screen.config.interval_seconds)),
        ]),
        Line::from(vec![
            label(i18n::tr!(ScheduleAnchorLabel)),
            Span::raw(screen.config.schedule_anchor.display()),
        ]),
        Line::from(vec![
            label(i18n::tr!(ScheduleNextLabel)),
            Span::styled(next, Style::default().fg(ACCENT)),
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
                Action::Anchor => i18n::tr!(EditScheduleAnchor),
                Action::Autopush => i18n::tr!(EditAutopush),
                Action::Apply => i18n::tr!(ActionApply),
                Action::EnableSchedule => i18n::tr!(EnableScheduledTask),
                Action::DisableSchedule => i18n::tr!(DisableScheduledTask),
                _ => unreachable!("schedule action catalog"),
            })
        })
        .collect::<Vec<_>>();
    let mut state = ListState::default()
        .with_offset(screen.scroll)
        .with_selected(Some(screen.setting_selected));
    frame.render_stateful_widget(
        List::new(items).highlight_style(selected_style()),
        parts[1],
        &mut state,
    );
    screen.scroll = state.offset();
    crate::scrollbar::vertical(frame, area, parts[1], actions.len(), state.offset());
    screen.pointer.list(
        area,
        parts[1],
        &(0..actions.len()).collect::<Vec<_>>(),
        state.offset(),
        crate::pointer::Target::Settings,
    );
}
