//! Checkbox workflow drafts keep execution order separate from keyboard focus.
use crate::{Effect, Mode, Outcome, Screen};
use anyhow::Result;
use crossterm::event::KeyCode;
use engine::{
    service::ConfigEdit,
    workflow::{Plan, Step},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Operation {
    Fetch,
    Pull,
    Push,
}
impl Operation {
    const ALL: [Self; 3] = [Self::Fetch, Self::Pull, Self::Push];
    pub fn step(self) -> Step {
        match self {
            Self::Fetch => Step::Fetch,
            Self::Pull => Step::Pull,
            Self::Push => Step::Push,
        }
    }
    fn from_step(step: Step) -> Option<Self> {
        match step {
            Step::Fetch => Some(Self::Fetch),
            Step::Pull => Some(Self::Pull),
            Step::Push => Some(Self::Push),
            Step::Inspect => None,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Inclusion {
    Enabled,
    Disabled,
}
#[derive(Debug, Clone, Copy)]
pub(crate) struct Entry {
    pub operation: Operation,
    pub inclusion: Inclusion,
}
impl Entry {
    pub fn enabled(self) -> bool {
        self.inclusion == Inclusion::Enabled
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Row {
    Optional(usize),
    Inspect,
}
impl Row {
    pub fn index(self) -> usize {
        match self {
            Self::Optional(index) => index,
            Self::Inspect => 3,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RowAction {
    Toggle,
    Up,
    Down,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FooterAction {
    Save,
    Cancel,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Cursor {
    Step { index: usize, action: RowAction },
    Inspect,
    Footer { row: Row, action: FooterAction },
}
impl Cursor {
    pub fn row(self) -> Row {
        match self {
            Self::Step { index, .. } => Row::Optional(index),
            Self::Inspect => Row::Inspect,
            Self::Footer { row, .. } => row,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Action {
    Toggle,
    Up,
    Down,
    Save,
    Cancel,
}
impl Action {
    pub const ALL: [Self; 5] = [Self::Toggle, Self::Up, Self::Down, Self::Save, Self::Cancel];
    pub fn label(self) -> &'static str {
        match self {
            Self::Toggle => i18n::tr!(WorkflowToggle),
            Self::Up => i18n::tr!(WorkflowMoveUp),
            Self::Down => i18n::tr!(WorkflowMoveDown),
            Self::Save => i18n::tr!(WorkflowSave),
            Self::Cancel => i18n::tr!(WorkflowCancel),
        }
    }
}
#[derive(Debug)]
pub(crate) struct Session {
    pub entries: [Entry; 3],
    pub cursor: Cursor,
    pub offset: usize,
    pub scroll: usize,
    pub revealed: Option<(Row, ratatui::layout::Rect)>,
}
pub(crate) enum Input {
    Pending,
    Save(Plan),
    Cancel,
}
impl Session {
    pub fn new(plan: &Plan) -> Self {
        let enabled = plan
            .steps()
            .iter()
            .copied()
            .filter_map(Operation::from_step)
            .collect::<Vec<_>>();
        let mut entries = Operation::ALL.map(|operation| Entry {
            operation,
            inclusion: Inclusion::Disabled,
        });
        // 仅重排已启用项所在的槽位，保持已有执行顺序，同时展示其余可选步骤。
        let slots = Operation::ALL
            .iter()
            .enumerate()
            .filter_map(|(index, operation)| enabled.contains(operation).then_some(index))
            .collect::<Vec<_>>();
        for (index, operation) in slots.into_iter().zip(enabled) {
            entries[index] = Entry {
                operation,
                inclusion: Inclusion::Enabled,
            };
        }
        Self {
            entries,
            cursor: Cursor::Step {
                index: 0,
                action: RowAction::Toggle,
            },
            offset: 0,
            scroll: 0,
            revealed: None,
        }
    }
    pub fn title(&self) -> &'static str {
        i18n::tr!(WorkflowScheduledActions)
    }
    pub fn steps(&self) -> Vec<Step> {
        self.entries
            .iter()
            .filter(|entry| entry.enabled())
            .map(|entry| entry.operation.step())
            .chain(std::iter::once(Step::Inspect))
            .collect()
    }
    pub fn plan(&self) -> Result<Plan> {
        self.steps().try_into()
    }
    pub fn selected_step(&self) -> Step {
        match self.cursor.row() {
            Row::Optional(index) => self.entries[index].operation.step(),
            Row::Inspect => Step::Inspect,
        }
    }
    fn set_cursor(&mut self, cursor: Cursor) {
        if self.cursor.row() != cursor.row() {
            self.scroll = 0;
            self.revealed = None;
        }
        self.cursor = cursor;
    }
    pub fn select(&mut self, index: usize) -> bool {
        match index {
            0..=2 => self.set_cursor(Cursor::Step {
                index,
                action: RowAction::Toggle,
            }),
            3 => self.set_cursor(Cursor::Inspect),
            _ => return false,
        }
        true
    }
    pub fn page(&self) -> crate::input_pages::Pair {
        match self.cursor {
            Cursor::Step { .. } | Cursor::Inspect => crate::input_pages::Pair::Content,
            Cursor::Footer { .. } => crate::input_pages::Pair::Actions,
        }
    }
    pub fn page_actions(&self) -> &'static [Action] {
        match self.page() {
            crate::input_pages::Pair::Content => &[Action::Toggle, Action::Up, Action::Down],
            crate::input_pages::Pair::Actions => &[Action::Save, Action::Cancel],
        }
    }
    fn cycle_page(&mut self) {
        let row = self.cursor.row();
        match self.page().next() {
            crate::input_pages::Pair::Content => {
                self.select(row.index());
            }
            crate::input_pages::Pair::Actions => self.set_cursor(Cursor::Footer {
                row,
                action: FooterAction::Save,
            }),
        }
    }
    fn move_row(&mut self, delta: isize) {
        let index = self
            .cursor
            .row()
            .index()
            .saturating_add_signed(delta)
            .min(3);
        match self.cursor {
            Cursor::Footer { action, .. } => self.set_cursor(Cursor::Footer {
                row: if index == 3 {
                    Row::Inspect
                } else {
                    Row::Optional(index)
                },
                action,
            }),
            _ => {
                self.select(index);
            }
        }
    }
    pub fn enabled(&self, action: Action) -> bool {
        match (action, self.cursor.row()) {
            (Action::Toggle, Row::Optional(_)) => true,
            (Action::Up, Row::Optional(index)) => index > 0,
            (Action::Down, Row::Optional(index)) => index < 2,
            (Action::Save | Action::Cancel, _) => true,
            _ => false,
        }
    }
    pub fn action_label(&self, action: Action) -> &'static str {
        match (action, self.cursor.row()) {
            (Action::Toggle, Row::Optional(index)) => {
                if self.entries[index].enabled() {
                    i18n::tr!(WorkflowDisableStep)
                } else {
                    i18n::tr!(WorkflowEnableStep)
                }
            }
            _ => action.label(),
        }
    }
    pub fn action_selected(&self, action: Action) -> bool {
        matches!(
            (self.cursor, action),
            (
                Cursor::Step {
                    action: RowAction::Toggle,
                    ..
                },
                Action::Toggle
            ) | (
                Cursor::Step {
                    action: RowAction::Up,
                    ..
                },
                Action::Up
            ) | (
                Cursor::Step {
                    action: RowAction::Down,
                    ..
                },
                Action::Down
            ) | (
                Cursor::Footer {
                    action: FooterAction::Save,
                    ..
                },
                Action::Save
            ) | (
                Cursor::Footer {
                    action: FooterAction::Cancel,
                    ..
                },
                Action::Cancel
            )
        )
    }
    pub fn key(&mut self, key: KeyCode) -> Result<Input> {
        match key {
            KeyCode::Esc if self.page() == crate::input_pages::Pair::Actions => self.cycle_page(),
            KeyCode::Esc => return Ok(Input::Cancel),
            KeyCode::Up => self.move_row(-1),
            KeyCode::Down => self.move_row(1),
            KeyCode::BackTab => self.cycle_page(),
            KeyCode::Home => {
                self.select(0);
            }
            KeyCode::End => {
                self.move_row(3);
            }
            KeyCode::Left | KeyCode::Right => match self.cursor {
                Cursor::Step { index, action } => {
                    let actions = [RowAction::Toggle, RowAction::Up, RowAction::Down]
                        .into_iter()
                        .filter(|action| match action {
                            RowAction::Toggle => true,
                            RowAction::Up => index > 0,
                            RowAction::Down => index < 2,
                        })
                        .collect::<Vec<_>>();
                    let current = actions
                        .iter()
                        .position(|item| *item == action)
                        .expect("enabled row action");
                    let next = (current
                        + if key == KeyCode::Right {
                            1
                        } else {
                            actions.len() - 1
                        })
                        % actions.len();
                    self.cursor = Cursor::Step {
                        index,
                        action: actions[next],
                    };
                }
                Cursor::Footer { row, action } => {
                    self.cursor = Cursor::Footer {
                        row,
                        action: match action {
                            FooterAction::Save => FooterAction::Cancel,
                            FooterAction::Cancel => FooterAction::Save,
                        },
                    }
                }
                Cursor::Inspect => {}
            },
            KeyCode::Char(' ') => {
                if matches!(self.cursor, Cursor::Step { .. }) {
                    return self.apply(Action::Toggle);
                }
            }
            KeyCode::Enter => {
                return match self.cursor {
                    Cursor::Step { action, .. } => self.apply(match action {
                        RowAction::Toggle => Action::Toggle,
                        RowAction::Up => Action::Up,
                        RowAction::Down => Action::Down,
                    }),
                    Cursor::Inspect => Ok(Input::Pending),
                    Cursor::Footer {
                        action: FooterAction::Save,
                        ..
                    } => self.apply(Action::Save),
                    Cursor::Footer {
                        action: FooterAction::Cancel,
                        ..
                    } => Ok(Input::Cancel),
                };
            }
            _ => {}
        }
        Ok(Input::Pending)
    }
    pub fn apply(&mut self, action: Action) -> Result<Input> {
        match action {
            Action::Save => {
                self.cursor = Cursor::Footer {
                    row: self.cursor.row(),
                    action: FooterAction::Save,
                };
                return Ok(Input::Save(self.plan()?));
            }
            Action::Cancel => return Ok(Input::Cancel),
            _ if !self.enabled(action) => return Ok(Input::Pending),
            _ => {}
        }
        let Row::Optional(mut index) = self.cursor.row() else {
            return Ok(Input::Pending);
        };
        match action {
            Action::Toggle => {
                self.entries[index].inclusion = match self.entries[index].inclusion {
                    Inclusion::Enabled => Inclusion::Disabled,
                    Inclusion::Disabled => Inclusion::Enabled,
                }
            }
            Action::Up => {
                self.entries.swap(index, index - 1);
                index -= 1;
            }
            Action::Down => {
                self.entries.swap(index, index + 1);
                index += 1;
            }
            Action::Save | Action::Cancel => unreachable!("handled footer action"),
        }
        self.cursor = Cursor::Step {
            index,
            action: RowAction::Toggle,
        };
        self.revealed = None;
        Ok(Input::Pending)
    }
}
impl Screen {
    pub(crate) fn open_workflow(&mut self) {
        self.open_page(crate::Page::Configuration(crate::Configuration::Schedule));
        let crate::Page::Configuration(section) = self.page else {
            unreachable!()
        };
        self.setting_selected = section
            .items(&self.snapshot, &self.config.ai)
            .iter()
            .position(|item| *item == crate::settings::Item::Command(crate::Action::Workflow))
            .expect("workflow setting");
        let session = Session::new(&self.config.scheduled_actions);
        let origin = self.take_mode();
        self.remember_view(origin);
        self.mode = Mode::Workflow(Box::new(session));
        self.sync_bar();
    }
    pub(crate) fn workflow_input(&mut self, session: Box<Session>, result: Result<Input>) {
        match result {
            Ok(Input::Cancel) => {
                self.mode = Mode::Workflow(session);
                self.return_to_view();
            }
            Ok(Input::Save(plan)) => {
                self.mode = Mode::Workflow(session);
                self.return_to_view();
                self.start(
                    i18n::tr!(WorkflowScheduled),
                    Effect::Configuration,
                    move |service, cancel, _| {
                        service
                            .configure_with_cancel(ConfigEdit::ScheduledWorkflow(plan), &cancel)?;
                        Ok(Outcome::Text(i18n::tr!(WorkflowSaved).into()))
                    },
                );
            }
            result => {
                self.mode = Mode::Workflow(session);
                self.message = crate::message::Message::error(match result {
                    Err(error) => format!("{error:#}"),
                    _ => String::new(),
                });
            }
        }
    }
}
