//! Persistent input: plain text filters the current list; slash-prefixed text selects commands.
use crate::{Action, Editor, Mode, Page, Screen, command};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use engine::cancel::Cancel;
use std::path::PathBuf;
#[derive(Debug)]
pub(crate) struct CommandDraft {
    pub input: command::Input,
    pub path: Option<PathBuf>,
    pub selection: crate::clipboard::Selection,
}
#[derive(Debug)]
pub(crate) enum InputBar {
    Search(Editor),
    Layout {
        selected: crate::diff::Preference,
        command_cursor: usize,
        draft: CommandDraft,
    },
    Command {
        input: command::Input,
        selected: usize,
        menu: command::path::Menu,
        path: Option<PathBuf>,
        selection: crate::clipboard::Selection,
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
            Self::Search(editor) => editor,
            Self::Command { input, .. } => input.editor(),
            Self::Layout { draft, .. } => draft.input.editor(),
        }
    }
    pub fn editor_mut(&mut self) -> &mut Editor {
        match self {
            Self::Search(editor) => editor,
            Self::Command { input, .. } => input.editor_mut(),
            Self::Layout { draft, .. } => draft.input.editor_mut(),
        }
    }
    pub fn commands(&self) -> Option<Vec<&'static command::Command>> {
        match self {
            Self::Command {
                input,
                path,
                selection,
                ..
            } => Some(
                input
                    .matches()
                    .into_iter()
                    .filter(|command| selection.admits(command.action, path.as_deref()))
                    .collect(),
            ),
            Self::Search(_) | Self::Layout { .. } => None,
        }
    }
    pub fn text(&self) -> String {
        match self {
            Self::Command { input, .. } => input.text(),
            Self::Layout { draft, .. } => draft.input.text(),
            Self::Search(editor) => editor.text.clone(),
        }
    }
    pub fn command_input(&self) -> Option<&command::Input> {
        match self {
            Self::Command { input, .. } => Some(input),
            _ => None,
        }
    }
    pub fn selection(&self) -> Option<&crate::clipboard::Selection> {
        match self {
            Self::Command { selection, .. } => Some(selection),
            _ => None,
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
        self.bar = InputBar::Search(Editor::new(if let Mode::History(view) = &self.mode {
            view.query().to_owned()
        } else if let Mode::Stashes(view) = &self.mode {
            view.session().query.clone()
        } else if let Mode::Branches(view) = &self.mode {
            view.session().query.clone()
        } else if matches!(self.mode, Mode::Panel(_)) {
            String::new()
        } else {
            self.mode
                .file_session()
                .map(|s| s.query.clone())
                .unwrap_or_else(|| self.query.clone())
        }))
    }
    fn normalize_bar(&mut self) {
        if matches!(&self.bar, InputBar::Command { input: command::Input::Global(editor), .. } if !editor.text.starts_with('/'))
        {
            let InputBar::Command { input, .. } = std::mem::take(&mut self.bar) else {
                unreachable!()
            };
            self.bar = InputBar::Search(input.into_search().expect("global command lost slash"));
        }
        if let InputBar::Search(editor) = &self.bar {
            let query = editor.text.clone();
            match &mut self.mode {
                Mode::Stashes(view) => view.search(query),
                Mode::Branches(view) => view.search(query),
                Mode::History(view) => view.search(query),
                Mode::Files(s) if s.query != query => {
                    s.query = query;
                    s.cursor = 0
                }
                Mode::Changes(session) if session.files.query != query => {
                    session.files.query = query;
                    session.files.cursor = 0;
                    session.focus = crate::changes::Focus::Files;
                }
                Mode::Browsing if self.query != query => {
                    self.query = query;
                    self.select(0);
                    self.page = Page::Repositories;
                }
                _ => {}
            }
        }
    }
    pub(crate) fn bar_paste(&mut self, text: &str) {
        if matches!(self.bar, InputBar::Layout { .. }) {
            return;
        }
        if matches!(self.bar, InputBar::Search(_))
            && let Some(body) = text.strip_prefix('/')
        {
            self.open_command_bar();
            let InputBar::Command { input, .. } = &mut self.bar else {
                unreachable!()
            };
            input.paste_initial(body);
        } else if let InputBar::Command { input, .. } = &mut self.bar {
            input.paste(text);
        } else {
            self.bar.editor_mut().insert(text);
        }
        self.normalize_bar();
        if let InputBar::Command { selected, menu, .. } = &mut self.bar {
            *menu = Default::default();
            *selected = 0
        }
        self.message.clear();
    }
    pub(crate) fn bar_key(&mut self, key: KeyEvent, root: &Cancel) -> Result<BarEvent> {
        // 错误页只开放复制与退出；Esc 先关闭命令，再交给错误页返回。
        if matches!(self.mode, Mode::Error { .. }) && !matches!(self.bar, InputBar::Command { .. })
        {
            if key.code == KeyCode::Char('/') && key.modifiers.is_empty() {
                self.open_command_bar();
                return Ok(BarEvent::Handled);
            }
            return Ok(BarEvent::Pass);
        }
        if !matches!(
            self.mode,
            Mode::Browsing
                | Mode::Panel(_)
                | Mode::Stashes(_)
                | Mode::Branches(_)
                | Mode::Files(_)
                | Mode::Changes(_)
                | Mode::History(_)
                | Mode::Error { .. }
        ) {
            return Ok(BarEvent::Pass);
        }
        if self.layout_picker_key(key) {
            return Ok(BarEvent::Handled);
        }
        if let InputBar::Command {
            input,
            selected,
            menu,
            path,
            selection,
        } = &mut self.bar
        {
            match input.namespace_key(key) {
                command::NamespaceEvent::Close => {
                    self.clear_bar();
                    return Ok(BarEvent::Handled);
                }
                command::NamespaceEvent::Handled => {
                    *selected = 0;
                    *menu = Default::default();
                    self.message.clear();
                    return Ok(BarEvent::Handled);
                }
                command::NamespaceEvent::Pass => {}
            }
            if let command::path::Menu::Directories(directories) = menu {
                match key.code {
                    KeyCode::Esc => {
                        *menu = Default::default();
                        self.message.clear();
                    }
                    KeyCode::Up | KeyCode::BackTab => directories.advance(true),
                    KeyCode::Down | KeyCode::Tab => directories.advance(false),
                    KeyCode::Enter => {
                        input.replace_canonical(directories.choose());
                        *menu = Default::default();
                        self.message.clear();
                    }
                    _ => {
                        *menu = Default::default();
                        input.editor_mut().key(key);
                        self.normalize_bar();
                        self.message.clear();
                    }
                }
                return Ok(BarEvent::Handled);
            }
            let matches = input
                .matches()
                .into_iter()
                .filter(|command| selection.admits(command.action, path.as_deref()))
                .collect::<Vec<_>>();
            let query = input.query();
            match key.code {
                KeyCode::Esc => self.clear_bar(),
                KeyCode::Up => *selected = selected.saturating_sub(1),
                KeyCode::Down => *selected = (*selected + 1).min(matches.len().saturating_sub(1)),
                KeyCode::Tab => {
                    if let Some(c) = matches.get(*selected) {
                        if command::argument_input(&query).is_some() {
                            let mut canonical = input.canonical_editor();
                            match command::path::complete(
                                &mut canonical,
                                self.service.scheduler.user_home(),
                            ) {
                                Ok(completion) => {
                                    input.replace_canonical(canonical);
                                    *menu = completion;
                                    self.message.clear();
                                }
                                Err(error) => self.message = format!("{error:#}"),
                            }
                        } else {
                            selection.complete(input, c);
                            self.message.clear();
                        }
                        *selected = 0
                    }
                }
                KeyCode::Enter => {
                    if !selection.is_error()
                        && let Some((argument, value)) = command::argument_input(&query)
                    {
                        match argument.parse(value, self.service.scheduler.user_home()) {
                            Ok(value) => {
                                let draft = CommandDraft {
                                    input: input.clone(),
                                    path: path.clone(),
                                    selection: selection.clone(),
                                };
                                match argument.mutation(&value) {
                                    Ok(Some(request)) => self.prepare_mutation(request, draft),
                                    Ok(None) => self.prepare_configuration(
                                        argument.configuration().expect("configuration parameter"),
                                        value,
                                        draft,
                                    ),
                                    Err(error) => self.message = format!("{error:#}"),
                                }
                            }
                            Err(error) => self.message = format!("{error:#}"),
                        }
                    } else if let Some(command) = matches.get(*selected) {
                        if command.action == Action::DiffLayoutMenu {
                            let draft = CommandDraft {
                                input: input.clone(),
                                path: path.clone(),
                                selection: selection.clone(),
                            };
                            let cursor = *selected;
                            self.open_layout_picker(draft, cursor);
                        } else if command.action.argument().is_some() {
                            selection.complete(input, command);
                            *selected = 0;
                            self.message.clear();
                        } else {
                            let action = command.action;
                            let path = path.clone();
                            let selection = selection.clone();
                            self.sync_bar();
                            self.message.clear();
                            if self.activate(action, path, selection, root)? {
                                return Ok(BarEvent::Quit);
                            }
                        }
                    } else {
                        self.message = i18n::tr!(NoMatchingCommands).into();
                    }
                }
                _ => {
                    input.editor_mut().key(key);
                    *selected = 0;
                    self.normalize_bar();
                    self.message.clear();
                }
            }
            return Ok(BarEvent::Handled);
        }
        if key.code == KeyCode::Esc && !self.bar.editor().text.is_empty() {
            self.clear_bar();
            return Ok(BarEvent::Handled);
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('r') => {
                    self.activate(
                        Action::Refresh,
                        self.scope_path(),
                        crate::clipboard::Selection::None,
                        root,
                    )?;
                    return Ok(BarEvent::Handled);
                }
                KeyCode::Char('a') if matches!(self.mode, Mode::Files(_) | Mode::Changes(_)) => {
                    let files = match &mut self.mode {
                        Mode::Files(files) => Some(files),
                        Mode::Changes(session) => Some(&mut session.files),
                        _ => None,
                    };
                    if let Some(s) = files {
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
            && matches!(self.mode, Mode::Files(_) | Mode::Changes(_))
        {
            return Ok(BarEvent::Pass);
        }
        // 空搜索栏把无修饰的左右键交给页面导航；有文字时仍移动输入光标。
        if self.bar.editor().text.is_empty()
            && (matches!(
                self.mode,
                Mode::History(_)
                    | Mode::Stashes(_)
                    | Mode::Branches(_)
                    | Mode::Changes(_)
                    | Mode::Files(_)
            ) || (matches!(self.mode, Mode::Browsing) && self.page == Page::Detail))
            && matches!(key.code, KeyCode::Home | KeyCode::End)
        {
            return Ok(BarEvent::Pass);
        }
        if matches!(&self.bar, InputBar::Search(editor) if editor.text.is_empty())
            && key.modifiers.is_empty()
            && matches!(key.code, KeyCode::Left | KeyCode::Right)
        {
            return Ok(BarEvent::Pass);
        }
        // 按 / 捕获当前视图和仓库，独立保存命令草稿，保留已有搜索。
        if key.code == KeyCode::Char('/')
            && key.modifiers.is_empty()
            && (self.repository_list_active() || self.bar.editor().cursor == 0)
        {
            self.open_command_bar();
            self.message.clear();
            return Ok(BarEvent::Handled);
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
            self.bar.editor_mut().key(key);
            self.normalize_bar();
            self.message.clear();
            return Ok(BarEvent::Handled);
        }
        Ok(BarEvent::Pass)
    }
    fn open_command_bar(&mut self) {
        self.bar = InputBar::Command {
            input: command::Input::new(self.command_context()),
            selected: 0,
            menu: Default::default(),
            path: self.scope_path(),
            selection: self.copy_selection(),
        };
    }
    fn clear_bar(&mut self) {
        if matches!(self.bar, InputBar::Command { .. }) {
            self.sync_bar();
            self.message.clear();
            return;
        }
        let path = self.selected_path();
        self.bar = InputBar::default();
        match &mut self.mode {
            Mode::Panel(_) => {}
            Mode::Browsing
                if self.page == Page::Detail
                    && self.workspace.session().is_some_and(|session| {
                        session.arrangement == crate::workspace::Arrangement::Maximized
                    }) =>
            {
                self.query.clear();
                self.focus(path);
            }
            Mode::Stashes(view) => view.search(String::new()),
            Mode::Branches(view) => view.search(String::new()),
            Mode::History(view) => view.clear_search(),
            Mode::Files(s) => {
                s.query.clear();
                s.cursor = 0;
            }
            Mode::Changes(session) => {
                session.files.query.clear();
                session.files.cursor = 0;
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
