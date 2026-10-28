//! Interaction state transitions.
use super::super::*;
use super::{ctrl_c, scroll_key};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use engine::service::ConfigEdit;
impl Screen {
    pub(crate) fn paste(&mut self, text: &str) {
        self.exit_sequence.reset();
        if self.tasks.session.open() || self.focus_mode == crate::focus::State::Selecting {
            return;
        }
        self.clicks.reset();
        self.drag = None;
        match &mut self.mode {
            Mode::Composition(session) => match &mut session.state {
                crate::composition::State::Editing(editor) => {
                    self.message = editor
                        .insert(text)
                        .err()
                        .map(|_| i18n::tr!(AgentMessageInvalid).to_owned())
                        .unwrap_or_default();
                }
                crate::composition::State::Browsing => self.bar_paste(text),
                _ => {}
            },
            Mode::PromptEditor(session) => {
                self.message = match session.insert(text) {
                    Ok(()) => String::new(),
                    Err(error) => format!("{error:#}"),
                };
            }
            Mode::CommitMessage { editor, .. } if editor.focused() && self.bar.is_plain() => {
                if let Some(editor) = editor.editable()
                    && editor.insert(text).is_err()
                {
                    self.message = i18n::tr!(AgentMessageInvalid).into();
                }
            }
            Mode::CommitMessage { .. } => {}
            Mode::Browsing
            | Mode::CompositionWaiting(_)
            | Mode::Browser(_)
            | Mode::Panel(_)
            | Mode::Stashes(_)
            | Mode::Branches(_)
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
        if key.kind == crossterm::event::KeyEventKind::Release {
            return Ok(false);
        }
        self.clicks.reset();
        self.drag = None;
        if ctrl_c(key) {
            self.view_revision += 1;
            return Ok(self.interrupt_key(key));
        }
        self.exit_sequence.reset();
        if self.interaction_key(key) {
            return Ok(false);
        }
        if self.tasks_key(key) {
            return Ok(false);
        }
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
        let mode = self.take_mode();
        match mode {
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
                            self.message = format!("{error:#}");
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
                            Ok(()) => String::new(),
                            Err(error) => format!("{error:#}"),
                        };
                    }
                }
                self.mode = Mode::PromptEditor(session);
            }
            Mode::Browser(session) => {
                self.browser_key(session, key);
            }
            Mode::Browsing => return self.navigate(key, root),
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
            Mode::History(mut view) => {
                match view.as_mut() {
                    history::View::Graph(session) => match key.code {
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
                        KeyCode::Home => session
                            .state
                            .select((!session.records.is_empty()).then_some(0)),
                        KeyCode::End => session.state.select(session.records.len().checked_sub(1)),
                        KeyCode::Tab => session.next_match(false),
                        KeyCode::BackTab => session.next_match(true),
                        KeyCode::Enter | KeyCode::Right => {
                            self.mode = Mode::History(view);
                            self.open_history_commit();
                            return Ok(false);
                        }
                        _ => {}
                    },
                    history::View::Commit(commit) => match key.code {
                        KeyCode::Esc
                            if commit.arrangement == crate::workspace::Arrangement::Maximized =>
                        {
                            commit.arrangement = crate::workspace::Arrangement::Tiled;
                        }
                        KeyCode::Esc | KeyCode::Left => {
                            self.mode =
                                Mode::History(Box::new(history::View::Graph(commit.graph.clone())));
                            self.sync_bar();
                            return Ok(false);
                        }
                        KeyCode::Up | KeyCode::Down | KeyCode::PageUp | KeyCode::PageDown => {
                            let delta = match key.code {
                                KeyCode::Up => -1,
                                KeyCode::Down => 1,
                                KeyCode::PageUp => -10,
                                KeyCode::PageDown => 10,
                                _ => unreachable!(),
                            };
                            match commit.focus {
                                history::Focus::Files => commit.advance(delta),
                                history::Focus::Message => {
                                    commit.scroll = commit.scroll.saturating_add_signed(delta)
                                }
                                history::Focus::Preview => commit.review.advance(delta),
                            }
                        }
                        KeyCode::Home | KeyCode::End => match commit.focus {
                            history::Focus::Files => {
                                commit.files.select(if key.code == KeyCode::Home {
                                    (!commit.rows().is_empty()).then_some(0)
                                } else {
                                    commit.rows().len().checked_sub(1)
                                })
                            }
                            history::Focus::Message => {
                                commit.scroll = if key.code == KeyCode::Home {
                                    0
                                } else {
                                    usize::MAX
                                }
                            }
                            history::Focus::Preview => {
                                commit.review.seek(if key.code == KeyCode::Home {
                                    0
                                } else {
                                    usize::MAX
                                })
                            }
                        },
                        KeyCode::Tab => {
                            self.mode = Mode::History(view);
                            self.cycle_history_parent();
                            return Ok(false);
                        }
                        KeyCode::Enter | KeyCode::Right
                            if commit.focus != history::Focus::Message =>
                        {
                            commit.open_preview_file();
                            self.mode = Mode::History(view);
                            self.show_history_diff();
                            return Ok(false);
                        }
                        _ => {}
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
                _ => choice.input(key.code),
            } {
                confirmation::Input::Execute => self.confirm(operation),
                confirmation::Input::Return => {
                    self.mode = match operation {
                        Confirmation::Configure { origin, draft, .. }
                        | Confirmation::Mutation { origin, draft, .. }
                        | Confirmation::ReplayParents { origin, draft, .. } => {
                            self.mode = *origin;
                            self.bar = crate::bar::InputBar::Command {
                                input: draft.input,
                                path: draft.path,
                                selection: draft.selection,
                                selected: 0,
                                menu: Default::default(),
                            };
                            self.message.clear();
                            return Ok(false);
                        }
                        Confirmation::Schedule(_) | Confirmation::Remove(_) => Mode::Browsing,
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
                    KeyCode::Tab => session.switch_group(self.file_layout),
                    KeyCode::Up => session.move_cursor(-1, self.file_layout),
                    KeyCode::Down => session.move_cursor(1, self.file_layout),
                    KeyCode::Home => session.edge(false, self.file_layout),
                    KeyCode::End => session.edge(true, self.file_layout),
                    KeyCode::PageDown => session.move_cursor(10, self.file_layout),
                    KeyCode::PageUp => session.move_cursor(-10, self.file_layout),
                    KeyCode::Char(' ') | KeyCode::Insert => session.toggle_current(),
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
                    KeyCode::Esc => {
                        if matches!(self.page, Page::View(_)) {
                            self.return_to_view();
                        }
                        self.sync_bar();
                        return Ok(false);
                    }
                    KeyCode::PageDown => scroll = scroll.saturating_add(10),
                    KeyCode::PageUp => scroll = scroll.saturating_sub(10),
                    KeyCode::Tab | KeyCode::BackTab => {
                        editor.tab(key.code == KeyCode::BackTab, self.config.ai.enabled);
                    }
                    KeyCode::Left | KeyCode::Right if !editor.focused() => {
                        editor.step(key.code == KeyCode::Left, self.config.ai.enabled);
                    }
                    KeyCode::Up if !editor.focused() => {
                        editor.focus(crate::commit_form::Focus::Message);
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
                    _ if editor.focused() => {
                        if let Some(editor) = editor.editable()
                            && editor.key(key).is_err()
                        {
                            self.message = i18n::tr!(AgentMessageInvalid).into();
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
                        self.start_repository(
                            plan.path.clone(),
                            i18n::tr!(PushCurrentBranch),
                            Effect::Push,
                            move |service, cancel, _| {
                                Ok(Outcome::Published(service.push(&plan, &cancel)?))
                            },
                        );
                        return Ok(false);
                    }
                    confirmation::Input::Return => {
                        self.message = i18n::tr!(Cancelled).into();
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
            Mode::Busy(_) | Mode::Workflow(_) => unreachable!(),
        }
        Ok(false)
    }
}
