//! Page candidate menus using their rendered row count.
use super::*;
use command::completion::Menu;

impl Screen {
    pub(super) fn command_page_key(&mut self, key: KeyEvent) -> bool {
        if !matches!(key.code, KeyCode::PageUp | KeyCode::PageDown) {
            return false;
        }
        let InputBar::Command { selected, menu, .. } = &self.bar else {
            return false;
        };
        let (current, count) = match menu {
            Menu::Commands => (
                *selected,
                self.search_commands().expect("command menu").len(),
            ),
            Menu::Options(values) => (values.selected, values.options.len()),
            Menu::Directories(values) => (values.selected(), values.names().len()),
        };
        if count == 0 {
            return true;
        }
        let visible = self.command_items.len();
        let next = if key.code == KeyCode::PageUp {
            current.saturating_sub(visible)
        } else {
            current.saturating_add(visible)
        }
        .min(count - 1);
        let InputBar::Command { selected, menu, .. } = &mut self.bar else {
            unreachable!("command menu remains open")
        };
        match menu {
            Menu::Commands => *selected = next,
            Menu::Options(values) => values.selected = next,
            Menu::Directories(values) => {
                values.select(next);
            }
        }
        true
    }
}
