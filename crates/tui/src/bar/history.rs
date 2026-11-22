//! Command submissions and history navigation share the normal command parser.
use super::*;
use command::{completion::Menu, history::Session};

impl Screen {
    pub(super) fn command_history_key(&mut self, key: KeyEvent) -> bool {
        let InputBar::Command {
            input,
            selected,
            menu,
            ..
        } = self.active_bar()
        else {
            return false;
        };
        let open = key.code == KeyCode::Up
            && key.modifiers.is_empty()
            && *selected == 0
            && matches!(menu, Menu::Commands)
            && matches!(input, command::Input::Global(editor) if editor.text == "/");
        if open {
            match self.service.store.command_history() {
                Ok(mut entries) => {
                    entries.retain(|text| {
                        let query = text.strip_prefix('/').expect("stored command slash");
                        let action = command::argument_input(query)
                            .map(|(argument, _)| argument.action())
                            .or_else(|| {
                                command::COMMANDS
                                    .iter()
                                    .find(|entry| entry.name == query)
                                    .map(|entry| entry.action)
                            });
                        action.is_some_and(|action| action.available(&self.config))
                    });
                    if let InputBar::Command { menu, .. } = self.active_bar_mut() {
                        *menu = Menu::History(Session::new(entries));
                    }
                    self.command_reader.clear();
                    self.message.clear();
                }
                Err(error) => {
                    self.message = crate::message::Message::error(i18n::tf!(
                        ReadCommandHistoryFailed,
                        format!("{error:#}")
                    ))
                }
            }
            return true;
        }
        let InputBar::Command {
            input,
            selected,
            menu,
            ..
        } = self.active_bar_mut()
        else {
            return false;
        };
        let Menu::History(history) = menu else {
            return false;
        };
        match key.code {
            KeyCode::Up => history.advance(true),
            KeyCode::Down => history.advance(false),
            KeyCode::PageUp | KeyCode::PageDown => return false,
            KeyCode::Enter | KeyCode::Tab => {
                let Some(editor) = history.choose() else {
                    return true;
                };
                // 历史保存完整命令；仓库和选中目标仍使用本次打开菜单时捕获的上下文。
                *input = command::Input::global(editor);
                *menu = Menu::Commands;
                *selected = 0;
                if key.code == KeyCode::Enter {
                    // 同一次 Enter 继续交给普通命令解析、目标校验和确认流程。
                    self.message.clear();
                    return false;
                }
                self.refresh_option_candidates();
            }
            KeyCode::Esc => *menu = Menu::Commands,
            _ => {
                *menu = Menu::Commands;
                return false;
            }
        }
        self.message.clear();
        true
    }
}
