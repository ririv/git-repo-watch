//! Repository and settings navigation.
use super::super::*;
use super::scroll_key;
use crossterm::event::{KeyCode, KeyEvent};
impl Screen {
    pub(crate) fn navigate(&mut self, key: KeyEvent, root: &Cancel) -> Result<bool> {
        match key.code {
            KeyCode::F(1) => {
                self.open_page(Page::Help);
                return Ok(false);
            }
            KeyCode::Esc | KeyCode::Left => {
                self.return_to_view();
                return Ok(false);
            }
            _ => {}
        }
        match self.page {
            Page::Repositories => {
                let count = self.rows().len();
                match key.code {
                    KeyCode::Up => self.select(self.selected().saturating_sub(1)),
                    KeyCode::Down => self.select(self.selected().saturating_add(1)),
                    KeyCode::PageDown => self.select(self.selected().saturating_add(10)),
                    KeyCode::PageUp => self.select(self.selected().saturating_sub(10)),
                    KeyCode::Home => self.select(0),
                    KeyCode::End => self.select(count.saturating_sub(1)),
                    KeyCode::Enter | KeyCode::Right if count > 0 => {
                        if let Some(path) = self.selected_path() {
                            self.open_repository(path);
                        }
                    }
                    _ => {}
                }
            }
            Page::Configuration(section) => match key.code {
                KeyCode::Up => self.setting_selected = self.setting_selected.saturating_sub(1),
                KeyCode::Down => {
                    self.setting_selected = (self.setting_selected + 1).min(
                        section
                            .items(self.snapshot.registration, &self.config.ai)
                            .len()
                            .saturating_sub(1),
                    )
                }
                KeyCode::Enter | KeyCode::Right => {
                    if let Some(item) = section
                        .items(self.snapshot.registration, &self.config.ai)
                        .get(self.setting_selected)
                    {
                        return self.activate_setting(*item, root);
                    }
                }
                _ => scroll_key(&mut self.scroll, key),
            },
            Page::Detail => match key.code {
                KeyCode::Enter | KeyCode::Right => {
                    self.switch_repository_section(crate::workspace::Section::Overview);
                }
                _ => self.scroll_overview(key),
            },
            Page::View(_) => {}
            Page::Help => scroll_key(&mut self.scroll, key),
        }
        Ok(false)
    }
    pub(super) fn scroll_overview(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::End => self.scroll = usize::MAX,
            KeyCode::PageUp | KeyCode::PageDown => {
                if let Some((area, _)) = self
                    .workspace_areas
                    .iter()
                    .find(|(_, section)| *section == crate::workspace::Section::Overview)
                {
                    let height = usize::from(area.height.saturating_sub(2));
                    self.scroll = if key.code == KeyCode::PageUp {
                        self.scroll.saturating_sub(height)
                    } else {
                        self.scroll.saturating_add(height)
                    };
                }
            }
            _ => scroll_key(&mut self.scroll, key),
        }
    }
}
