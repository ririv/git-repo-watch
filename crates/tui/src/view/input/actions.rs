//! Bottom actions share clipping, styling and pointer geometry.
use super::*;
use crate::pointer::Hit;
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, Copy)]
pub(in crate::view) enum State {
    Available,
    Selected,
    Disabled,
    SelectedDisabled,
}
impl State {
    pub fn new(enabled: bool, selected: bool) -> Self {
        match (enabled, selected) {
            (true, false) => Self::Available,
            (true, true) => Self::Selected,
            (false, false) => Self::Disabled,
            (false, true) => Self::SelectedDisabled,
        }
    }
    fn selected(self) -> bool {
        matches!(self, Self::Selected | Self::SelectedDisabled)
    }
}
#[derive(Debug, Clone, Copy)]
pub(in crate::view) enum Target {
    Button(crate::pointer::Action),
    Choice(crate::pointer::Target, usize),
}
impl Target {
    pub fn hit(self) -> Hit {
        match self {
            Self::Button(action) => Hit::Button(action),
            Self::Choice(target, index) => Hit::Item(target, index),
        }
    }
}
pub(in crate::view) struct Item {
    pub label: &'static str,
    pub state: State,
    pub target: Target,
}
pub(in crate::view) fn management(
    session: &crate::management::Session,
    selected: crate::management::Action,
) -> Vec<Item> {
    session
        .actions()
        .iter()
        .copied()
        .map(|action| Item {
            label: action.label(),
            state: State::new(
                action != crate::management::Action::Apply || !session.selection().is_empty(),
                action == selected,
            ),
            target: Target::Button(crate::pointer::Action::Management(action)),
        })
        .collect()
}
pub(in crate::view) fn workflow(session: &crate::workflow::Session) -> Vec<Item> {
    session
        .page_actions()
        .iter()
        .copied()
        .map(|action| Item {
            label: session.action_label(action),
            state: State::new(session.enabled(action), session.action_selected(action)),
            target: Target::Choice(
                crate::pointer::Target::WorkflowActions,
                crate::workflow::Action::ALL
                    .iter()
                    .position(|value| *value == action)
                    .expect("declared workflow action"),
            ),
        })
        .collect()
}
pub(in crate::view) fn render(
    frame: &mut Frame,
    area: Rect,
    items: &[Item],
    hits: &mut crate::pointer::Map,
) {
    let selected = items
        .iter()
        .position(|item| item.state.selected())
        .unwrap_or(0);
    let widths = items
        .iter()
        .map(|item| item.label.width().saturating_add(2))
        .collect::<Vec<_>>();
    let mut first = 0;
    while first < selected
        && widths[first..=selected].iter().sum::<usize>() + 2 * (selected - first)
            > usize::from(area.width)
    {
        first += 1;
    }
    let mut x = area.x;
    for (index, item) in items.iter().enumerate().skip(first) {
        let width = widths[index].min(usize::from(area.right().saturating_sub(x))) as u16;
        let cell = Rect::new(x, area.y, width, area.height.min(1));
        let style = match item.state {
            State::Available => Style::default().fg(Color::Gray),
            State::Selected => Style::default().fg(Color::Black).bg(INPUT_ACCENT).bold(),
            State::Disabled => Style::default().fg(MUTED),
            State::SelectedDisabled => Style::default().fg(MUTED).bg(INPUT_ACCENT).bold(),
        };
        frame.render_widget(
            Paragraph::new(format!(" {} ", item.label)).style(style),
            cell,
        );
        if matches!(item.state, State::Available | State::Selected) {
            match item.target {
                Target::Button(action) => hits.button(cell, action),
                Target::Choice(target, index) => hits.item(cell, target, index),
            }
        }
        x = x.saturating_add(width).saturating_add(2).min(area.right());
    }
}
