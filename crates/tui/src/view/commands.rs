//! Single-line command rows show names, concise descriptions and current values.
mod elision;
use super::*;
use crate::features::Maturity;
use ratatui::layout::Alignment;
use unicode_width::UnicodeWidthStr;

struct Entry {
    name: command::hierarchy::Label,
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
            .map(|entry| entry.name.branch.width() + entry.name.name.width())
            .max()
            .unwrap_or(0);
        let minimum_description = entries
            .iter()
            .map(|entry| {
                entry.description.width().min(
                    if matches!(
                        &entry.availability,
                        command::availability::Availability::Unavailable(_)
                    ) {
                        40
                    } else {
                        16
                    },
                )
            })
            .max()
            .unwrap_or(0);
        let value = entries
            .iter()
            .map(|entry| entry.value.width())
            .max()
            .unwrap_or(0);
        // 命令名保留完整宽度；参数语法由输入栏按需呈现。
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
        let branch = clipped(&self.name.branch, columns.name);
        let name = clipped(&self.name.name, columns.name.saturating_sub(branch.width()));
        let used = branch.width() + name.width();
        let enabled = matches!(
            self.availability,
            command::availability::Availability::Ready
        );
        let mut spans = vec![
            Span::styled(branch, Style::default().fg(MUTED)),
            Span::styled(
                name,
                if enabled {
                    Style::default()
                        .fg(if self.maturity == Maturity::Experimental {
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
            Style::default().fg(if matches!(self.value_kind, ValueKind::Repository) {
                ACCENT
            } else {
                GOOD
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
        ..
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
    use command::target::IndexSelection;
    let partial_index = matches!(
        IndexSelection::current(screen),
        Some(IndexSelection::Diff(_))
    );
    let commands = screen.search_commands().expect("command candidates");
    let source =
        candidates::Source::Commands(commands.iter().map(|command| command.name).collect());
    let group = command::hierarchy::Group::current(input, &commands);
    let title = group.as_ref().and_then(command::hierarchy::Group::title);
    let tree = command::hierarchy::Tree::new(group.as_ref(), &commands);
    let entries = commands
        .iter()
        .copied()
        .map(|command| {
            let selection = screen.bar.selection().expect("command selection");
            let availability =
                screen.command_availability(command.action, path.as_deref(), selection);
            let description = match command.action {
                Action::Stage if partial_index => i18n::tr!(PartialStage),
                Action::Unstage if partial_index => i18n::tr!(PartialUnstage),
                Action::Copy => selection
                    .annotation(command.action)
                    .expect("copy target label"),
                _ => command.summary().text(),
            };
            let repository_action = matches!(
                command.action,
                Action::Push
                    | Action::Pull
                    | Action::Sync
                    | Action::Commit
                    | Action::CommitGenerate
                    | Action::BranchCreate
                    | Action::BranchSwitch
                    | Action::StashSave
                    | Action::StashSaveMode(_)
                    | Action::Replay(_)
                    | Action::ReplayControl(_)
                    | Action::Compose(crate::composition::Action::Open)
            );
            let target = screen.command_target(command.action, path.as_deref(), selection);
            let value = if repository_action {
                path.as_deref()
                    .and_then(|path| repository.label(path))
                    .map(|repository| match &target {
                        Some(target) => format!("{repository} · {target}"),
                        None => repository,
                    })
            } else {
                None
            }
            .or(target)
            .or_else(|| match (&screen.mode, command.action) {
                (Mode::Branches(view), Action::BranchFilterMenu) => {
                    Some(view.session().filter.title().into())
                }
                (Mode::Changes(session), Action::ConflictMenu) => match &session.preview {
                    crate::changes::Preview::Ready { content, .. } => match content.as_ref() {
                        crate::document::View::Conflict(view) => Some(view.mode.label().into()),
                        _ => None,
                    },
                    _ => None,
                },
                _ => None,
            })
            .or_else(|| repository_list::annotation(command.action, screen))
            .or_else(|| layout_summary::annotation(command.action, screen, body))
            .or_else(|| settings::annotation(command.action, screen))
            .or_else(|| match command.action {
                Action::PartialSelectMenu => {
                    screen.partial_mode().map(|mode| mode.label().to_owned())
                }
                Action::BrowserLayoutMenu | Action::BrowserModeMenu => {
                    selection.annotation(command.action).map(str::to_owned)
                }
                _ => None,
            });
            Entry {
                name: tree.label(input, command),
                maturity: if command.action.experimental() {
                    Maturity::Experimental
                } else {
                    Maturity::Stable
                },
                description: safe(match availability.label() {
                    None => description,
                    Some(reason) => reason,
                }),
                value: if matches!(availability, command::availability::Availability::Ready) {
                    value.unwrap_or_default()
                } else {
                    String::new()
                },
                value_kind: if repository_action {
                    ValueKind::Repository
                } else if matches!(command.action, Action::OpenEditor | Action::Terminal) {
                    ValueKind::Path
                } else {
                    ValueKind::Text
                },
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
