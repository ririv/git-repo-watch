use super::*;
pub(super) fn settings(frame: &mut Frame, screen: &mut Screen, area: Rect, section: Configuration) {
    let mut lines = Vec::new();
    for (i, action) in section
        .actions(screen.snapshot.registration)
        .iter()
        .enumerate()
    {
        let value = match action {
            Action::Interval => {
                i18n::tf!(EveryHours, screen.config.interval_seconds as f64 / 3600.0)
            }
            Action::Autopush => screen.config.push_policy.to_string(),
            Action::Concurrency => screen.config.max_concurrency.to_string(),
            Action::Remove => screen
                .selected_path()
                .map(|path| name(&path))
                .unwrap_or_else(|| i18n::tr!(NoRepositorySelected).into()),
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
    if section == Configuration::Repositories {
        lines.push(Line::from(""));
        lines.push(Line::from(i18n::tr!(ExcludedDirectories)));
        for path in &screen.config.excluded_roots {
            lines.push(Line::from(safe(&path.to_string_lossy())))
        }
    }
    let start = if screen.setting_selected + 1 >= area.height.saturating_sub(2) as usize {
        screen.setting_selected + 2 - area.height.saturating_sub(2) as usize
    } else {
        screen.scroll
    };
    let text = Text::from(lines);
    let paragraph = Paragraph::new(text)
        .wrap(Wrap { trim: false })
        .block(block(section.title()));
    frame.render_widget(
        paragraph.scroll((start.min(u16::MAX as usize) as u16, 0)),
        area,
    );
}
