//! Welcome screen for a profile that has never been initialized.
use super::Entry;
use crate::{TerminalEvent, TerminalGuard, read_terminal_event};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind};
use engine::cancel::Cancel;
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Layout, Position, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, List, ListState, Paragraph, Wrap},
};
use std::{io, path::Path, time::Duration};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Choice<'a> {
    CurrentRepository(&'a Path),
    Directory,
    Later,
}

impl Choice<'_> {
    fn label(self) -> &'static str {
        match self {
            Self::CurrentRepository(_) => i18n::tr!(SetupAddCurrent),
            Self::Directory => i18n::tr!(SetupChooseDirectory),
            Self::Later => i18n::tr!(SetupInitializeOnly),
        }
    }
}

pub(super) fn choose<'a>(
    entry: &'a Entry,
    config: &Path,
    cancel: &Cancel,
) -> Result<Option<Choice<'a>>> {
    let repo = match entry {
        Entry::Repository(path) => Some(path.as_path()),
        Entry::Repositories => None,
    };
    let choices = if let Some(repo) = repo {
        &[
            Choice::CurrentRepository(repo),
            Choice::Directory,
            Choice::Later,
        ][..]
    } else {
        &[Choice::Directory, Choice::Later][..]
    };
    let _guard = TerminalGuard::enter(Default::default())?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let mut options = ListState::default().with_selected(Some(0));
    let mut option_area = Rect::default();
    let mut dirty = true;
    while !cancel.is_cancelled() {
        if dirty {
            terminal.draw(|frame| {
                let panel = Block::bordered().title(i18n::tr!(SetupTitle));
                let area = panel.inner(frame.area());
                frame.render_widget(panel, frame.area());
                let intro = if let Some(repo) = repo {
                    format!(
                        "{}\n{}\n{}",
                        i18n::tr!(SetupIntro),
                        crate::view::safe(&config.to_string_lossy()),
                        crate::view::safe(&repo.to_string_lossy())
                    )
                } else {
                    format!(
                        "{}\n{}",
                        i18n::tr!(SetupIntro),
                        crate::view::safe(&config.to_string_lossy())
                    )
                };
                let intro = Paragraph::new(intro);
                let parts = Layout::vertical([
                    Constraint::Length(u16::from(area.height > 5) * 3),
                    Constraint::Min(1),
                    Constraint::Length(1),
                ])
                .split(area);
                frame.render_widget(intro.wrap(Wrap { trim: true }), parts[0]);
                option_area = parts[1];
                frame.render_stateful_widget(
                    List::new(choices.iter().map(|choice| choice.label()))
                        .highlight_symbol("› ")
                        .highlight_style(
                            Style::default()
                                .fg(Color::LightBlue)
                                .add_modifier(Modifier::BOLD),
                        ),
                    option_area,
                    &mut options,
                );
                frame.render_widget(Paragraph::new(i18n::tr!(SetupKeys)), parts[2]);
            })?;
            dirty = false;
        }
        let Some(event) = read_terminal_event(Duration::from_millis(80))? else {
            continue;
        };
        let selected = options.selected().expect("setup choice");
        match event {
            TerminalEvent::Key(key) if key.kind != KeyEventKind::Release => match key.code {
                KeyCode::Esc => return Ok(None),
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    return Ok(None);
                }
                KeyCode::Enter => return Ok(Some(choices[selected])),
                KeyCode::Up => options.select(Some(selected.saturating_sub(1))),
                KeyCode::Down => options.select(Some((selected + 1).min(choices.len() - 1))),
                _ => continue,
            },
            TerminalEvent::Mouse(mouse)
                if mouse.kind == MouseEventKind::Down(MouseButton::Left)
                    && option_area.contains(Position::new(mouse.column, mouse.row)) =>
            {
                let selected = options.offset() + usize::from(mouse.row - option_area.y);
                if selected < choices.len() {
                    options.select(Some(selected));
                }
            }
            TerminalEvent::Resize(..) => {}
            _ => continue,
        }
        dirty = true;
    }
    Ok(None)
}
