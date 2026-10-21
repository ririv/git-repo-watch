use super::*;
pub(super) fn settings(frame: &mut Frame, screen: &mut Screen, area: Rect) {
    let mut lines = vec![Line::from("检查与推送"), Line::from("")];
    for (i, action) in command::SETTINGS.iter().enumerate() {
        let value = match action {
            Action::Interval => {
                format!("每 {} 小时", screen.config.interval_seconds as f64 / 3600.0)
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
    lines.push(Line::from(format!(
        "系统任务：{} · {}",
        if screen.snapshot.registration == Registration::Loaded {
            "已加载"
        } else {
            "未加载"
        },
        if matches!(screen.snapshot.activity, Activity::Running) {
            "检查运行中"
        } else {
            "空闲"
        }
    )));
    if let Schedule::Saved { interval_seconds } = screen.snapshot.schedule
        && interval_seconds != screen.config.interval_seconds
    {
        lines.push(Line::from(format!(
            "已应用间隔 {} 小时，配置尚未应用",
            interval_seconds as f64 / 3600.0
        )))
    }
    lines.push(Line::from(""));
    lines.push(Line::from("排除目录"));
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
        .block(block("设置"));
    frame.render_widget(
        paragraph.scroll((start.min(u16::MAX as usize) as u16, 0)),
        area,
    );
}
