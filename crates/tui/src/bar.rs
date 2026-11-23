//! Persistent input: plain text filters the current list; slash-prefixed text selects commands.
mod complete;
mod history;
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
    Command {
        input: command::Input,
        selected: usize,
        menu: command::completion::Menu,
        path: Option<PathBuf>,
        selection: Box<crate::clipboard::Selection>,
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
        }
    }
    pub fn text(&self) -> String {
        match self {
            Self::Command { input, .. } => input.text(),
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
    pub(crate) fn active_bar(&self) -> &InputBar {
        if self.tasks.session.open() {
            &self.tasks.bar
        } else {
            &self.bar
        }
    }
    pub(super) fn active_bar_mut(&mut self) -> &mut InputBar {
        self.tasks.input_bar_mut(&mut self.bar)
    }
    pub(crate) fn sync_task_bar(&mut self) {
        use crate::task_center::Session;
        self.tasks.bar = match self.tasks.session {
            Session::List => InputBar::Search(self.tasks.search.clone()),
            Session::Detail { .. } | Session::Closed => InputBar::CommandsOnly,
        };
    }
    fn sync_active_bar(&mut self) {
        if self.tasks.session.open() {
            self.sync_task_bar();
        } else {
            self.sync_bar();
        }
    }

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
        let query = self
            .search_query_in(&self.mode)
            .expect("search capability requires a visible searchable list")
            .to_owned();
        self.bar = InputBar::Search(Editor::new(query));
    }
    pub(super) fn normalize_bar(&mut self) {
        self.promote_common_command();
        if let InputBar::Command {
            input: command::Input::Global(editor),
            ..
        } = self.active_bar()
            && !editor.text.starts_with('/')
        {
            // 删除整个命令草稿等同于关闭候选，恢复当前视图保存的搜索。
            if editor.text.is_empty() {
                self.clear_bar();
                return;
            }
            let InputBar::Command { input, .. } = std::mem::take(self.active_bar_mut()) else {
                unreachable!()
            };
            *self.active_bar_mut() =
                if self.active_search_support() == crate::search::Support::Unavailable {
                    InputBar::CommandsOnly
                } else {
                    InputBar::Search(input.into_search().expect("global command lost slash"))
                };
        }
        if self.tasks.session.open() {
            if let InputBar::Search(editor) = &self.tasks.bar {
                self.tasks.update_search(editor.clone());
            }
            return;
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
                Mode::Browsing => self.search_repositories(query),
                _ => {}
            }
        }
    }
    pub(crate) fn bar_paste(&mut self, text: &str) {
        let text = self.command_trigger_text(text);
        let text = text.as_ref();
        if self.active_bar().is_plain()
            && (self.at_command_entry() || text.starts_with("//"))
            && let Some(body) = text.strip_prefix('/')
        {
            self.open_command_bar();
            let InputBar::Command { input, .. } = self.tasks.input_bar_mut(&mut self.bar) else {
                unreachable!()
            };
            input.paste_initial(body);
        } else if let InputBar::Command { input, .. } = self.tasks.input_bar_mut(&mut self.bar) {
            input.paste(text);
        } else if let InputBar::Search(editor) = self.tasks.input_bar_mut(&mut self.bar) {
            editor.insert(text);
        }
        self.normalize_bar();
        if let InputBar::Command {
            selected,
            menu,
            input,
            ..
        } = self.tasks.input_bar_mut(&mut self.bar)
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
        if self.command_history_key(key) {
            return Ok(BarEvent::Handled);
        }
        let mut submitted = None;
        let result = self.bar_key_inner(key, root, &mut submitted);
        if let Some(text) = submitted
            && let Err(error) = self.service.store.record_command(&text)
        {
            self.message = crate::message::Message::error(i18n::tf!(
                SaveCommandHistoryFailed,
                format!("{error:#}")
            ));
        }
        result
    }
    fn bar_key_inner(
        &mut self,
        key: KeyEvent,
        root: &Cancel,
        submitted: &mut Option<String>,
    ) -> Result<BarEvent> {
        let key = self.command_trigger_key(key);
        // 错误页开放复制、任务与退出；Esc 先关闭命令，再交给错误页返回。
        if !self.tasks.session.open()
            && matches!(self.mode, Mode::Error { .. })
            && self.active_bar().is_plain()
        {
            if key.code == KeyCode::Char('/') && key.modifiers.is_empty() {
                self.open_command_bar();
                return Ok(BarEvent::Handled);
            }
            return Ok(BarEvent::Pass);
        }
        if !self.tasks.session.open()
            && matches!(&self.mode, Mode::Composition(session) if !matches!(session.state, crate::composition::State::Browsing))
        {
            return Ok(BarEvent::Pass);
        }
        if !self.tasks.session.open()
            && !matches!(
                self.mode,
                Mode::Management(_)
                    | Mode::Browsing
                    | Mode::RepositoryEnvironment(_)
                    | Mode::Composition(_)
                    | Mode::CompositionWaiting(_)
                    | Mode::Browser(_)
                    | Mode::Panel(_)
                    | Mode::Stashes(_)
                    | Mode::Branches(_)
                    | Mode::Worktrees(_)
                    | Mode::Submodules(_)
                    | Mode::Files(_)
                    | Mode::Changes(_)
                    | Mode::History(_)
                    | Mode::Error { .. }
            )
        {
            return Ok(BarEvent::Pass);
        }
        if self.command_page_key(key) {
            return Ok(BarEvent::Handled);
        }
        if key.code == KeyCode::Enter
            && matches!(self.active_bar(), InputBar::Command { input, menu: command::completion::Menu::Loading(command::loader::Request::Values { .. } | command::loader::Request::Editors { .. }), .. }
                if command::argument_input(&input.query()).is_some_and(|(argument, value)| !value.trim().is_empty() && argument.parse(value, self.service.scheduler.user_home()).is_ok()))
        {
            if let InputBar::Command { menu, .. } = self.tasks.input_bar_mut(&mut self.bar) {
                *menu = Default::default();
            }
            self.command_reader.clear();
        }
        if matches!(
            self.active_bar(),
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
                    if let InputBar::Command { menu, .. } = self.tasks.input_bar_mut(&mut self.bar)
                    {
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
        use command::named::Candidate;
        let command_menu = self.command_menu().unwrap_or_default();
        let candidates = command_menu.entries;
        let group = command_menu.group.as_ref();
        if key.code == KeyCode::Esc
            && let InputBar::Command {
                input,
                selected,
                menu: command::completion::Menu::Commands,
                ..
            } = self.tasks.input_bar_mut(&mut self.bar)
            && let Some(query) = command::named::Query::new(&input.query())
            && query.parameter_only()
        {
            input.replace_canonical(Editor::new(format!("/{} ", query.owner().name)));
            *selected = 0;
            self.message.clear();
            return Ok(BarEvent::Handled);
        }
        if key.code == KeyCode::Esc
            && let InputBar::Command {
                input,
                selected,
                menu: command::completion::Menu::Commands,
                ..
            } = self.tasks.input_bar_mut(&mut self.bar)
            && let Some(group) = group
            && group.location == command::hierarchy::Location::Children
        {
            input.set_group(group.parent, command::hierarchy::Location::Parent);
            *selected = 0;
            self.message.clear();
            return Ok(BarEvent::Handled);
        }
        if key.code == KeyCode::Enter
            && let InputBar::Command {
                input,
                path,
                selection,
                selected,
                ..
            } = self.active_bar()
            && !input.prefix_selected()
        {
            let query = input.query();
            let action = command::argument_input(&query)
                .map(|(argument, _)| argument.action())
                .or_else(|| self.out_of_view_command(&query, path.as_deref(), selection))
                .or_else(|| {
                    candidates
                        .get(*selected)
                        .map(|candidate| candidate.owner().action)
                });
            if let Some(action) = action
                && let Some(reason) = self
                    .command_availability(action, path.as_deref(), selection)
                    .reason()
            {
                self.message = crate::message::Message::error(reason.to_owned());
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
        } = self.tasks.input_bar_mut(&mut self.bar)
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
            let values_menu = matches!(menu, command::completion::Menu::Values(values) if values.kind != command::values::Kind::WorktreeBranch);
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
                                return self.bar_key_inner(key, root, submitted);
                            }
                        } else {
                            let reason = candidates
                                .unavailable()
                                .unwrap_or(i18n::tr!(NoMatchingOptions));
                            self.message = crate::message::Message::error(reason.into());
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
                KeyCode::Down => {
                    *selected = (*selected + 1).min(candidates.len().saturating_sub(1))
                }
                KeyCode::Tab => {
                    if let Some(Candidate::Parameter(parameter)) = candidates.get(*selected) {
                        parameter.complete(input);
                        *selected = 0;
                        self.message.clear();
                    } else if let Some(Candidate::Command(c)) = candidates.get(*selected) {
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
                            if group.is_some_and(|group| group.at_parent(c)) {
                                input.set_group(c, command::hierarchy::Location::Children);
                            } else {
                                selection.complete(input, c);
                            }
                            self.message.clear();
                        }
                        *selected = 0
                    }
                }
                KeyCode::Enter => {
                    if !selection.is_error()
                        && let Some((argument, value)) = command::argument_input(&query)
                    {
                        if let command::ArgumentAction::Option(kind) = argument {
                            let action = kind.parse(value).map(|value| value.action());
                            let target = path.clone();
                            let selection = selection.as_ref().clone();
                            match action {
                                Ok(action) => {
                                    *submitted = Some(input.text());
                                    self.sync_active_bar();
                                    self.activate(action, target, selection, root)?;
                                }
                                Err(error) => {
                                    self.message =
                                        crate::message::Message::error(format!("{error:#}"))
                                }
                            }
                            return Ok(BarEvent::Handled);
                        }
                        if argument.action() == Action::AiPrompt && value.trim().is_empty() {
                            *submitted = Some(input.text());
                            self.activate(
                                Action::AiPrompt,
                                None,
                                crate::clipboard::Selection::None,
                                root,
                            )?;
                            return Ok(BarEvent::Handled);
                        }
                        if argument == command::ArgumentAction::Scan {
                            let text = input.text();
                            let value = value.to_owned();
                            self.close_tasks();
                            if self.scan_command(&value) {
                                *submitted = Some(text);
                            }
                            return Ok(BarEvent::Handled);
                        }
                        let parsed = argument.parse(value, self.service.scheduler.user_home());
                        if parsed.is_ok()
                            && !matches!(
                                argument,
                                command::ArgumentAction::RepositoryEnvironment(
                                    command::EnvironmentField::Set
                                )
                            )
                        {
                            *submitted = Some(input.text());
                        }
                        match parsed {
                            Ok(value)
                                if matches!(
                                    argument,
                                    command::ArgumentAction::WorktreeAdd
                                        | command::ArgumentAction::SubmoduleAdd
                                        | command::ArgumentAction::SubmoduleCheckout(_),
                                ) =>
                            {
                                let draft = CommandDraft {
                                    input: input.clone(),
                                    path: path.clone(),
                                    selection: selection.as_ref().clone(),
                                    repository: repository.clone(),
                                    return_to: ReturnTo::Command,
                                };
                                if let Err(error) =
                                    self.prepare_repository_argument(argument, &value, draft)
                                {
                                    self.message =
                                        crate::message::Message::error(format!("{error:#}"));
                                }
                            }
                            Ok(value) if argument == command::ArgumentAction::Editor => {
                                let editor = value.try_into()?;
                                self.close_tasks();
                                self.select_editor(editor);
                            }
                            Ok(value) if argument == command::ArgumentAction::AiAgent => {
                                let agent = value.parse()?;
                                self.close_tasks();
                                self.select_agent(agent);
                            }
                            Ok(value) => {
                                let draft = CommandDraft {
                                    input: input.clone(),
                                    path: path.clone(),
                                    selection: selection.as_ref().clone(),
                                    repository: repository.clone(),
                                    return_to: ReturnTo::Command,
                                };
                                match argument.mutation(&value) {
                                    Ok(Some(request)) => {
                                        self.close_tasks();
                                        self.prepare_mutation(request, draft);
                                    }
                                    Ok(None) => {
                                        self.close_tasks();
                                        self.prepare_configuration(
                                            argument
                                                .configuration()
                                                .expect("configuration parameter"),
                                            value,
                                            draft,
                                        );
                                    }
                                    Err(error) => {
                                        self.message =
                                            crate::message::Message::error(format!("{error:#}"))
                                    }
                                }
                            }
                            Err(error) => {
                                self.message = crate::message::Message::error(format!("{error:#}"))
                            }
                        }
                    } else if let Some(Candidate::Parameter(parameter)) = candidates.get(*selected)
                    {
                        parameter.complete(input);
                        *selected = 0;
                        self.message.clear();
                    } else if let Some(Candidate::Command(command)) = candidates.get(*selected) {
                        if command.action.argument().is_some() && command.action != Action::AiPrompt
                        {
                            selection.complete(input, command);
                            *selected = 0;
                            self.message.clear();
                        } else {
                            *submitted = Some(format!("/{}", command.name));
                            let action = command.action;
                            let path = path.clone();
                            let selection = selection.as_ref().clone();
                            self.sync_active_bar();
                            self.message.clear();
                            if self.activate(action, path, selection, root)? {
                                return Ok(BarEvent::Quit);
                            }
                        }
                    } else {
                        self.message =
                            crate::message::Message::error(i18n::tr!(NoMatchingCommands).into());
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
        if key.code == KeyCode::Tab && key.modifiers.is_empty() && self.complete_filter() {
            return Ok(BarEvent::Handled);
        }
        if key.code == KeyCode::Esc && !self.active_bar().editor().text.is_empty() {
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
                KeyCode::Char('a')
                    if !self.tasks.session.open() && matches!(self.mode, Mode::Management(_)) =>
                {
                    if let Mode::Management(session) = &mut self.mode {
                        session.toggle_all();
                    }
                    return Ok(BarEvent::Handled);
                }
                KeyCode::Char('a')
                    if !self.tasks.session.open()
                        && matches!(self.mode, Mode::Files(_) | Mode::Changes(_)) =>
                {
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
            && self.space_behavior() == crate::input::space::Space::Content
        {
            return Ok(BarEvent::Pass);
        }
        // 空搜索栏把无修饰的左右键交给内容视图；有文字时仍移动输入光标。
        if self.active_bar().editor().text.is_empty()
            && (self.tasks.session.open()
                || matches!(
                    self.mode,
                    Mode::Management(_)
                        | Mode::History(_)
                        | Mode::Browser(_)
                        | Mode::Panel(crate::workspace::Section::Overview)
                        | Mode::Stashes(_)
                        | Mode::Branches(_)
                        | Mode::Changes(_)
                        | Mode::Files(_)
                )
                || (matches!(self.mode, Mode::Browsing) && self.page == Page::Detail))
            && matches!(key.code, KeyCode::Home | KeyCode::End)
        {
            return Ok(BarEvent::Pass);
        }
        if self.active_bar().is_plain()
            && self.active_bar().editor().text.is_empty()
            && key.modifiers.is_empty()
            && matches!(key.code, KeyCode::Left | KeyCode::Right)
        {
            return Ok(BarEvent::Pass);
        }
        // 搜索中的转义斜杠写入查询，直接按 / 则捕获当前视图和选中项。
        if key.code == KeyCode::Char('/') && key.modifiers.is_empty() {
            if let InputBar::Search(editor) = self.tasks.input_bar_mut(&mut self.bar)
                && editor.text[..editor.cursor].ends_with('\\')
            {
                editor
                    .text
                    .replace_range(editor.cursor - 1..editor.cursor, "/");
                self.normalize_bar();
            } else {
                self.open_command_bar();
            }
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
        if editing && let InputBar::Search(editor) = self.tasks.input_bar_mut(&mut self.bar) {
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
        *self.active_bar_mut() = InputBar::Command {
            input: command::Input::new(self.command_context()),
            selected: 0,
            menu: Default::default(),
            repository: command::repository::Captured::new(path.clone()),
            path,
            selection: Box::new(self.copy_selection()),
        };
    }
    pub(crate) fn clear_input_bar(&mut self) {
        if self.tasks.session.open() {
            self.clear_bar();
            return;
        }
        if self.search_support() == crate::search::Support::Unavailable {
            self.bar = InputBar::CommandsOnly;
        } else {
            self.bar = InputBar::default();
            self.normalize_bar();
        }
        self.message.clear();
    }
    fn clear_bar(&mut self) {
        if self.tasks.session.open() {
            if self.tasks.bar.is_plain() {
                self.tasks.update_search(Editor::default());
            }
            self.sync_task_bar();
            self.message.clear();
            return;
        }
        if matches!(self.bar, InputBar::Command { .. }) {
            self.sync_bar();
            self.message.clear();
            return;
        }
        if self.repository_list_active() {
            self.scroll = 0;
            self.clear_input_bar();
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
                self.selection_before_repository_filter = None;
                self.page = Page::Repositories;
                self.scroll = 0;
                self.focus(path)
            }
        }
        self.message.clear();
    }
}
