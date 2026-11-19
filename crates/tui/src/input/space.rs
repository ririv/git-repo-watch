//! Space ownership follows the active input surface before view dispatch.
use crate::{Mode, Screen, bar::InputBar};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Space {
    Insert,
    Content,
    Ignore,
}

impl Screen {
    // 先按当前输入面决定空格归属，避免空输入被空格意外切成搜索。
    // 已有搜索、命令和正文编辑接收空格；空搜索交给勾选／展开／选区操作，
    // 没有对应操作就忽略。底部操作页保持原页，统一用 Enter 执行。
    pub(crate) fn space_behavior(&self) -> Space {
        if self.tasks.session.open() {
            return if self.active_bar().command_input().is_some()
                || !self.active_bar().editor().text.is_empty()
            {
                Space::Insert
            } else {
                Space::Ignore
            };
        }
        if self.focus_mode == crate::focus::State::Selecting || self.history_actions().is_some() {
            return Space::Ignore;
        }
        // 独立编辑框按实际编辑焦点接收空格，操作页不能被空格触发或切走。
        match &self.mode {
            Mode::CommitMessage { editor, .. } => {
                return if editor.focused() {
                    Space::Insert
                } else {
                    Space::Ignore
                };
            }
            Mode::StashSave(session) => {
                return if session.focus.is_input() {
                    Space::Insert
                } else {
                    Space::Ignore
                };
            }
            Mode::PromptEditor(_) | Mode::PushTarget(_) => return Space::Insert,
            Mode::Workflow(session) => {
                return match session.cursor {
                    crate::workflow::Cursor::Step { .. } => Space::Content,
                    crate::workflow::Cursor::Inspect | crate::workflow::Cursor::Footer { .. } => {
                        Space::Ignore
                    }
                };
            }
            Mode::Composition(session)
                if matches!(session.state, crate::composition::State::Editing(_)) =>
            {
                return Space::Insert;
            }
            Mode::Management(session) if !session.focus.is_input() => return Space::Ignore,
            _ => {}
        }
        if self.focused_conflict_control().is_some() || self.focused_replay_control().is_some() {
            return Space::Ignore;
        }
        // 命令前缀也是输入；空格不得穿透到正文的勾选或差异操作。
        match &self.bar {
            InputBar::Command { .. } => return Space::Insert,
            InputBar::Search(editor) if !editor.text.is_empty() => return Space::Insert,
            InputBar::Search(_) | InputBar::CommandsOnly => {}
        }
        match &self.mode {
            Mode::Management(session)
                if matches!(session.source, crate::management::Source::Scan { .. }) =>
            {
                Space::Content
            }
            Mode::Files(_) | Mode::Changes(_) => Space::Content,
            Mode::Browser(session) if session.focus == crate::browser::Focus::Tree => {
                Space::Content
            }
            Mode::History(view)
                if matches!(view.as_ref(), crate::history::View::Graph(session)
                    if session.focus == crate::history::GraphFocus::Graph && session.selection.active()) =>
            {
                Space::Content
            }
            _ => Space::Ignore,
        }
    }
}
