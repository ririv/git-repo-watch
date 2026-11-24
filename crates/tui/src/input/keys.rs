//! Interaction state transitions.
use super::super::*;
use super::{ctrl_c, scroll_key};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use engine::service::ConfigEdit;
impl Screen {
    pub(crate) fn paste(&mut self, text: &str) {
        if !text.is_empty() {
            self.begin_temporary_input();
        }
        self.paste_inner(text);
        self.restore_temporary_input();
    }
    fn paste_inner(&mut self, text: &str) {
        self.exit_sequence.reset();
        if self.tasks.session.open() {
            self.bar_paste(text);
            return;
        }
        if self.focus_mode == crate::focus::State::Selecting {
            return;
        }
        self.clicks.reset();
        self.drag = None;
        self.close_management_actions();
        if self.history_actions().is_some() {
            let command = self.command_trigger_text(text).starts_with('/');
            self.close_history_actions();
            if command {
                self.open_command_bar();
            }
        }

        if self.command_trigger_text(text).starts_with('/') {
            if self.close_replay_controls() {
                self.open_command_bar();
            }
        } else {
            self.resume_replay_search();
        }
        match &mut self.mode {
            Mode::StashSave(session) => {
                session.message.insert(text);
            }
            Mode::PushTarget(session) => session.insert(text),
            Mode::Composition(session) => match &mut session.state {
                crate::composition::State::Editing(editor) => {
                    self.message = crate::message::Message::error(
                        editor
                            .insert(text)
                            .err()
                            .map(|_| i18n::tr!(AgentMessageInvalid).to_owned())
                            .unwrap_or_default(),
                    );
                }
                crate::composition::State::Browsing => self.bar_paste(text),
                _ => {}
            },
            Mode::PromptEditor(session) => {
                self.message = match session.insert(text) {
                    Ok(()) => crate::message::Message::information(String::new()),
                    Err(error) => crate::message::Message::error(format!("{error:#}")),
                };
            }
            Mode::CommitMessage { editor, .. } if self.bar.is_plain() => {
                if let Some(editor) = editor.editable()
                    && editor.insert(text).is_err()
                {
                    self.message =
                        crate::message::Message::error(i18n::tr!(AgentMessageInvalid).into());
                }
            }
            Mode::CommitMessage { .. } => {}
            Mode::Management(_) => self.bar_paste(text),
            Mode::Browsing
            | Mode::RepositoryEnvironment(_)
            | Mode::CompositionWaiting(_)
            | Mode::Browser(_)
            | Mode::Panel(_)
            | Mode::Stashes(_)
            | Mode::Branches(_)
            | Mode::Worktrees(_)
            | Mode::Submodules(_)
            | Mode::Files(_)
            | Mode::Changes(_)
            | Mode::History(_) => self.bar_paste(text),
            Mode::Error { .. }
                if matches!(self.bar, crate::bar::InputBar::Command { .. })
                    || text.starts_with('/') =>
            {
                self.bar_paste(text)
            }
            _ => {}
        }
    }
    pub(crate) fn key(&mut self, key: KeyEvent, root: &Cancel) -> Result<bool> {
        if key.code == KeyCode::Char(' ')
            && key.modifiers.is_empty()
            && self.space_behavior() == super::space::Space::Ignore
        {
            return Ok(false);
        }
        let typing = matches!(key.code, KeyCode::Char(c) if c != ' ' || matches!(self.mode, Mode::CommitMessage { .. } | Mode::StashSave(_)))
            && !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
            && key.kind != crossterm::event::KeyEventKind::Release;
        if typing {
            self.begin_temporary_input();
        }
        let result = self.key_inner(key, root);
        self.restore_temporary_input();
        result
    }
    fn key_inner(&mut self, key: KeyEvent, root: &Cancel) -> Result<bool> {
        if key.kind == crossterm::event::KeyEventKind::Release {
            return Ok(false);
        }
        self.clicks.reset();
        self.drag = None;
        if ctrl_c(key) {
            self.view_revision += 1;
            if !self.tasks.session.open()
                && (self.close_history_actions() || self.dismiss_input_actions())
            {
                return Ok(false);
            }
            return Ok(self.interrupt_key(key));
        }
        self.exit_sequence.reset();
        if self.interaction_key(key) {
            return Ok(false);
        }
        if self.tasks.session.open() {
            return self.tasks_key(key, root);
        }
        if self.history_actions_key(key) {
            return Ok(false);
        }
        #[cfg(feature = "developer-scheduler")]
        if matches!(self.mode, Mode::Workflow(_)) {
            let Mode::Workflow(mut session) = self.take_mode() else {
                unreachable!()
            };
            let result = session.key(key.code);
            self.workflow_input(session, result);
            return Ok(false);
        }
        if matches!(
            key.code,
            KeyCode::Esc
                | KeyCode::Enter
                | KeyCode::Left
                | KeyCode::Right
                | KeyCode::Tab
                | KeyCode::BackTab
                | KeyCode::Up
                | KeyCode::Down
                | KeyCode::Home
                | KeyCode::End
                | KeyCode::PageUp
                | KeyCode::PageDown
        ) || key.modifiers.contains(KeyModifiers::CONTROL)
        {
            self.view_revision += 1;
        }
        if matches!(
            key.code,
            KeyCode::Up
                | KeyCode::Down
                | KeyCode::PageUp
                | KeyCode::PageDown
                | KeyCode::Home
                | KeyCode::End
        ) {
            self.scroll_input.keyboard(std::time::Instant::now());
        }
        if self.management_controls_key(key) {
            self.sync_plain_bar();
            return Ok(false);
        }
        if self.replay_controls_key(key) {
            self.sync_plain_bar();
            return Ok(false);
        }
        if self.focus_mode == crate::focus::State::Inactive && self.locate_key(key) {
            return Ok(false);
        }
        let result = self.dispatch_key(key, root);
        self.sync_plain_bar();
        result
    }
    pub(super) fn dispatch_key(&mut self, key: KeyEvent, root: &Cancel) -> Result<bool> {
        if let Mode::Busy(p) = &mut self.mode {
            if key.code == KeyCode::Esc {
                p.cancel.cancel();
                p.state = TaskState::Cancelling;
            }
            return Ok(false);
        }
        if self.config.ai.enabled
            && matches!(self.mode, Mode::CommitMessage { .. })
            && key.code == KeyCode::Char('r')
            && key.modifiers == KeyModifiers::CONTROL
        {
            self.generate_commit_message();
            return Ok(false);
        }
        if matches!(self.mode, Mode::StashSave(_)) {
            let Mode::StashSave(session) = self.take_mode() else {
                unreachable!()
            };
            self.stash_save_key(session, key);
            return Ok(false);
        }
        if self.focus_key(key) {
            return Ok(false);
        }
        if self.horizontal_key(key) {
            return Ok(false);
        }
        match self.bar_key(key, root)? {
            crate::bar::BarEvent::Handled => return Ok(false),
            crate::bar::BarEvent::Quit => return Ok(true),
            crate::bar::BarEvent::Pass => {}
        }
        let text_editor = matches!(self.mode, Mode::PromptEditor(_))
            || matches!(&self.mode, Mode::Composition(session) if matches!(session.state, crate::composition::State::Editing(_)));
        if key.code == KeyCode::Tab && !text_editor {
            return Ok(false);
        }
        if key.code == KeyCode::BackTab
            && matches!(
                self.mode,
                Mode::Browsing
                    | Mode::Panel(_)
                    | Mode::Browser(_)
                    | Mode::Files(_)
                    | Mode::Changes(_)
                    | Mode::Branches(_)
                    | Mode::Worktrees(_)
                    | Mode::Submodules(_)
                    | Mode::Stashes(_)
                    | Mode::Composition(_)
            )
        {
            return Ok(false);
        }
        let mode = self.take_mode();
        match mode {
            Mode::StashSave(session) => self.stash_save_key(session, key),
            Mode::Management(session) => self.management_key(session, key),
            Mode::PushTarget(session) => self.push_target_key(session, key),
            Mode::CompositionWaiting(waiting) => self.composition_waiting_key(waiting, key),
            Mode::Composition(session) => self.compose_key(session, key, root),
            Mode::PromptEditor(mut session) => {
                match (key.code, key.modifiers) {
                    (KeyCode::Esc, _) => {
                        self.sync_bar();
                        return Ok(false);
                    }
                    (KeyCode::Char('s'), KeyModifiers::CONTROL) => {
                        let prompt = session.prompt();
                        self.mode = Mode::PromptEditor(session);
                        if let Err(error) = prompt.validate() {
                            self.message = crate::message::Message::error(format!("{error:#}"));
                        } else {
                            self.start(
                                i18n::tr!(AgentPrompt),
                                Effect::Configuration,
                                move |service, cancel, _| {
                                    service.configure_with_cancel(
                                        ConfigEdit::AiPrompt(prompt),
                                        &cancel,
                                    )?;
                                    Ok(Outcome::Text(i18n::tr!(AgentSaved).into()))
                                },
                            );
                        }
                        return Ok(false);
                    }
                    _ => {
                        self.message = match session.key(key) {
                            Ok(()) => crate::message::Message::information(String::new()),
                            Err(error) => crate::message::Message::error(format!("{error:#}")),
                        };
                    }
                }
                self.mode = Mode::PromptEditor(session);
            }
            Mode::Browser(session) => {
                self.browser_key(session, key);
            }
            Mode::Browsing => return self.navigate(key, root),
            Mode::RepositoryEnvironment(path) => {
                self.mode = Mode::RepositoryEnvironment(path);
                return self.navigate(key, root);
            }
            Mode::Panel(_) => match key.code {
                KeyCode::Esc | KeyCode::Left => {
                    self.mode = mode;
                    self.leave_repository_section();
                }
                _ if matches!(mode, Mode::Panel(crate::workspace::Section::Overview)) => {
                    self.mode = mode;
                    if key.code == KeyCode::F(1) {
                        self.open_page(Page::Help);
                    } else {
                        self.scroll_overview(key);
                    }
                }
                KeyCode::Enter => {
                    self.mode = mode;
                    self.refresh_workspace();
                }
                _ => self.mode = mode,
            },
            Mode::Stashes(view) => self.stash_key(view, key),
            Mode::Branches(view) => self.branch_key(view, key),
            Mode::Worktrees(session) => return self.worktree_key(session, key, root),
            Mode::Submodules(_) => {
                self.mode = mode;
                match key.code {
                    KeyCode::Esc | KeyCode::Left => self.return_to_view(),
                    _ => scroll_key(&mut self.scroll, key),
                }
            }
            Mode::History(mut view) => {
                match view.as_mut() {
                    history::View::Graph(session)
                        if session.focus == history::GraphFocus::Files =>
                    {
                        match key.code {
                            KeyCode::Up => self.history_preview.scroll(-1),
                            KeyCode::Down => self.history_preview.scroll(1),
                            KeyCode::PageUp => self.history_preview.scroll(-10),
                            KeyCode::PageDown => self.history_preview.scroll(10),
                            KeyCode::Home => self.history_preview.offset = 0,
                            KeyCode::End => {
                                self.history_preview.offset = self.history_preview.maximum
                            }
                            KeyCode::Esc | KeyCode::Left => {
                                session.focus = history::GraphFocus::Graph
                            }
                            KeyCode::Enter => {
                                self.mode = Mode::History(view);
                                self.open_history_commit();
                                return Ok(false);
                            }
                            _ => {}
                        }
                    }
                    history::View::Graph(session) => match key.code {
                        KeyCode::Char(' ') | KeyCode::Insert if session.selection.active() => {
                            if let Err(error) = session.mark(history::SelectionAction::ToggleCommit)
                            {
                                self.message = crate::message::Message::error(format!("{error:#}"));
                            }
                        }
                        KeyCode::Esc if session.selection.active() => {
                            session.selection.finish();
                        }
                        KeyCode::Esc | KeyCode::Left => {
                            if matches!(self.page, Page::View(_)) {
                                self.return_to_view();
                                return Ok(false);
                            }
                            if self.page == Page::Detail {
                                self.mode = Mode::History(view);
                                self.leave_repository_section();
                            } else {
                                self.sync_bar();
                            }
                            return Ok(false);
                        }
                        KeyCode::Up | KeyCode::Down
                            if session.selection.active()
                                && key.modifiers == KeyModifiers::SHIFT =>
                        {
                            if key.code == KeyCode::Down
                                && session.state.selected() == session.records.len().checked_sub(1)
                                && session.cursor().is_some()
                            {
                                self.mode = Mode::History(view);
                                self.more_history();
                                return Ok(false);
                            }
                            let delta = if key.code == KeyCode::Up { -1 } else { 1 };
                            if let Err(error) = session.extend_selection(delta) {
                                self.message = crate::message::Message::error(format!("{error:#}"));
                            }
                        }
                        KeyCode::Up => session.advance(-1),
                        KeyCode::Down
                            if session.state.selected() == session.records.len().checked_sub(1)
                                && session.cursor().is_some() =>
                        {
                            self.mode = Mode::History(view);
                            self.more_history();
                            return Ok(false);
                        }
                        KeyCode::Down => session.advance(1),
                        KeyCode::PageUp => session.advance(-10),
                        KeyCode::PageDown => session.advance(10),
                        KeyCode::Home => {
                            session.selection.settle();
                            session
                                .state
                                .select((!session.records.is_empty()).then_some(0));
                        }
                        KeyCode::End => {
                            session.selection.settle();
                            session.state.select(session.records.len().checked_sub(1));
                        }
                        KeyCode::BackTab
                            if session.selection.active() && !session.picks().is_empty() =>
                        {
                            self.mode = Mode::History(view);
                            self.open_history_actions();
                            return Ok(false);
                        }
                        KeyCode::Enter if session.selection.active() => {
                            self.mode = Mode::History(view);
                            self.open_history_actions();
                            return Ok(false);
                        }
                        KeyCode::Enter | KeyCode::Right => {
                            self.mode = Mode::History(view);
                            self.open_history_commit();
                            return Ok(false);
                        }
                        _ => {}
                    },
                    history::View::Commit(commit) => match commit.inspect.key(key) {
                        crate::inspection::Input::Back => {
                            self.mode =
                                Mode::History(Box::new(history::View::Graph(commit.graph.clone())));
                            self.sync_bar();
                            return Ok(false);
                        }
                        crate::inspection::Input::Open => {
                            self.mode = Mode::History(view);
                            self.show_history_diff();
                            return Ok(false);
                        }
                        crate::inspection::Input::Stay => {}
                    },
                    history::View::Diff { commit, scroll, .. } => match key.code {
                        KeyCode::Esc | KeyCode::Left => {
                            self.mode =
                                Mode::History(Box::new(history::View::Commit((**commit).clone())));
                            self.sync_bar();
                            return Ok(false);
                        }
                        _ => scroll_key(scroll, key),
                    },
                }
                self.mode = Mode::History(view);
            }
            Mode::Confirm {
                mut operation,
                mut choice,
            } => match match &mut operation {
                Confirmation::ReplayParents {
                    selected,
                    candidate,
                    ..
                } => match key.code {
                    KeyCode::Up => {
                        *selected = selected.saturating_sub(1);
                        confirmation::Input::Pending
                    }
                    KeyCode::Down => {
                        *selected = (*selected + 1).min(candidate.parents.len() - 1);
                        confirmation::Input::Pending
                    }
                    KeyCode::Enter => confirmation::Input::Execute,
                    KeyCode::Esc => confirmation::Input::Return,
                    _ => confirmation::Input::Pending,
                },
                Confirmation::DiscardFile { scroll, .. }
                | Confirmation::Mutation { scroll, .. } => {
                    scroll_key(scroll, key);
                    choice.input(key.code)
                }
                _ => choice.input(key.code),
            } {
                confirmation::Input::Execute => self.confirm(operation),
                confirmation::Input::Return => {
                    self.mode = match operation {
                        Confirmation::Configure { origin, draft, .. }
                        | Confirmation::Mutation { origin, draft, .. }
                        | Confirmation::ReplayParents { origin, draft, .. } => {
                            self.mode = *origin;
                            match draft.return_to {
                                crate::bar::ReturnTo::View => self.sync_bar(),
                                crate::bar::ReturnTo::Command => {
                                    self.bar = crate::bar::InputBar::Command {
                                        input: draft.input,
                                        path: draft.path,
                                        selection: Box::new(draft.selection),
                                        repository: draft.repository,
                                        selected: 0,
                                        menu: Default::default(),
                                    };
                                }
                            }
                            self.message.clear();
                            return Ok(false);
                        }
                        Confirmation::ConflictReset { origin, .. }
                        | Confirmation::DiscardFile { origin, .. }
                        | Confirmation::Partial { origin, .. }
                        | Confirmation::Remove { origin, .. } => *origin,
                        #[cfg(feature = "developer-scheduler")]
                        Confirmation::Schedule(_) => Mode::Browsing,
                    };
                    self.message.clear();
                    if matches!(self.page, Page::View(_)) && matches!(self.mode, Mode::Browsing) {
                        self.return_to_view();
                    }
                    self.sync_bar();
                }
                confirmation::Input::Pending => self.mode = Mode::Confirm { operation, choice },
            },
            Mode::Files(mut session) => {
                match key.code {
                    KeyCode::Esc if session.selected.active() => session.selected.finish(),
                    KeyCode::Esc | KeyCode::Left => {
                        self.message.clear();
                        if self.page == Page::Detail {
                            self.mode = Mode::Files(session);
                            self.leave_repository_section();
                        } else {
                            self.refresh_overview();
                            self.sync_bar();
                        }
                        return Ok(false);
                    }
                    KeyCode::Up => session.move_cursor(-1, self.file_layout),
                    KeyCode::Down => session.move_cursor(1, self.file_layout),
                    KeyCode::Home => session.edge(false, self.file_layout),
                    KeyCode::End => session.edge(true, self.file_layout),
                    KeyCode::PageDown => session.move_cursor(10, self.file_layout),
                    KeyCode::PageUp => session.move_cursor(-10, self.file_layout),
                    KeyCode::Char(' ') | KeyCode::Insert => session.toggle_current(),
                    KeyCode::Enter if session.toggle_directory(self.file_layout) => {}
                    KeyCode::Enter | KeyCode::Right if session.current().is_some() => {
                        self.mode = Mode::Files(session);
                        self.show_diff();
                        return Ok(false);
                    }
                    _ => {}
                }
                self.mode = Mode::Files(session);
            }
            Mode::Changes(session) => self.changes_key(session, key),
            Mode::CommitMessage {
                plan,
                mut editor,
                mut scroll,
            } => {
                match key.code {
                    KeyCode::Esc if editor.generating() => self.commit_generator.cancel(),
                    KeyCode::Esc if !editor.focused() => {
                        editor.focus(crate::commit_form::Focus::Input)
                    }
                    KeyCode::Esc => {
                        if matches!(self.page, Page::View(_)) {
                            self.return_to_view();
                        }
                        self.sync_bar();
                        return Ok(false);
                    }
                    KeyCode::PageDown => scroll = scroll.saturating_add(10),
                    KeyCode::PageUp => scroll = scroll.saturating_sub(10),
                    KeyCode::BackTab => editor.toggle_mode(),
                    KeyCode::Left | KeyCode::Right if !editor.focused() => {
                        editor.step(key.code == KeyCode::Left, self.config.ai.enabled);
                    }
                    KeyCode::Up if !editor.focused() => {
                        editor.focus(crate::commit_form::Focus::Input);
                    }
                    KeyCode::Enter if !editor.focused() => {
                        let action = editor.selected().expect("focused commit action");
                        self.mode = Mode::CommitMessage {
                            plan,
                            editor,
                            scroll,
                        };
                        self.commit_form_action(action);
                        return Ok(false);
                    }
                    KeyCode::Char('s') if key.modifiers == KeyModifiers::CONTROL => {
                        self.mode = Mode::CommitMessage {
                            plan,
                            editor,
                            scroll,
                        };
                        self.submit_commit();
                        return Ok(false);
                    }
                    KeyCode::Char(_)
                        if !editor.focused()
                            && !key
                                .modifiers
                                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                    {
                        if let Some(editor) = editor.editable()
                            && editor.key(key).is_err()
                        {
                            self.message = crate::message::Message::error(
                                i18n::tr!(AgentMessageInvalid).into(),
                            );
                        }
                    }
                    _ if editor.focused() => {
                        if let Some(editor) = editor.editable()
                            && editor.key(key).is_err()
                        {
                            self.message = crate::message::Message::error(
                                i18n::tr!(AgentMessageInvalid).into(),
                            );
                        }
                    }
                    _ => {}
                }
                self.mode = Mode::CommitMessage {
                    plan,
                    editor,
                    scroll,
                };
            }
            Mode::PushConfirm {
                plan,
                mut scroll,
                mut choice,
            } => {
                match choice.input(key.code) {
                    confirmation::Input::Execute => {
                        self.execute_push(plan);
                        return Ok(false);
                    }
                    confirmation::Input::Return => {
                        self.message =
                            crate::message::Message::information(i18n::tr!(Cancelled).into());
                        if matches!(self.page, Page::View(_)) {
                            self.return_to_view();
                        }
                        self.sync_bar();
                        return Ok(false);
                    }
                    confirmation::Input::Pending => scroll_key(&mut scroll, key),
                }
                self.mode = Mode::PushConfirm {
                    plan,
                    scroll,
                    choice,
                };
            }
            Mode::Error {
                origin,
                title,
                detail,
                mut scroll,
            } => {
                match key.code {
                    KeyCode::Esc | KeyCode::Enter => {
                        self.mode = *origin;
                        if matches!(self.page, Page::View(_)) && matches!(self.mode, Mode::Browsing)
                        {
                            self.return_to_view();
                        }
                        self.sync_bar();
                        return Ok(false);
                    }

                    _ => scroll_key(&mut scroll, key),
                }
                self.mode = Mode::Error {
                    origin,
                    title,
                    detail,
                    scroll,
                };
            }
            Mode::Busy(_) => unreachable!(),
            #[cfg(feature = "developer-scheduler")]
            Mode::Workflow(_) => unreachable!(),
        }
        Ok(false)
    }
}
