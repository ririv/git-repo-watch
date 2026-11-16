//! Locale-aware input aliases normalize only command entry, never command arguments.
use crate::{Mode, Screen, bar::InputBar, command::Input};
use crossterm::event::{KeyCode, KeyEvent};
use std::borrow::Cow;

impl Screen {
    fn chinese_command_trigger_enabled(&self) -> bool {
        self.config.chinese_command_trigger && i18n::language() == i18n::Language::Chinese
    }
    pub(crate) fn at_command_entry(&self) -> bool {
        if self.focused_replay_control().is_some() || self.history_actions().is_some() {
            return true;
        }
        match &self.bar {
            InputBar::CommandsOnly => true,
            InputBar::Search(editor) => {
                matches!(self.mode, Mode::Error { .. }) || editor.cursor == 0
            }
            InputBar::Command { input, .. } => match input {
                Input::Prefix(_) => true,
                Input::Scoped { editor, .. } => editor.text.is_empty(),
                Input::Global(editor) => editor.text == "/",
            },
        }
    }
    pub(crate) fn command_trigger_key(&self, mut key: KeyEvent) -> KeyEvent {
        if self.chinese_command_trigger_enabled()
            && self.at_command_entry()
            && key.modifiers.is_empty()
            && key.code == KeyCode::Char('、')
        {
            key.code = KeyCode::Char('/');
        }
        key
    }
    pub(crate) fn command_trigger_text<'a>(&self, text: &'a str) -> Cow<'a, str> {
        // 粘贴沿用 / 的入口语义；只规范开头最多两个触发符，正文不做全局替换。
        if !self.chinese_command_trigger_enabled() {
            return Cow::Borrowed(text);
        }
        let prefix = text
            .chars()
            .take(2)
            .take_while(|c| matches!(c, '/' | '、'))
            .collect::<String>();
        if !(self.at_command_entry() || self.bar.is_plain() && prefix.chars().count() == 2) {
            return Cow::Borrowed(text);
        }
        if !prefix.contains('、') {
            return Cow::Borrowed(text);
        }
        Cow::Owned(format!(
            "{}{}",
            "/".repeat(prefix.chars().count()),
            &text[prefix.len()..]
        ))
    }
}
