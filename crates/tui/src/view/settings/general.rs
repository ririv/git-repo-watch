//! Grouped settings form with aligned values and item-sized pointer targets.
use super::*;
use engine::state::MouseSetting;
use unicode_width::UnicodeWidthStr;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Group {
    Checks,
    Display,
    Input,
    Pointer,
    Ai,
    Tools,
}
impl Group {
    fn title(self) -> &'static str {
        match self {
            Self::Checks => i18n::tr!(SettingsChecks),
            Self::Display => i18n::tr!(SettingsDisplay),
            Self::Input => i18n::tr!(SettingsInput),
            Self::Pointer => i18n::tr!(MouseSettings),
            Self::Ai => i18n::tr!(Ai),
            Self::Tools => i18n::tr!(ExternalEditorTools),
        }
    }
}
enum Value {
    Toggle(bool),
    Number(usize),
    Interaction(engine::state::InteractionMode),
    Text(String),
}
struct Entry {
    group: Group,
    label: &'static str,
    hint: &'static str,
    value: Value,
}
impl Entry {
    fn new(item: Item, screen: &Screen) -> Self {
        let (group, label, hint, value) = match item {
            Item::Command(Action::EditorSetting) => (
                Group::Tools,
                i18n::tr!(ExternalEditorSetting),
                i18n::tr!(ExternalEditorHint),
                Value::Text(screen.config.editor.label()),
            ),
            Item::Command(Action::Concurrency) => (
                Group::Checks,
                i18n::tr!(SettingsConcurrency),
                i18n::tr!(SettingsConcurrencyHint),
                Value::Number(screen.config.max_concurrency),
            ),
            Item::Command(Action::StartupFetchMenu) => (
                Group::Checks,
                i18n::tr!(StartupFetch),
                i18n::tr!(StartupFetchHint),
                Value::Toggle(screen.config.fetch_on_startup),
            ),
            Item::Command(Action::WrapMenu) => (
                Group::Display,
                i18n::tr!(TextWrap),
                i18n::tr!(TextWrapHint),
                Value::Toggle(screen.config.text_wrap),
            ),
            Item::FileIcons => (
                Group::Display,
                i18n::tr!(FileIcons),
                i18n::tr!(FileIconsHint),
                Value::Toggle(screen.config.file_icons),
            ),
            Item::ChineseCommandTrigger => (
                Group::Input,
                i18n::tr!(SettingsChineseTrigger),
                i18n::tr!(SettingsChineseTriggerHint),
                Value::Toggle(screen.config.chinese_command_trigger),
            ),
            Item::Interaction => (
                Group::Pointer,
                i18n::tr!(InteractionSetting),
                i18n::tr!(InteractionHint),
                Value::Interaction(screen.config.interaction_mode),
            ),
            Item::Mouse(setting) => {
                let (label, hint) = match setting {
                    MouseSetting::PreciseClick => (
                        i18n::tr!(SettingsPreciseClick),
                        i18n::tr!(SettingsPreciseClickHint),
                    ),
                    MouseSetting::Scroll => (i18n::tr!(MouseScroll), i18n::tr!(SettingsScrollHint)),
                    MouseSetting::SelectPane => (
                        i18n::tr!(SettingsSelectPane),
                        i18n::tr!(SettingsSelectPaneHint),
                    ),
                };
                (
                    Group::Pointer,
                    label,
                    hint,
                    Value::Toggle(screen.config.mouse.enabled(setting)),
                )
            }
            Item::AiEnabled => (
                Group::Ai,
                i18n::tr!(AiEnabled),
                i18n::tr!(AiEnabledHint),
                Value::Toggle(screen.config.ai.enabled),
            ),
            Item::Command(action @ (Action::AiAgent | Action::AiModel | Action::AiPrompt)) => {
                let config = &screen.config.ai;
                let (label, hint, value) = match action {
                    Action::AiAgent => (
                        i18n::tr!(AgentChoice),
                        i18n::tr!(AgentChoiceHint),
                        config.agent.label().to_owned(),
                    ),
                    Action::AiModel => (
                        i18n::tr!(AgentModel),
                        i18n::tr!(AgentModelHint),
                        super::ai::model(config),
                    ),
                    Action::AiPrompt => (
                        i18n::tr!(AgentPrompt),
                        i18n::tr!(AgentPromptHint),
                        match &config.prompt {
                            engine::ai::Prompt::Default => i18n::tr!(AgentDefaultPrompt).into(),
                            engine::ai::Prompt::Custom { .. } => {
                                i18n::tr!(AgentCustomPrompt).into()
                            }
                        },
                    ),
                    _ => unreachable!(),
                };
                (Group::Ai, label, hint, Value::Text(value))
            }
            Item::Command(_) => unreachable!("general settings catalog"),
        };
        Self {
            group,
            label,
            hint,
            value,
        }
    }
}

