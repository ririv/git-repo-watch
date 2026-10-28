//! Read-only schedule values and their explicitly associated actions.
use super::*;
use engine::scheduler::timing::Anchor;

pub(super) enum Interaction {
    ReadOnly,
    Button { index: usize, label: &'static str },
}
pub(super) struct Entry {
    pub(super) label: &'static str,
    pub(super) value: String,
    pub(super) color: Color,
    pub(super) interaction: Interaction,
}
pub(super) fn entries(screen: &Screen) -> [Entry; 7] {
    let actions = Configuration::Schedule.actions(screen.snapshot.registration);
    let button = |action, label| Interaction::Button {
        index: actions
            .iter()
            .position(|item| *item == action)
            .expect("schedule action"),
        label,
    };
    let (status, color, action, label) = match screen.snapshot.registration {
        Registration::Loaded => (
            i18n::tr!(TaskEnabled),
            GOOD,
            Action::DisableSchedule,
            i18n::tr!(ScheduleDisableButton),
        ),
        Registration::NotLoaded => (
            match screen.snapshot.schedule {
                Schedule::Missing => i18n::tr!(TaskMissing),
                Schedule::Saved { .. } | Schedule::Anchored { .. } => i18n::tr!(TaskDisabled),
            },
            WARN,
            Action::EnableSchedule,
            i18n::tr!(ScheduleEnableButton),
        ),
    };
    let (application, application_color) = match &screen.snapshot.schedule {
        Schedule::Missing => (i18n::tr!(IntervalUninstalled).into(), WARN),
        Schedule::Anchored { plan }
            if plan.interval_seconds() == screen.config.interval_seconds
                && screen.config.schedule_anchor == Anchor::Set(plan.basis().clone()) =>
        {
            (i18n::tr!(IntervalApplied).into(), GOOD)
        }
        Schedule::Saved { interval_seconds } => (
            i18n::tf!(IntervalPending, duration::compact(*interval_seconds)),
            WARN,
        ),
        Schedule::Anchored { plan } => (
            i18n::tf!(IntervalPending, duration::compact(plan.interval_seconds())),
            WARN,
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
    [
        Entry {
            label: i18n::tr!(TaskStatusLabel),
            value: status.into(),
            color,
            interaction: button(action, label),
        },
        Entry {
            label: i18n::tr!(CheckProcessLabel),
            value: if screen.check_task.is_some()
                || matches!(screen.snapshot.activity, Activity::Running)
            {
                i18n::tr!(CheckingAltAlt)
            } else {
                i18n::tr!(Idle)
            }
            .into(),
            color: MUTED,
            interaction: Interaction::ReadOnly,
        },
        Entry {
            label: i18n::tr!(ScheduleNextLabel),
            value: next,
            color: ACCENT,
            interaction: Interaction::ReadOnly,
        },
        Entry {
            label: i18n::tr!(IntervalLabel),
            value: duration::compact(screen.config.interval_seconds),
            color: Color::LightCyan,
            interaction: button(Action::Interval, i18n::tr!(ScheduleEditButton)),
        },
        Entry {
            label: i18n::tr!(ScheduleAnchorLabel),
            value: screen.config.schedule_anchor.display(),
            color: Color::LightCyan,
            interaction: button(Action::Anchor, i18n::tr!(ScheduleEditButton)),
        },
        Entry {
            label: i18n::tr!(WorkflowScheduledActions),
            value: screen.config.scheduled_actions.summary(),
            color: Color::LightCyan,
            interaction: button(
                Action::Workflow(engine::workflow::Trigger::Scheduled),
                i18n::tr!(ScheduleWorkflowButton),
            ),
        },
        Entry {
            label: i18n::tr!(ApplicationLabel),
            value: application,
            color: application_color,
            interaction: button(Action::Apply, i18n::tr!(ScheduleApplyButton)),
        },
    ]
}
