//! Clear an existing input draft before arming exit confirmation.
use crate::{Mode, Screen, TaskState};
use crossterm::event::{KeyEvent, KeyEventKind};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Sequence {
    #[default]
    Idle,
    ExitPending,
}

impl Sequence {
    pub fn reset(&mut self) -> bool {
        let changed = *self != Self::Idle;
        *self = Self::Idle;
        changed
    }

    pub fn notice(self) -> Option<&'static str> {
        match self {
            Self::ExitPending => Some(i18n::tr!(PressCtrlCAgainToQuit)),
            Self::Idle => None,
        }
    }
}

impl Screen {
    pub(super) fn interrupt_key(&mut self, key: KeyEvent) -> bool {
        // 支持按键事件类型的终端中，长按产生的重复事件不推进退出步骤。
        if key.kind != KeyEventKind::Press {
            return false;
        }
        if self.tasks.session.open() && !self.active_bar().text().is_empty() {
            self.clear_input_bar();
            self.exit_sequence.reset();
            return false;
        }
        if let Mode::Busy(pending) = &mut self.mode {
            pending.cancel.cancel();
            pending.state = TaskState::Cancelling;
            self.exit_sequence.reset();
            return false;
        }
        if self.exit_sequence == Sequence::ExitPending {
            return true;
        }
        // 只判断当前页面的输入；任务页覆盖的草稿不应增加一次清空步骤。
        if !self.tasks.session.open() && !self.bar.text().is_empty() {
            self.clear_input_bar();
        } else {
            self.exit_sequence = Sequence::ExitPending;
        }
        false
    }
}
