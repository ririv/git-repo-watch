//! Bottom actions share clipping, styling and pointer geometry.
use super::*;
use unicode_width::UnicodeWidthStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::view) enum Target {
    Button(crate::pointer::Action),
    KeyboardOnly,
    #[cfg(feature = "developer-scheduler")]
    Choice(crate::pointer::Target, usize),
}
#[derive(Debug, PartialEq, Eq)]
pub(in crate::view) struct Item {
    pub label: &'static str,
    pub state: State,
    pub target: Target,
}
pub(in crate::view) fn management(
    session: &crate::management::Session,
    selected: crate::management::Action,
) -> Row {
    let items = session
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
        .collect();
    Row::new(items, Spacing::Flush)
}
#[cfg(feature = "developer-scheduler")]
pub(in crate::view) fn workflow(session: &crate::workflow::Session) -> Row {
    let items = session
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
        .collect();
    Row::new(items, Spacing::Flush)
}
#[derive(Debug, PartialEq, Eq)]
pub(in crate::view) enum Spacing {
    Flush,
    Indented,
    Confirmation,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Row {
    items: Vec<Item>,
    spacing: Spacing,
}

impl Row {
    pub(in crate::view) fn new(items: Vec<Item>, spacing: Spacing) -> Self {
        Self { items, spacing }
    }

    pub(in crate::view) fn cells(&self, area: Rect) -> Vec<(Rect, &Item)> {
        let (inset, gap) = match self.spacing {
            Spacing::Flush => (0, 2),
            Spacing::Indented => (u16::from(area.width > 1), 2),
            Spacing::Confirmation => (u16::from(area.width > 1), 3),
        };
        let selected = self
            .items
            .iter()
            .position(|item| item.state.selected())
            .unwrap_or(0);
        let widths = self
            .items
            .iter()
            .map(|item| item.label.width().saturating_add(2))
            .collect::<Vec<_>>();
        let mut first = 0;
        while first < selected
            && widths[first..=selected].iter().sum::<usize>()
                + usize::from(gap) * (selected - first)
                > usize::from(area.width.saturating_sub(inset))
        {
            first += 1;
        }
        let mut x = area.x.saturating_add(inset);
        self.items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let height = area.height.min(1);
                if index < first {
                    return (Rect::new(area.x, area.y, 0, height), item);
                }
                let width = widths[index].min(usize::from(area.right().saturating_sub(x))) as u16;
                let cell = Rect::new(x, area.y, width, height);
                x = x
                    .saturating_add(width)
                    .saturating_add(gap)
                    .min(area.right());
                (cell, item)
            })
            .collect()
    }

    pub(in crate::view) fn render(
        self,
        frame: &mut Frame,
        area: Rect,
        hits: &mut crate::pointer::Map,
    ) -> Surface {
        let area = Rect::new(area.x, area.y, area.width, area.height.min(1));
        for (cell, item) in self.cells(area) {
            let style = match item.state {
                State::Available => Style::default().fg(Color::Gray).bg(Color::Reset),
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
                    #[cfg(feature = "developer-scheduler")]
                    Target::Choice(target, index) => hits.item(cell, target, index),
                    Target::KeyboardOnly => {}
                }
            }
        }
        Surface::Actions { area, row: self }
    }
}

pub(super) fn confirmation(
    choice: confirmation::Choice,
    execute: &'static str,
    back: &'static str,
) -> Row {
    let items = [
        (confirmation::Choice::Execute, execute),
        (confirmation::Choice::Return, back),
    ]
    .into_iter()
    .map(|(action, label)| Item {
        label,
        state: State::new(true, action == choice),
        target: Target::KeyboardOnly,
    })
    .collect();
    Row::new(items, Spacing::Confirmation)
}
