//! Persistent input: plain text filters the current list; slash-prefixed text selects commands.
mod complete;
mod navigation;
mod options;
use crate::{Action, Editor, Mode, Page, Screen, command};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use engine::cancel::Cancel;
use std::path::PathBuf;
#[derive(Debug, Clone, Copy)]
pub(crate) enum ReturnTo {
    Command,
    View,
}
#[derive(Debug, Clone)]
pub(crate) struct CommandDraft {
    pub input: command::Input,
    pub path: Option<PathBuf>,
    pub selection: crate::clipboard::Selection,
    pub repository: command::repository::Captured,
    pub return_to: ReturnTo,
}
#[derive(Debug)]
pub(crate) enum InputBar {
    Search(Editor),
    CommandsOnly,
    Layout {
        selected: crate::diff::Preference,
        command_cursor: usize,
        draft: CommandDraft,
    },
    Command {
        input: command::Input,
        selected: usize,
        menu: command::completion::Menu,
        path: Option<PathBuf>,
        selection: crate::clipboard::Selection,
        repository: command::repository::Captured,
    },
}
impl Default for InputBar {
    fn default() -> Self {
        Self::Search(Editor::default())
    }
}
impl InputBar {
    pub fn is_plain(&self) -> bool {
        matches!(self, Self::Search(_) | Self::CommandsOnly)
    }
    pub fn editor(&self) -> &Editor {
        // 命令入口没有可编辑的搜索草稿，只提供空内容用于读取光标和显示状态。
        static EMPTY: Editor = Editor {
            text: String::new(),
            cursor: 0,
        };
        match self {
            Self::Search(editor) => editor,
            Self::CommandsOnly => &EMPTY,
            Self::Command { input, .. } => input.editor(),
            Self::Layout { draft, .. } => draft.input.editor(),
        }
    }
    pub fn text(&self) -> String {
        match self {
            Self::Command { input, .. } => input.text(),
            Self::Layout { draft, .. } => draft.input.text(),
            Self::Search(editor) => editor.text.clone(),
            Self::CommandsOnly => String::new(),
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
    pub(crate) fn capture_repository(
        &self,
        path: Option<&std::path::Path>,
    ) -> command::repository::Captured {
        if let Some(path) = path
            && let InputBar::Command { repository, .. } = &self.bar
            && repository.matches(path)
        {
            return repository.clone();
        }
        command::repository::Captured::new(path.map(std::path::Path::to_owned))
    }
    pub(crate) fn poll_command_repository(&mut self) -> bool {
        let captured = match &self.bar {
            InputBar::Command { repository, .. } => repository.clone(),
            InputBar::Layout { draft, .. } => draft.repository.clone(),
            _ => Default::default(),
        };
        self.command_repository.poll(&captured, &self.service)
    }
    pub(crate) fn sync_plain_bar(&mut self) {
        if self.tasks.session.open() {
            return;
        }
        let searchable = self.search_support() != crate::search::Support::Unavailable;
        if matches!(
            (&self.bar, searchable),
            (InputBar::CommandsOnly, true) | (InputBar::Search(_), false)
        ) || matches!((&self.mode, &self.bar), (Mode::Browser(session), InputBar::Search(editor)) if searchable && editor.text != session.input_query())
        {
            self.sync_bar();
        }
    }
    pub(crate) fn sync_bar(&mut self) {
        if self.search_support() == crate::search::Support::Unavailable {
            self.bar = InputBar::CommandsOnly;
            return;
        }
        let query = match &self.mode {
            Mode::Browsing => self.query.clone(),
            Mode::Management(session) => session.query.clone(),
            Mode::Browser(session) => session.input_query().to_owned(),
            Mode::History(view) => view.query().to_owned(),
            Mode::Stashes(view) => view.query().to_owned(),
            Mode::Branches(view) => view.session().query.clone(),
            Mode::Files(files) => files.query.clone(),
            Mode::Changes(session) => session.files.query.clone(),
            _ => unreachable!("search capability requires a visible searchable list"),
        };
        self.bar = InputBar::Search(Editor::new(query));
    }
    fn normalize_bar(&mut self) {
        if matches!(&self.bar, InputBar::Command { input: command::Input::Global(editor), .. } if !editor.text.starts_with('/'))
        {
            let InputBar::Command { input, .. } = std::mem::take(&mut self.bar) else {
                unreachable!()
            };
            self.bar = if self.search_support() == crate::search::Support::Unavailable {
                InputBar::CommandsOnly
            } else {
                InputBar::Search(input.into_search().expect("global command lost slash"))
            };
        }
        if self.search_support() != crate::search::Support::Unavailable
            && let InputBar::Search(editor) = &self.bar
        {
            let query = editor.text.clone();
            match &mut self.mode {
                Mode::Browser(session) => session.search_input(query),
                Mode::Management(session) => session.search(query),
                Mode::Stashes(view) => view.search(query),
                Mode::Branches(view) => view.search(query),
                Mode::History(view) => view.search(query),
                Mode::Files(s) => s.search(query),
                Mode::Changes(session) if session.files.query != query => {
                    session.files.search(query);
                    session.focus = crate::changes::Focus::Files;
                }
                Mode::Browsing if self.query != query => {
                    self.query = query;
                    match self.search_mode {
                        crate::search::Mode::Filter => self.select(0),
                        crate::search::Mode::Locate => {
                            if let Some(index) = self
                                .repository_matches()
                                .nearest(self.repository_viewport.selected())
                            {
                                self.select(index);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    pub(crate) fn bar_paste(&mut self, text: &str) {
        let text = self.command_trigger_text(text);
        let text = text.as_ref();
        if matches!(self.bar, InputBar::Layout { .. }) {
            return;
        }
        if self.bar.is_plain()
            && (self.at_command_entry() || text.starts_with("//"))
            && let Some(body) = text.strip_prefix('/')
        {
            self.open_command_bar();
            let InputBar::Command { input, .. } = &mut self.bar else {
                unreachable!()
            };
            input.paste_initial(body);
        } else if let InputBar::Command { input, .. } = &mut self.bar {
            input.paste(text);
        } else if let InputBar::Search(editor) = &mut self.bar {
            editor.insert(text);
        }
        self.normalize_bar();
        if let InputBar::Command {
            selected,
            menu,
            input,
            ..
        } = &mut self.bar
        {
            if let command::completion::Menu::Directories(values) = menu {
                match values.refine(
                    &input.canonical_editor(),
                    self.service.scheduler.user_home(),
                ) {
                    Ok(true) => {}
                    Ok(false) => *menu = Default::default(),
                    Err(error) => *menu = command::completion::Menu::Failed(format!("{error:#}")),
                }
            } else if !matches!(
                menu,
                command::completion::Menu::Values(_)
                    | command::completion::Menu::Loading(
                        command::loader::Request::Values { .. }
                            | command::loader::Request::Editors { .. }
                    )
            ) {
                *menu = Default::default();
            }
            *selected = 0
        }
        self.refresh_option_candidates();
        self.message.clear();
    }
    pub(crate) fn bar_key(&mut self, key: KeyEvent, root: &Cancel) -> Result<BarEvent> {
        let key = self.command_trigger_key(key);
        // 错误页开放复制、任务与退出；Esc 先关闭命令，再交给错误页返回。
        if matches!(self.mode, Mode::Error { .. }) && !matches!(self.bar, InputBar::Command { .. })
        {
            if key.code == KeyCode::Char('/') && key.modifiers.is_empty() {
                self.open_command_bar();
                return Ok(BarEvent::Handled);
            }
            return Ok(BarEvent::Pass);
        }
        if matches!(&self.mode, Mode::Composition(session) if !matches!(session.state, crate::composition::State::Browsing))
        {
            return Ok(BarEvent::Pass);
        }
        if !matches!(
            self.mode,
            Mode::Management(_)
                | Mode::Browsing
                | Mode::Composition(_)
                | Mode::CompositionWaiting(_)
                | Mode::Browser(_)
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
        if self.empty_layout_picker_key(key) {
            return Ok(BarEvent::Handled);
        }
        if self.command_page_key(key) {
            return Ok(BarEvent::Handled);
        }
        if key.code == KeyCode::Enter
            && matches!(&self.bar, InputBar::Command { input, menu: command::completion::Menu::Loading(command::loader::Request::Values { .. } | command::loader::Request::Editors { .. }), .. }
                if command::argument_input(&input.query()).is_some_and(|(argument, value)| !value.trim().is_empty() && argument.parse(value, self.service.scheduler.user_home()).is_ok()))
        {
            if let InputBar::Command { menu, .. } = &mut self.bar {
                *menu = Default::default();
            }
            self.command_reader.clear();
        }
        if matches!(
            self.bar,
            InputBar::Command {
                menu: command::completion::Menu::Loading(_),
                ..
            }
        ) {
            match key.code {
                KeyCode::Enter | KeyCode::Tab | KeyCode::Up | KeyCode::Down | KeyCode::BackTab => {
                    return Ok(BarEvent::Handled);
                }
                KeyCode::Esc => {
                    if let InputBar::Command { menu, .. } = &mut self.bar {
                        *menu = Default::default();
                    }
                    self.command_reader.clear();
                    self.message.clear();
                    return Ok(BarEvent::Handled);
                }
                _ => {}
            }
        }
        if self.option_candidate_key(key) {
            return Ok(BarEvent::Handled);
        }
        let matches = self.search_commands().unwrap_or_default();
        if key.code == KeyCode::Enter
            && let InputBar::Command {
                input,
                path,
                selection,
                selected,
                ..
            } = &self.bar
            && !input.prefix_selected()
        {
            let query = input.query();
            let action = command::argument_input(&query)
                .map(|(argument, _)| argument.action())
                .or_else(|| self.out_of_view_command(&query, path.as_deref(), selection))
                .or_else(|| matches.get(*selected).map(|command| command.action));
            if let Some(action) = action
                && let Some(reason) = self
                    .command_availability(action, path.as_deref(), selection)
                    .reason()
            {
                self.message = reason.to_owned();
                return Ok(BarEvent::Handled);
            }
        }
        if let InputBar::Command {
            input,
            selected,
            menu,
            path,
            selection,
            repository,
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
            let values_menu = matches!(menu, command::completion::Menu::Values(_));
            if let Some(candidates) = menu.selection() {
                match key.code {
                    KeyCode::Esc => {
                        *menu = Default::default();
                        self.message.clear();
                    }
                    KeyCode::Up => candidates.advance(true),
                    KeyCode::Down => candidates.advance(false),
                    KeyCode::Enter | KeyCode::Tab => {
                        let execute = key.code == KeyCode::Enter && values_menu;
                        if let Some(editor) = candidates.choose() {
                            input.replace_canonical(editor);
                            *menu = Default::default();
                            self.message.clear();
                            if execute {
                                return self.bar_key(key, root);
                            }
                        } else {
                            self.message = i18n::tr!(NoMatchingOptions).into();
                        }
                    }
                    _ => {
                        input.editor_mut().key(key);
                        if let command::completion::Menu::Directories(values) = menu {
                            match values.refine(
                                &input.canonical_editor(),
                                self.service.scheduler.user_home(),
                            ) {
                                Ok(true) => {}
                                Ok(false) => *menu = Default::default(),
                                Err(error) => {
                                    *menu = command::completion::Menu::Failed(format!("{error:#}"))
                                }
                            }
                        }
                        self.normalize_bar();
                        self.refresh_option_candidates();
                        self.message.clear();
                    }
                }
                return Ok(BarEvent::Handled);
            }
            let query = input.query();
            match key.code {
                KeyCode::Esc => self.clear_bar(),
                KeyCode::Up => *selected = selected.saturating_sub(1),
                KeyCode::Down => *selected = (*selected + 1).min(matches.len().saturating_sub(1)),
                KeyCode::Tab => {
                    if let Some(c) = matches.get(*selected) {
                        if command::argument_input(&query)
                            .is_some_and(|(argument, _)| argument.options().is_some())
                        {
                            self.message.clear();
                        } else if command::argument_input(&query).is_some() {
                            if command::argument_input(&query)
                                .is_some_and(|(argument, _)| argument.is_path())
                            {
                                self.start_path_completion();
                                return Ok(BarEvent::Handled);
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
                        if !argument.action().available(&self.config) {
                            self.message = i18n::tr!(NoMatchingCommands).into();
                            return Ok(BarEvent::Handled);
                        }
                        if let command::ArgumentAction::Option(kind) = argument {
                            let action = kind.parse(value).map(|value| value.action());
                            let target = path.clone();
                            let selection = selection.clone();
                            match action {
                                Ok(action) => {
                                    self.sync_bar();
                                    self.activate(action, target, selection, root)?;
                                }
                                Err(error) => self.message = format!("{error:#}"),
                            }
                            return Ok(BarEvent::Handled);
                        }
                        if argument.action() == Action::AiPrompt && value.trim().is_empty() {
                            self.activate(
                                Action::AiPrompt,
                                None,
                                crate::clipboard::Selection::None,
                                root,
                            )?;
                            return Ok(BarEvent::Handled);
                        }
                        if argument == command::ArgumentAction::Scan {
                            let value = value.to_owned();
                            self.scan_command(&value);
                            return Ok(BarEvent::Handled);
                        }
                        match argument.parse(value, self.service.scheduler.user_home()) {
                            Ok(value) if argument == command::ArgumentAction::Editor => {
                                self.select_editor(value.try_into()?);
                            }
                            Ok(value) if argument == command::ArgumentAction::AiAgent => {
                                self.select_agent(value.parse()?);
                            }
                            Ok(value) => {
                                let draft = CommandDraft {
                                    input: input.clone(),
                                    path: path.clone(),
                                    selection: selection.clone(),
                                    repository: repository.clone(),
                                    return_to: ReturnTo::Command,
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
                                repository: repository.clone(),
                                return_to: ReturnTo::Command,
                            };
                            let cursor = *selected;
                            self.open_layout_picker(draft, cursor);
                        } else if command.action.argument().is_some()
                            && !matches!(command.action, Action::AiPrompt | Action::LogSelect)
                        {
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
                    if matches!(
                        menu,
                        command::completion::Menu::Loading(
                            command::loader::Request::Directory { .. }
                        ) | command::completion::Menu::Failed(_)
                    ) {
                        *menu = Default::default();
                    }
                    input.editor_mut().key(key);
                    *selected = 0;
                    self.normalize_bar();
                    self.message.clear();
                }
            }
            self.refresh_option_candidates();
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
                        self.copy_selection(),
                        root,
                    )?;
                    return Ok(BarEvent::Handled);
                }
                KeyCode::Char('a') if matches!(self.mode, Mode::Management(_)) => {
                    if let Mode::Management(session) = &mut self.mode {
                        session.toggle_all();
                    }
                    return Ok(BarEvent::Handled);
                }
                KeyCode::Char('a') if matches!(self.mode, Mode::Files(_) | Mode::Changes(_)) => {
                    match &mut self.mode {
                        Mode::Files(files) => files.toggle_visible_group(),
                        Mode::Changes(session) if session.focus == crate::changes::Focus::Files => {
                            session.files.toggle_visible_group();
                        }
                        _ => {}
                    }
                    return Ok(BarEvent::Handled);
                }
                _ => {}
            }
        }
        if key.code == KeyCode::Char(' ')
            && self.bar.editor().text.is_empty()
            && (matches!(
                self.mode,
                Mode::Management(_) | Mode::Files(_) | Mode::Changes(_)
            ) || matches!(&self.mode, Mode::History(view) if matches!(view.as_ref(), crate::history::View::Graph(session) if session.selection.active())))
        {
            return Ok(BarEvent::Pass);
        }
        // 空搜索栏把无修饰的左右键交给内容视图；有文字时仍移动输入光标。
        if self.bar.editor().text.is_empty()
            && (matches!(
                self.mode,
                Mode::Management(_)
                    | Mode::History(_)
                    | Mode::Browser(_)
                    | Mode::Panel(crate::workspace::Section::Overview)
                    | Mode::Stashes(_)
                    | Mode::Branches(_)
                    | Mode::Changes(_)
                    | Mode::Files(_)
            ) || (matches!(self.mode, Mode::Browsing) && self.page == Page::Detail))
            && matches!(key.code, KeyCode::Home | KeyCode::End)
        {
            return Ok(BarEvent::Pass);
        }
        if self.bar.is_plain()
            && self.bar.editor().text.is_empty()
            && key.modifiers.is_empty()
            && matches!(key.code, KeyCode::Left | KeyCode::Right)
        {
            return Ok(BarEvent::Pass);
        }
        // 按 / 捕获当前视图和仓库，独立保存命令草稿，保留已有搜索。
        if key.code == KeyCode::Char('/') && key.modifiers.is_empty() && self.at_command_entry() {
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
        if editing && let InputBar::Search(editor) = &mut self.bar {
            editor.key(key);
            self.normalize_bar();
            self.message.clear();
            return Ok(BarEvent::Handled);
        }
        if editing
            && matches!(
                key.code,
                KeyCode::Char(_) | KeyCode::Backspace | KeyCode::Delete
            )
        {
            return Ok(BarEvent::Handled);
        }
        Ok(BarEvent::Pass)
    }
    pub(crate) fn open_command_bar(&mut self) {
        let path = self.command_repository();
        self.bar = InputBar::Command {
            input: command::Input::new(self.command_context()),
            selected: 0,
            menu: Default::default(),
            repository: command::repository::Captured::new(path.clone()),
            path,
            selection: self.copy_selection(),
        };
    }
    pub(crate) fn clear_input_bar(&mut self) {
        // 任务页没有可见输入；保留它覆盖的页面草稿和独立的多行编辑器。
        if self.tasks.session.open() {
            return;
        }
        if self.search_support() == crate::search::Support::Unavailable {
            self.bar = InputBar::CommandsOnly;
        } else {
            let path = self.selected_path();
            self.bar = InputBar::default();
            self.normalize_bar();
            if self.page == Page::Repositories && matches!(self.mode, Mode::Browsing) {
                self.focus(path);
            }
        }
        self.message.clear();
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
            Mode::Browser(session) => session.search_input(String::new()),
            Mode::Panel(_) => {}
            Mode::Stashes(view) => view.search(String::new()),
            Mode::Branches(view) => view.search(String::new()),
            Mode::History(view) => view.clear_search(),
            Mode::Files(s) => s.search(String::new()),
            Mode::Changes(session) => {
                session.files.search(String::new());
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
