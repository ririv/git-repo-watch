//! Shared input validation and scrolling primitives.
mod actions;
mod keys;
mod mouse;
mod navigation;
pub(crate) mod scroll;
use super::*;
use anyhow::{Result, ensure};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use engine::state;
use std::path::Path;
fn validate_input(action: Action, value: &str) -> Result<()> {
    match action {
        Action::Add | Action::Exclude | Action::Unexclude | Action::Scan | Action::Import => {
            ensure!(
                Path::new(value).is_absolute(),
                i18n::tr!(EnterAnAbsolutePath)
            )
        }
        Action::Interval => {
            state::interval(value)?;
        }
        Action::Autopush => {
            value.parse::<domain::PushPolicy>()?;
        }
        Action::Concurrency => {
            let n = value.parse::<usize>()?;
            ensure!(
                (1..=32).contains(&n),
                i18n::tr!(ConcurrencyMustBeBetween1And32)
            )
        }
        _ => {}
    }
    Ok(())
}
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
