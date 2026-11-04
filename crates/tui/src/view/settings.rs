mod ai;
mod general;
use super::*;
use crate::settings::Item;
pub(super) fn annotation(action: Action, screen: &Screen) -> Option<String> {
    if action == Action::EditorSetting {
        return Some(screen.config.editor.label());
    }
    if action == Action::ScheduleEnabledMenu {
        return Some(
            match screen.snapshot.registration {
                Registration::Loaded => i18n::tr!(TaskEnabled),
                Registration::NotLoaded => i18n::tr!(TaskDisabled),
            }
            .into(),
        );
    }
    if let Action::Workflow(trigger) = action {
        return Some(match trigger {
            engine::workflow::Trigger::Startup => screen.config.startup_actions.summary(),
            engine::workflow::Trigger::Scheduled => screen.config.scheduled_actions.summary(),
        });
    }
    if action == Action::WrapMenu {
        return Some(
            if screen.config.text_wrap {
                i18n::tr!(TaskEnabled)
            } else {
                i18n::tr!(TaskDisabled)
            }
            .into(),
        );
    }
    ai::annotation(action, screen)
}
pub(super) fn settings(frame: &mut Frame, screen: &mut Screen, area: Rect, section: Configuration) {
    if section == Configuration::General {
        general::render(frame, screen, area, section);
        return;
    }
    let mut lines = Vec::new();
    let panel = block(section.title());
    let inner = panel.inner(area);
    let mut item_rows = Vec::new();
    let mut row = 0usize;
    let mut mouse_heading = false;
    for (i, item) in section
        .items(screen.snapshot.registration, &screen.config.ai)
        .iter()
        .enumerate()
    {
        if matches!(item, Item::Mouse(_)) && !mouse_heading {
            lines.push(Line::default());
            lines.push(Line::styled(
                i18n::tr!(MouseSettings),
                Style::default().fg(ACCENT).bold(),
            ));
            row += 2;
            mouse_heading = true;
        }
        let value = match item {
            Item::Mouse(setting) => if screen.config.mouse.enabled(*setting) {
                i18n::tr!(TaskEnabled)
            } else {
                i18n::tr!(TaskDisabled)
            }
            .into(),
            Item::Command(Action::Interval) => {
                engine::scheduler::duration::compact(screen.config.interval_seconds)
            }
            Item::Command(Action::Anchor) => screen.config.schedule_anchor.display(),
            Item::Command(Action::Autopush) => {
                screen.config.scheduled_actions.push_policy().to_string()
            }
            Item::Command(Action::Concurrency) => screen.config.max_concurrency.to_string(),
            Item::Command(Action::StartupCheck) => if screen.config.check_on_startup {
                i18n::tr!(TaskEnabled)
            } else {
                i18n::tr!(TaskDisabled)
            }
            .into(),
            Item::ChineseCommandTrigger => if screen.config.chinese_command_trigger {
                i18n::tr!(TaskEnabled)
            } else {
                i18n::tr!(TaskDisabled)
            }
            .into(),
            Item::Command(Action::Remove) => screen
                .selected_path()
                .map(|path| name(&path))
                .unwrap_or_else(|| i18n::tr!(NoRepositorySelected).into()),
            _ => String::new(),
        };
        let line = Line::styled(
            format!("{}  {value}", item.description()),
            if screen.setting_selected == i {
                selected_style()
            } else {
                Style::default()
            },
        );
        let height = Paragraph::new(line.clone())
            .wrap(Wrap { trim: false })
            .line_count(inner.width);
        item_rows.push((i, row, height));
        row += height;
        lines.push(line);
    }
    let (selected_row, selected_height) = item_rows
        .iter()
        .find(|(index, _, _)| *index == screen.setting_selected)
        .map(|(_, row, height)| (*row, *height))
        .unwrap_or((0, 0));
    let start = if selected_row + selected_height >= inner.height as usize {
        (selected_row + selected_height).saturating_sub(inner.height as usize)
    } else {
        screen.scroll
    };
    let text = Text::from(lines);
    let text = screen.pointer.links.text(text);
    let paragraph = Paragraph::new(text.plain()).wrap(Wrap { trim: false });
    let lines = paragraph.line_count(inner.width);
    let start = start.min(lines.saturating_sub(inner.height as usize));
    for (index, row, height) in item_rows {
        let first = row.max(start);
        let end = (row + height).min(start + inner.height as usize);
        if first < end {
            screen.pointer.item(
                Rect::new(
                    inner.x,
                    inner.y + (first - start) as u16,
                    inner.width,
                    (end - first) as u16,
                ),
                crate::pointer::Target::Settings,
                index,
            );
        }
    }
    screen.pointer.vertical(
        area,
        inner,
        lines.saturating_sub(inner.height as usize),
        start,
        crate::pointer::ScrollTarget::Text,
    );
    frame.render_widget(panel, area);
    screen.pointer.links.paragraph(frame, inner, &text, start);
    crate::scrollbar::vertical(frame, area, inner, lines, start);
}
