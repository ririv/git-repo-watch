//! Single-line command rows show names, concise descriptions and current values.
mod elision;
use super::*;
use crate::{command::named::Candidate, features::Maturity};
use ratatui::layout::Alignment;
use unicode_width::UnicodeWidthStr;

struct Entry {
    name: command::hierarchy::Label,
    argument: Option<command::ArgumentAction>,
    named_parameter: bool,
    maturity: Maturity,
    description: String,
    value: String,
    value_kind: ValueKind,
    availability: command::availability::Availability,
}
enum ValueKind {
    Text,
    Path,
    Repository,
}
struct Columns {
    name: usize,
    description: usize,
    value: usize,
}
impl Columns {
    fn fit(entries: &[Entry], width: usize) -> Self {
        let desired_name = entries
            .iter()
            .map(|entry| {
                entry.name.branch.width()
                    + entry.name.name.width()
                    + entry.argument.map_or(0, |_| 2)
            })
            .max()
            .unwrap_or(0);
        let minimum_description = entries
            .iter()
            .map(|entry| {
                entry
                    .description
                    .width()
                    .min(if entry.availability.reason().is_some() {
                        40
                    } else {
                        16
                    })
            })
            .max()
            .unwrap_or(0);
        let value = entries
            .iter()
            .map(|entry| entry.value.width())
            .max()
            .unwrap_or(0);
        // 命令名带紧凑参数标记；完整参数语法由输入栏按需呈现。
        let name = desired_name.min(width);
        let free = width.saturating_sub(name + 2);
        // 窄屏优先保留命令名和短说明；配置值在剩余空间中右对齐。
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
        let marker = self.argument.map_or("", |_| " …");
        let marker_width = marker.width().min(columns.name);
        let name_width = columns.name.saturating_sub(marker_width);
        let branch = clipped(&self.name.branch, name_width);
        let name = clipped(&self.name.name, name_width.saturating_sub(branch.width()));
        let used = branch.width() + name.width() + marker_width;
        let enabled = self.availability.is_ready();
        let mut spans = vec![
            Span::styled(branch, Style::default().fg(MUTED)),
            Span::styled(
                name,
                if enabled {
                    Style::default()
                        .fg(if self.named_parameter {
                            WARN
                        } else if self.maturity == Maturity::Experimental {
                            EXPERIMENTAL
                        } else {
                            ACCENT
                        })
                        .bold()
                } else {
                    Style::default().fg(MUTED)
                },
            ),
        ];
        spans.push(Span::styled(
            clipped(marker, marker_width),
            Style::default().fg(if enabled { PARAMETER } else { MUTED }),
        ));
        spans.push(Span::raw(" ".repeat(columns.name.saturating_sub(used))));
        field(
            &mut spans,
            &self.description,
            &ValueKind::Text,
            columns.description,
            if enabled {
                if self.maturity == Maturity::Experimental {
                    Style::default().fg(EXPERIMENTAL)
                } else {
                    Style::default()
                }
            } else {
                Style::default().fg(MUTED)
            },
            Alignment::Left,
        );
        field(
            &mut spans,
            &self.value,
            &self.value_kind,
            columns.value,
            Style::default().fg(match self.value_kind {
                ValueKind::Repository => ACCENT,
                ValueKind::Text | ValueKind::Path => GOOD,
            }),
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
    kind: &ValueKind,
    width: usize,
    style: Style,
    alignment: Alignment,
) {
    if width == 0 {
        return;
    }
    let value = match kind {
        ValueKind::Text => clipped(text, width),
        ValueKind::Path | ValueKind::Repository => elision::middle(text, width),
    };
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
    let mut viewport = std::mem::take(&mut screen.command_viewport);
    render_menu(frame, screen, body, &mut viewport);
    screen.command_viewport = viewport;
}

fn render_menu(
    frame: &mut Frame,
    screen: &mut Screen,
    body: Rect,
    viewport: &mut candidates::Viewport,
) {
    let crate::bar::InputBar::Command {
        input,
        selected,
        menu,
        path,
        repository,
        selection,
    } = &screen.bar
    else {
        return;
    };
    if !matches!(
        screen.mode,
        Mode::Management(_)
            | Mode::Browsing
            | Mode::Composition(_)
            | Mode::CompositionWaiting(_)
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
            screen.command_items = completion::render(frame, screen, values, body, viewport);
            return;
        }
        command::completion::Menu::Options(values) => {
            screen.command_items =
                completion::render_options(frame, screen, values, body, viewport);
            return;
        }
        command::completion::Menu::History(history) => {
            screen.command_items =
                completion::render_history(frame, screen, history, body, viewport);
            return;
        }
        command::completion::Menu::Commands => {}
        command::completion::Menu::Values(values) => {
            screen.command_items = completion::render_values(frame, screen, values, body, viewport);
            return;
        }
        command::completion::Menu::Loading(_) => {}
        command::completion::Menu::Failed(error) => {
            let areas = candidates::panel(frame, screen, body, 1, None);
            candidates::render_list(
                frame,
                areas,
                vec![ListItem::new(safe(error))],
                None,
                candidates::Source::Commands(Vec::new()),
                viewport,
            );
            return;
        }
    }
    let menu = screen.command_menu().expect("command candidates");
    let source = candidates::Source::Commands(
        menu.entries
            .iter()
            .map(|candidate| match candidate {
                Candidate::Command(command) => (command.name, None),
                Candidate::Parameter(parameter) => (parameter.command().name, Some(parameter.name)),
            })
            .collect(),
    );
    let title = if menu.group.is_none()
        && !menu.entries.is_empty()
        && menu
            .entries
            .iter()
            .all(|candidate| matches!(candidate, Candidate::Parameter(_)))
    {
        Some(i18n::tr!(CommandGroupParameters).into())
    } else {
        menu.group
            .as_ref()
            .and_then(command::hierarchy::Group::title)
    };
    let tree = command::hierarchy::Tree::new(menu.group.as_ref(), &menu.entries);
    let entries = menu
        .entries
        .iter()
        .copied()
        .enumerate()
        .map(|(index, candidate)| {
            let command = candidate.owner();
            let resolved = screen.resolve_command(command.action, path.as_deref(), selection);
            let target = resolved
                .value()
                .and_then(|target| target.label(command.action, selection));
            let partial_index = resolved
                .value()
                .is_some_and(|target| target.is_partial_index());
            let description = match candidate {
                Candidate::Parameter(parameter) => parameter.description.text(),
                Candidate::Command(_) => match command.action {
                    Action::Stage if partial_index => i18n::tr!(PartialStage),
                    Action::Unstage if partial_index => i18n::tr!(PartialUnstage),
                    Action::Copy => selection
                        .annotation(command.action)
                        .expect("copy target label"),
                    _ => command.summary().text(),
                },
            };
            let repository_path = resolved.value().and_then(|target| target.repository_path());
            // 仓库目标由 /repo 行展示；子命令只展示各自的选区或配置值。
            let (value, value_kind) = match command.action {
                Action::OpenRepository => (
                    repository_path.and_then(|path| repository.label(path)),
                    ValueKind::Repository,
                ),
                Action::RepositoryEnvEdit(field) => (
                    repository_path
                        .map(|path| field.value(screen.config.repository_environment.get(path))),
                    ValueKind::Text,
                ),
                Action::OpenEditor | Action::Terminal => (target, ValueKind::Path),
                action => (
                    target
                        .or_else(|| repository_list::annotation(action, screen))
                        .or_else(|| layout_summary::annotation(action, screen, body))
                        .or_else(|| settings::annotation(action, screen)),
                    ValueKind::Text,
                ),
            };
            let argument = match candidate {
                Candidate::Command(command) => command.action.argument(),
                Candidate::Parameter(parameter) => {
                    Some(command::ArgumentAction::Option(parameter.kind))
                }
            };
            let availability = resolved.map(|_| ());
            Entry {
                name: tree.label(input, index),
                argument,
                named_parameter: matches!(candidate, Candidate::Parameter(_)),
                maturity: if command.action.experimental() {
                    Maturity::Experimental
                } else {
                    Maturity::Stable
                },
                description: safe(match availability.label() {
                    None => description,
                    Some(reason) => reason,
                }),
                value: if availability.is_ready() {
                    value.unwrap_or_default()
                } else {
                    String::new()
                },
                value_kind,
                availability,
            }
        })
        .collect::<Vec<_>>();
    let (area, list) = candidates::panel(frame, screen, body, entries.len(), title.as_deref());
    if list.is_empty() {
        return;
    }
    let columns = Columns::fit(&entries, list.width.into());
    let items = entries
        .iter()
        .map(|entry| entry.item(&columns))
        .collect::<Vec<_>>();
    let selected = if items.is_empty() {
        None
    } else {
        Some((*selected).min(items.len() - 1))
    };
    screen.command_items = candidates::render_list(
        frame,
        (area, list),
        if items.is_empty() {
            vec![ListItem::new(i18n::tr!(NoMatchingCommands))]
        } else {
            items
        },
        selected,
        source,
        viewport,
    );
}
