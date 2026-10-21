//! Repository and settings navigation.
use super::super::*;
use super::scroll_key;
use crossterm::event::{KeyCode, KeyEvent};
impl Screen {
    pub(crate) fn navigate(&mut self, key: KeyEvent, root: &Cancel) -> Result<bool> {
        match key.code {
            KeyCode::F(1) => {
                self.history.push(self.page);
                self.page = Page::Help;
                self.scroll = 0;
                return Ok(false);
            }
            KeyCode::Esc => {
                if let Some(page) = self.history.pop() {
                    self.page = page
                } else {
                    self.page = Page::Repositories
                }
                self.scroll = 0;
                self.sync_bar();
                return Ok(false);
            }
            KeyCode::Tab => {
                self.history.clear();
                self.page = if self.page == Page::Settings {
                    Page::Repositories
                } else {
                    Page::Settings
                };
                self.scroll = 0;
                self.sync_bar();
                return Ok(false);
            }
            _ => {}
        }
        match self.page {
            Page::Repositories => {
                let count = self.rows().len();
                match key.code {
                    KeyCode::Up => self.selected = self.selected.saturating_sub(1),
                    KeyCode::Down => {
                        self.selected = (self.selected + 1).min(count.saturating_sub(1))
                    }
                    KeyCode::PageDown => {
                        self.selected = (self.selected + 10).min(count.saturating_sub(1))
                    }
                    KeyCode::PageUp => self.selected = self.selected.saturating_sub(10),
                    KeyCode::Home => self.selected = 0,
                    KeyCode::End => self.selected = count.saturating_sub(1),
                    KeyCode::Enter if count > 0 => {
                        self.history.push(self.page);
                        self.page = Page::Detail;
                        self.scroll = 0
                    }
                    _ => {}
                }
            }
            Page::Settings => match key.code {
                KeyCode::Up => self.setting_selected = self.setting_selected.saturating_sub(1),
                KeyCode::Down => {
                    self.setting_selected =
                        (self.setting_selected + 1).min(command::SETTINGS.len() - 1)
                }
                KeyCode::Enter => {
                    return self.activate(
                        command::SETTINGS[self.setting_selected],
                        self.selected_path(),
                        root,
                    );
                }
                _ => scroll_key(&mut self.scroll, key),
            },
            Page::Detail | Page::Help => scroll_key(&mut self.scroll, key),
        }
        Ok(false)
    }
}
