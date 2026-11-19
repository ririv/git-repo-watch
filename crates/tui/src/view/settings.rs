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
    if action == Action::Workflow {
        return Some(screen.config.scheduled_actions.summary());
    }
    if action == Action::AutoFetchInterval {
        return Some(engine::scheduler::duration::compact(
            screen.config.auto_fetch.interval_seconds,
        ));
    }
    if action == Action::AutoFetchMenu {
        return Some(
            if screen.config.auto_fetch.enabled {
                i18n::tr!(TaskEnabled)
            } else {
                i18n::tr!(TaskDisabled)
            }
            .into(),
        );
    }
    if action == Action::StartupFetchMenu {
        return Some(
            if screen.config.fetch_on_startup {
                i18n::tr!(TaskEnabled)
            } else {
                i18n::tr!(TaskDisabled)
            }
            .into(),
        );
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
    let repository = match &screen.mode {
        Mode::RepositoryEnvironment(path) => Some(path.as_path()),
        Mode::Busy(pending) => match pending.origin.as_ref() {
            Mode::RepositoryEnvironment(path) => Some(path.as_path()),
            _ => None,
        },
        _ => None,
    };
    let environment = repository.and_then(|path| screen.config.repository_environment.get(path));
    let panel = block(repository.map_or_else(
        || section.title().to_owned(),
        |path| format!("{} · {}", section.title(), name(path)),
    ));
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
            Item::Command(Action::RepositoryEnvEdit(command::EnvironmentField::Path)) => {
                i18n::tr!(GitEnvironmentInheritedPath).into()
            }
            Item::Command(Action::RepositoryEnvEdit(command::EnvironmentField::Set)) => environment
                .map(|env| env.variables.keys().cloned().collect::<Vec<_>>().join(", "))
                .unwrap_or_default(),
            Item::Command(Action::RepositoryEnvEdit(field)) => safe(&field.value(environment)),
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
        let style = if screen.setting_selected == i {
            selected_style()
        } else {
            Style::default()
        };
        let entry = if matches!(
            item,
            Item::Command(Action::RepositoryEnvEdit(command::EnvironmentField::Path))
        ) && let Some((root, environment)) = repository.zip(environment)
            && !environment.path_prepend.is_empty()
        {
            std::iter::once(Line::styled(item.description(), style))
                .chain(environment.path_directories(root).map(|path| {
                    Line::styled(format!("  {}", safe(&path.display().to_string())), style)
                }))
                .collect::<Vec<_>>()
        } else {
            vec![Line::styled(
                format!("{}  {value}", item.description()),
                style,
            )]
        };
        let height = Paragraph::new(Text::from(entry.clone()))
            .wrap(Wrap { trim: false })
            .line_count(inner.width);
        item_rows.push((i, row, height));
        row += height;
        lines.extend(entry);
    }
    let (selected_row, selected_height) = item_rows
        .iter()
        .find(|(index, _, _)| *index == screen.setting_selected)
        .map(|(_, row, height)| (*row, *height))
        .unwrap_or((0, 0));
    let viewport_height = usize::from(inner.height);
    let selected_end = selected_row + selected_height;
    let start =
        if section == Configuration::RepositoryEnvironment && selected_height > viewport_height {
            screen
                .scroll
                .clamp(selected_row, selected_end - viewport_height)
        } else if selected_end >= viewport_height {
            selected_end.saturating_sub(viewport_height)
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
