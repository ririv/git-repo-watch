//! Single-line command rows separate usage, concise descriptions and configuration values.
use super::*;
use ratatui::layout::Alignment;
use unicode_width::UnicodeWidthStr;

struct Entry {
    command: &'static command::Command,
    name: String,
    usage: String,
    description: String,
    value: String,
}
struct Columns {
    name: usize,
    description: usize,
    value: usize,
}
impl Columns {
    fn fit(entries: &[Entry], width: usize) -> Self {
        let name = entries
            .iter()
            .map(|entry| entry.usage.width())
            .max()
            .unwrap_or(0)
            .min(width);
        let minimum_description = entries
            .iter()
            .map(|entry| entry.description.width())
            .max()
            .unwrap_or(0)
            .min(10);
        let value = entries
            .iter()
            .map(|entry| entry.value.width())
            .max()
            .unwrap_or(0);
        let free = width.saturating_sub(name + 2);
        // 窄屏优先保留命令、参数和短说明；配置值在剩余空间中右对齐。
        let value = value.min(free.saturating_sub(minimum_description + 2));
        let description = free.saturating_sub(if value > 0 { value + 2 } else { 0 });
        Self {
            name,
            description,
            value,
        }
    }
}
impl Entry {
    fn item(&self, columns: &Columns) -> ListItem<'static> {
        let name = clipped(&self.name, columns.name);
        let mut used = name.width();
        let mut spans = vec![Span::styled(name, Style::default().fg(ACCENT).bold())];
        if let Some(argument) = self.command.action.argument() {
            let argument = clipped(
                &format!(" {}", argument.placeholder()),
                columns.name.saturating_sub(used),
            );
            used += argument.width();
            spans.push(Span::styled(
                argument,
                Style::default().fg(Color::LightCyan).italic(),
            ));
        }
        spans.push(Span::raw(" ".repeat(columns.name.saturating_sub(used))));
        field(
            &mut spans,
            &self.description,
            columns.description,
            Style::default(),
            Alignment::Left,
        );
        field(
            &mut spans,
            &self.value,
            columns.value,
            Style::default().fg(GOOD),
            Alignment::Right,
        );
        ListItem::new(Line::from(spans))
    }
}
fn clipped(text: &str, width: usize) -> String {
    if text.width() <= width {
        text.into()
    } else {
        truncate(text, width)
    }
}
fn field(
    spans: &mut Vec<Span<'static>>,
    text: &str,
    width: usize,
    style: Style,
    alignment: Alignment,
) {
    if width == 0 {
        return;
    }
    let value = clipped(text, width);
    let padding = width.saturating_sub(value.width());
    let leading = match alignment {
        Alignment::Left => 0,
        Alignment::Center => padding / 2,
        Alignment::Right => padding,
    };
    spans.push(Span::raw("  "));
    spans.push(Span::raw(" ".repeat(leading)));
    spans.push(Span::styled(value, style));
    spans.push(Span::raw(" ".repeat(padding - leading)));
}

pub(super) fn render(frame: &mut Frame, screen: &mut Screen, body: Rect) {
    let crate::bar::InputBar::Command {
        input,
        selected,
        path,
        menu,
        ..
    } = &screen.bar
    else {
        return;
    };
    if !matches!(
        screen.mode,
        Mode::Browsing
            | Mode::Browser(_)
            | Mode::Panel(_)
            | Mode::Stashes(_)
            | Mode::Branches(_)
            | Mode::Files(_)
            | Mode::Changes(_)
            | Mode::History(_)
            | Mode::Error { .. }
    ) {
        return;
    }
    match menu {
        command::completion::Menu::Directories(values) => {
            completion::render(frame, values, body);
            return;
        }
        command::completion::Menu::Values(values) => {
            completion::render_values(frame, values, body);
            return;
        }
        command::completion::Menu::Commands => {}
    }
    let entries = screen
        .bar
        .commands(screen.config.ai.enabled)
        .expect("command candidates")
        .into_iter()
        .map(|command| {
            let selection = screen.bar.selection().expect("command selection");
            let description = match command.action {
                Action::Copy(_) => selection
                    .annotation(command.action)
                    .expect("copy target label"),
                _ => command.summary().text(),
            };
            let value = repository_list::annotation(command.action, screen)
                .or_else(|| layout_picker::annotation(command.action, screen, body))
                .or_else(|| settings::annotation(command.action, screen))
                .or_else(|| match command.action {
                    Action::BrowserToggle | Action::BrowserSplit => {
                        selection.annotation(command.action).map(str::to_owned)
                    }
                    _ => None,
                });
            let prefix = if input.menu_context().is_some() {
                ""
            } else {
                "/"
            };
            Entry {
                command,
                name: format!("{prefix}{}", input.name(command)),
                usage: format!("{prefix}{}", input.usage(command)),
                description: safe(description),
                value: value.unwrap_or_default(),
            }
        })
        .collect::<Vec<_>>();
    let available = body.height.saturating_sub(u16::from(body.height >= 9));
    let height = (entries.len().clamp(1, 8) as u16 + 2).min(available);
    let area = Rect::new(
        body.x,
        body.bottom().saturating_sub(height),
        body.width,
        height,
    );
    let title = input
        .menu_context()
        .map(|context| {
            format!(
                "/{} · {}",
                context.name(),
                repository_list::menu_title(screen).unwrap_or(i18n::tr!(ContextCommands))
            )
        })
        .or_else(|| repository_list::menu_title(screen).map(str::to_owned))
        .unwrap_or_else(|| {
            i18n::tf!(
                Commands,
                path.as_deref()
                    .map(name)
                    .unwrap_or_else(|| i18n::tr!(NoRepositorySelected).into())
            )
        });
    let border = if area.height >= 3 {
        block(title)
    } else {
        Block::default()
    };
    let inner = border.inner(area);
    frame.render_widget(Clear, area);
    frame.render_widget(border, area);
    if inner.is_empty() {
        return;
    }
    let list = inner;
    let columns = Columns::fit(&entries, list.width.into());
    let items = entries
        .iter()
        .map(|entry| entry.item(&columns))
        .collect::<Vec<_>>();
    let mut state = ListState::default().with_selected(if items.is_empty() {
        None
    } else {
        Some((*selected).min(items.len() - 1))
    });
    frame.render_stateful_widget(
        List::new(if items.is_empty() {
            vec![ListItem::new(i18n::tr!(NoMatchingCommands))]
        } else {
            items
        })
        .highlight_style(selected_style()),
        list,
        &mut state,
    );
    crate::scrollbar::vertical(frame, area, list, entries.len(), state.offset());
    screen.command_items = (state.offset()..entries.len())
        .take(list.height.into())
        .enumerate()
        .map(|(row, index)| (Rect::new(list.x, list.y + row as u16, list.width, 1), index))
        .collect();
}
