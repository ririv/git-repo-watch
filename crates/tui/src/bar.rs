//! Persistent input: plain text filters the current list; slash-prefixed text selects commands.
use crate::{Action, Editor, Mode, Order, Page, Screen, command};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use engine::{cancel::Cancel, domain::Filter};
use std::path::PathBuf;
#[derive(Debug)]
pub(crate) enum InputBar {
    Search(Editor),
    Command {
        editor: Editor,
        selected: usize,
        path: Option<PathBuf>,
    },
}
impl Default for InputBar {
    fn default() -> Self {
        Self::Search(Editor::default())
    }
}
impl InputBar {
    pub fn editor(&self) -> &Editor {
        match self {
            Self::Search(editor) | Self::Command { editor, .. } => editor,
        }
    }
    pub fn editor_mut(&mut self) -> &mut Editor {
        match self {
            Self::Search(editor) | Self::Command { editor, .. } => editor,
        }
    }
    pub fn commands(&self) -> Option<Vec<&'static command::Command>> {
        match self {
            Self::Command { editor, .. } => Some(command::matches(
                editor
                    .text
                    .strip_prefix('/')
                    .expect("command input starts with slash"),
            )),
            Self::Search(_) => None,
        }
    }
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum BarEvent {
    Handled,
    Pass,
    Quit,
}
impl Screen {
    pub(crate) fn sync_bar(&mut self) {
        self.bar = InputBar::Search(Editor::new(
            self.mode
                .file_session()
                .map(|s| s.query.clone())
                .unwrap_or_else(|| self.query.clone()),
        ))
    }
    fn normalize_bar(&mut self, path: Option<PathBuf>) {
        let command = self.bar.editor().text.starts_with('/');
        match (&self.bar, command) {
            (InputBar::Search(_), true) => {
                let old = std::mem::take(&mut self.bar);
                let InputBar::Search(editor) = old else {
                    unreachable!()
                };
                self.bar = InputBar::Command {
                    editor,
                    selected: 0,
                    path,
                };
            }
            (InputBar::Command { .. }, false) => {
                let old = std::mem::take(&mut self.bar);
                let InputBar::Command { editor, .. } = old else {
                    unreachable!()
                };
                self.bar = InputBar::Search(editor);
            }
            _ => {}
        }
        if let InputBar::Search(editor) = &self.bar {
            let query = editor.text.clone();
            match &mut self.mode {
                Mode::Files(s) if s.query != query => {
                    s.query = query;
                    s.cursor = 0
                }
                Mode::Diff { session, .. } if session.query != query => {
                    let mut session = session.clone();
                    session.query = query;
                    session.cursor = 0;
                    self.mode = Mode::Files(session)
                }
                Mode::Browsing if self.query != query => {
                    self.query = query;
                    self.selected = 0;
                    self.page = Page::Repositories;
                }
                _ => {}
            }
        }
    }
    pub(crate) fn bar_paste(&mut self, text: &str) {
        let path = self.scope_path();
        self.bar.editor_mut().insert(text);
        self.normalize_bar(path);
        if let InputBar::Command { selected, .. } = &mut self.bar {
            *selected = 0
        }
        self.message.clear();
    }
    pub(crate) fn bar_key(&mut self, key: KeyEvent, root: &Cancel) -> Result<BarEvent> {
        if !matches!(
            self.mode,
            Mode::Browsing | Mode::Files(_) | Mode::Diff { .. }
        ) {
            return Ok(BarEvent::Pass);
        }
        if let InputBar::Command {
            editor,
            selected,
            path,
        } = &mut self.bar
        {
            let matches = command::matches(
                editor
                    .text
                    .strip_prefix('/')
                    .expect("command input starts with slash"),
            );
            match key.code {
                KeyCode::Esc => self.clear_bar(),
                KeyCode::Up => *selected = selected.saturating_sub(1),
                KeyCode::Down => *selected = (*selected + 1).min(matches.len().saturating_sub(1)),
                KeyCode::Tab => {
                    if let Some(c) = matches.get(*selected) {
                        *editor = Editor::new(format!("/{}", c.name));
                        *selected = 0
                    }
                }
                KeyCode::Enter => {
                    let action = command::exact(
                        editor
                            .text
                            .strip_prefix('/')
                            .expect("command input starts with slash"),
                    );
                    let path = path.clone();
                    if let Some(action) = action {
                        self.sync_bar();
                        self.message.clear();
                        if self.activate(action, path, root)? {
                            return Ok(BarEvent::Quit);
                        }
                    } else {
                        self.message = i18n::tr!(UnknownCommandUseTabToCompleteBefore).into();
                    }
                }
                _ => {
                    let path = path.clone();
                    editor.key(key);
                    *selected = 0;
                    self.normalize_bar(path);
                    self.message.clear();
                }
            }
            return Ok(BarEvent::Handled);
        }
        if key.code == KeyCode::Esc && !self.bar.editor().text.is_empty() {
            self.clear_bar();
            return Ok(BarEvent::Handled);
        }
        if key.code == KeyCode::Char('f')
            && key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(self.mode, Mode::Browsing)
            && self.page == Page::Repositories
        {
            let filters = [
                Filter::All,
                Filter::Problems,
                Filter::WithoutUpstream,
                Filter::Pushes,
            ];
            let index = filters
                .iter()
                .position(|filter| *filter == self.filter)
                .expect("known filter");
            let path = self.selected_path();
            self.filter = filters[(index + 1) % filters.len()];
            self.focus(path);
            return Ok(BarEvent::Handled);
        }
        if key.modifiers.contains(KeyModifiers::ALT) && key.code == KeyCode::Char('o') {
            let path = self.selected_path();
            self.order = if self.order == Order::Priority {
                Order::Path
            } else {
                Order::Priority
            };
            self.focus(path);
            return Ok(BarEvent::Handled);
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('r') => {
                    self.activate(Action::Refresh, self.scope_path(), root)?;
                    return Ok(BarEvent::Handled);
                }
                KeyCode::Char('a') if matches!(self.mode, Mode::Files(_)) => {
                    if let Mode::Files(s) = &mut self.mode {
                        let indices = s
                            .rows()
                            .iter()
                            .map(|c| s.change_index(c))
                            .collect::<Vec<_>>();
                        if indices.iter().all(|i| s.selected.contains(i)) {
                            for i in indices {
                                s.selected.remove(&i);
                            }
                        } else {
                            s.selected.extend(indices)
                        }
                    }
                    return Ok(BarEvent::Handled);
                }
                _ => {}
            }
        }
        if key.code == KeyCode::Char(' ')
            && self.bar.editor().text.is_empty()
            && matches!(self.mode, Mode::Files(_))
        {
            return Ok(BarEvent::Pass);
        }
        let editing = matches!(
            key.code,
            KeyCode::Char(_)
                | KeyCode::Backspace
                | KeyCode::Delete
                | KeyCode::Left
                | KeyCode::Right
                | KeyCode::Home
                | KeyCode::End
        );
        if editing {
            let path = self.scope_path();
            self.bar.editor_mut().key(key);
            self.normalize_bar(path);
            self.message.clear();
            return Ok(BarEvent::Handled);
        }
        Ok(BarEvent::Pass)
    }
    fn clear_bar(&mut self) {
        let path = self.selected_path();
        self.bar = InputBar::default();
        match &mut self.mode {
            Mode::Files(s) | Mode::Diff { session: s, .. } => {
                s.query.clear();
                s.cursor = 0;
            }
            _ => {
                self.query.clear();
                self.page = Page::Repositories;
                self.scroll = 0;
                self.focus(path)
            }
        }
        self.message.clear();
    }
}
