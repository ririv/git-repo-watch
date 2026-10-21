//! Shared input validation and scrolling primitives.
mod actions;
mod keys;
mod navigation;
use super::*;
use anyhow::{Result, ensure};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use gw_core::state;
use std::path::Path;
fn validate_input(action: Action, value: &str) -> Result<()> {
    match action {
        Action::Add | Action::Exclude | Action::Unexclude | Action::Scan | Action::Import => {
            ensure!(Path::new(value).is_absolute(), "请输入绝对路径")
        }
        Action::Interval => {
            state::interval(value)?;
        }
        Action::Autopush => {
            value.parse::<domain::PushPolicy>()?;
        }
        Action::Concurrency => {
            let n = value.parse::<usize>()?;
            ensure!((1..=32).contains(&n), "并发数必须在 1 至 32 之间")
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
