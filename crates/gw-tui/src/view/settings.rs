use super::*;
pub(super) fn settings(frame: &mut Frame, screen: &mut Screen, area: Rect) {
    let mut lines = vec![Line::from(gw_i18n::tr!(ChecksAndPushes)), Line::from("")];
    for (i, action) in command::SETTINGS.iter().enumerate() {
        let value = match action {
            Action::Interval => {
                gw_i18n::tf!(EveryHours, screen.config.interval_seconds as f64 / 3600.0)
            }
            Action::Autopush => screen.config.push_policy.to_string(),
            Action::Concurrency => screen.config.max_concurrency.to_string(),
            _ => String::new(),
        };
        lines.push(Line::styled(
            format!(
                "{} {}  {value}",
                if screen.setting_selected == i {
                    "›"
                } else {
                    " "
                },
                command::description(*action)
            ),
            if screen.setting_selected == i {
                selected_style()
            } else {
                Style::default()
            },
        ));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(gw_i18n::tf!(
        SystemSchedule,
        if screen.snapshot.registration == Registration::Loaded {
            gw_i18n::tr!(Loaded)
        } else {
            gw_i18n::tr!(NotLoaded)
        },
        if matches!(screen.snapshot.activity, Activity::Running) {
            gw_i18n::tr!(CheckingAltAlt)
        } else {
            gw_i18n::tr!(Idle)
        }
    )));
    if let Schedule::Saved { interval_seconds } = screen.snapshot.schedule
        && interval_seconds != screen.config.interval_seconds
    {
        lines.push(Line::from(gw_i18n::tf!(
            AppliedIntervalHoursConfigurationNotYetApplied,
            interval_seconds as f64 / 3600.0
        )))
    }
    lines.push(Line::from(""));
    lines.push(Line::from(gw_i18n::tr!(ExcludedDirectories)));
    for path in &screen.config.excluded_roots {
        lines.push(Line::from(safe(&path.to_string_lossy())))
    }
    let start = if screen.setting_selected + 3 >= area.height.saturating_sub(2) as usize {
        screen.setting_selected + 4 - area.height.saturating_sub(2) as usize
    } else {
        screen.scroll
    };
    let text = Text::from(lines);
    let paragraph = Paragraph::new(text)
        .wrap(Wrap { trim: false })
        .block(block(gw_i18n::tr!(Settings)));
    frame.render_widget(
        paragraph.scroll((start.min(u16::MAX as usize) as u16, 0)),
        area,
    );
}
