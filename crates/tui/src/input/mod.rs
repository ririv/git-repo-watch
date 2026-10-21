//! Shared input validation and scrolling primitives.
mod actions;
mod keys;
mod mouse;
mod navigation;
pub(crate) mod scroll;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
fn ctrl_c(key: KeyEvent) -> bool {
    key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL)
}
fn scroll_key(scroll: &mut usize, key: KeyEvent) {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => *scroll = scroll.saturating_sub(1),
        KeyCode::Down | KeyCode::Char('j') => *scroll = scroll.saturating_add(1),
        KeyCode::PageUp => *scroll = scroll.saturating_sub(10),
        KeyCode::PageDown => *scroll = scroll.saturating_add(10),
        KeyCode::Home | KeyCode::Char('g') => *scroll = 0,
        _ => {}
    }
}