pub(super) fn render(frame: &mut Frame, screen: &mut Screen, area: Rect, section: Configuration) {
    let panel = block(section.title()).padding(ratatui::widgets::Padding::horizontal(1));
    let inner = panel.inner(area);
    let width = inner.width.min(88);
    let entries = section
        .items(screen.snapshot.registration, &screen.config.ai)
        .into_iter()
        .map(|item| Entry::new(item, screen))
        .collect::<Vec<_>>();
    screen.setting_selected = screen.setting_selected.min(entries.len().saturating_sub(1));
    let label_width = entries
        .iter()
        .map(|item| item.label.width())
        .max()
        .unwrap_or(0);
    let value_width = [
        i18n::tr!(TaskEnabled),
        i18n::tr!(TaskDisabled),
        i18n::tr!(InteractionApplication),
        i18n::tr!(InteractionText),
    ]
    .iter()
    .map(|v| v.width() + 2)
    .max()
    .unwrap_or(0);
    let inline = label_width + 4 + value_width <= usize::from(width);
    let mut lines = Vec::new();
    let mut rows = Vec::new();
    let mut group = None;
    let mut row = 0;
    let push = |lines: &mut Vec<Line<'static>>, row: &mut usize, line: Line<'static>| {
        *row += Paragraph::new(line.clone())
            .wrap(Wrap { trim: false })
            .line_count(width)
            .max(1);
        lines.push(line);
    };
    for (index, entry) in entries.iter().enumerate() {
        if group != Some(entry.group) {
            if group.is_some() {
                push(&mut lines, &mut row, Line::default());
            }
            let title = entry.group.title();
            let rule = "─".repeat(usize::from(width).saturating_sub(title.width() + 3));
            push(
                &mut lines,
                &mut row,
                Line::from(vec![
                    Span::styled(
                        format!(" {title} "),
                        Style::default().fg(Color::LightMagenta).bold(),
                    ),
                    Span::styled(rule, Style::default().fg(BORDER)),
                ]),
            );
            group = Some(entry.group);
        }
        let first = row;
        let selected = screen.setting_selected == index;
        let base = if selected {
            selected_style()
        } else {
            Style::default()
        };
        let (value, color) = match &entry.value {
            Value::Toggle(true) => (format!("● {}", i18n::tr!(TaskEnabled)), GOOD),
            Value::Toggle(false) => (format!("○ {}", i18n::tr!(TaskDisabled)), Color::Gray),
            Value::Interaction(mode) => (
                crate::interaction::label(*mode).to_owned(),
                Color::LightCyan,
            ),
            Value::Number(value) => (value.to_string(), Color::LightCyan),
            Value::Text(value) => (value.clone(), Color::LightCyan),
        };
        let mut label = vec![Span::styled(
            entry.label,
            Style::default().fg(Color::White).bold(),
        )];
        if inline {
            label.push(Span::raw(" ".repeat(label_width - entry.label.width() + 4)));
            label.push(Span::styled(
                value.clone(),
                Style::default().fg(color).bold(),
            ));
        }
        push(&mut lines, &mut row, Line::from(label).style(base));
        if !inline {
            push(
                &mut lines,
                &mut row,
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(value, Style::default().fg(color).bold()),
                ])
                .style(base),
            );
        }
        push(
            &mut lines,
            &mut row,
            Line::styled(
                format!("  {}", entry.hint),
                base.fg(Color::Rgb(125, 132, 145)),
            ),
        );
        rows.push((index, first, row));
    }
    let selected = rows
        .iter()
        .find(|(index, _, _)| *index == screen.setting_selected);
    let mut start = screen.scroll;
    let focus = crate::settings::Focus {
        selected: screen.setting_selected,
        area: inner,
    };
    if screen.settings_focus != Some(focus)
        && let Some((_, first, end)) = selected
    {
        if *first < start {
            start = *first;
        } else if *end > start + usize::from(inner.height) {
            start = end.saturating_sub(usize::from(inner.height));
        }
    }
    start = start.min(row.saturating_sub(usize::from(inner.height)));
    screen.scroll = start;
    screen.settings_focus = Some(focus);
    frame.render_widget(panel, area);
    let content = Rect::new(inner.x, inner.y, width, inner.height);
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .scroll((start.min(u16::MAX as usize) as u16, 0)),
        content,
    );
    for (index, first, end) in rows {
        let first = first.max(start);
        let end = end.min(start + usize::from(inner.height));
        if first < end {
            screen.pointer.item(
                Rect::new(
                    content.x,
                    content.y + (first - start) as u16,
                    content.width,
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
        row.saturating_sub(usize::from(inner.height)),
        start,
        crate::pointer::ScrollTarget::Text,
    );
    crate::scrollbar::vertical(frame, area, inner, row, start);
}
