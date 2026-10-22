//! Interaction state transitions.
use super::super::*;
use super::{ctrl_c, scroll_key};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
impl Screen {
    pub(crate) fn paste(&mut self, text: &str) {
        match &mut self.mode {
            Mode::CommitMessage { editor, .. } => editor.insert(text),
            Mode::Browsing
            | Mode::Panel(_)
            | Mode::Files(_)
            | Mode::Diff { .. }
            | Mode::History(_) => self.bar_paste(text),
            _ => {}
        }
    }
    pub(crate) fn key(&mut self, key: KeyEvent, root: &Cancel) -> Result<bool> {
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
        self.dispatch_key(key, root)
    }
    pub(super) fn dispatch_key(&mut self, key: KeyEvent, root: &Cancel) -> Result<bool> {
        if let Mode::Busy(p) = &mut self.mode {
            if key.code == KeyCode::Esc || ctrl_c(key) {
                p.cancel.cancel();
                p.state = TaskState::Cancelling;
            }
            return Ok(false);
        }
        if ctrl_c(key) {
            return Ok(true);
        }
        if self.repository_pane_key(key) {
            return Ok(false);
        }
        if let Mode::History(view) = &mut self.mode
            && matches!(self.bar, crate::bar::InputBar::Search(_))
            && key.modifiers.contains(KeyModifiers::ALT)
            && let history::View::Graph(session) = view.as_mut()
        {
            match key.code {
                KeyCode::Left => {
                    session.pan = session.pan.saturating_sub(2);
                    return Ok(false);
                }
                KeyCode::Right => {
                    session.pan = (session.pan + 2).min(session.width.saturating_sub(1));
                    return Ok(false);
                }
                _ => {}
            }
        }
        match self.bar_key(key, root)? {
            crate::bar::BarEvent::Handled => return Ok(false),
            crate::bar::BarEvent::Quit => return Ok(true),
            crate::bar::BarEvent::Pass => {}
        }
        let mode = self.take_mode();
        match mode {
            Mode::Browsing => return self.navigate(key, root),
            Mode::Panel(_) => match key.code {
                KeyCode::Esc | KeyCode::Left => {
                    self.mode = Mode::Browsing;
                    self.sync_bar();
                }
                KeyCode::Enter => {
                    self.mode = mode;
                    self.refresh_workspace();
                }
                _ => self.mode = mode,
            },
            Mode::History(mut view) => {
                match view.as_mut() {
                    history::View::Graph(session) => match key.code {
                        KeyCode::Esc | KeyCode::Left => {
                            if self.page == Page::Detail {
                                self.workspace.save(Mode::History(view));
                            }
                            self.sync_bar();
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
                        KeyCode::Esc | KeyCode::Left => {
                            self.mode =
                                Mode::History(Box::new(history::View::Graph(commit.graph.clone())));
                            self.sync_bar();
                            return Ok(false);
                        }
                        KeyCode::Up => commit.advance(-1),
                        KeyCode::Down => commit.advance(1),
                        KeyCode::PageUp => commit.scroll = commit.scroll.saturating_sub(10),
                        KeyCode::PageDown => commit.scroll = commit.scroll.saturating_add(10),
                        KeyCode::Tab => {
                            self.mode = Mode::History(view);
                            self.cycle_history_parent();
                            return Ok(false);
                        }
                        KeyCode::Enter | KeyCode::Right => {
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
                operation,
                mut choice,
            } => match choice.input(key.code) {
                confirmation::Input::Execute => self.confirm(operation),
                confirmation::Input::Return => {
                    self.mode = match operation {
                        Confirmation::Configure { origin, draft, .. } => {
                            self.mode = *origin;
                            self.bar = crate::bar::InputBar::Command {
                                editor: draft.editor,
                                path: draft.path,
                                selected: 0,
                                menu: Default::default(),
                            };
                            self.message.clear();
                            return Ok(false);
                        }
                        Confirmation::Schedule(_) | Confirmation::Remove(_) => Mode::Browsing,
                    };
                    self.message.clear();
                    self.sync_bar();
                }
                confirmation::Input::Pending => self.mode = Mode::Confirm { operation, choice },
            },
            Mode::Files(mut session) => {
                let count = session.rows().len();
                match key.code {
                    KeyCode::Esc | KeyCode::Left => {
                        self.message.clear();
                        if self.page == Page::Detail {
                            self.workspace.save(Mode::Files(session));
                        }
                        self.refresh_overview();
                        self.sync_bar();
                        return Ok(false);
                    }
                    KeyCode::Tab => {
                        session.area = if session.area == FileArea::Worktree {
                            FileArea::Index
                        } else {
                            FileArea::Worktree
                        };
                        session.cursor = 0;
                        session.selected.clear();
                    }
                    KeyCode::Up => session.cursor = session.cursor.saturating_sub(1),
                    KeyCode::Down => {
                        session.cursor = (session.cursor + 1).min(count.saturating_sub(1))
                    }
                    KeyCode::PageDown => {
                        session.cursor = (session.cursor + 10).min(count.saturating_sub(1))
                    }
                    KeyCode::PageUp => session.cursor = session.cursor.saturating_sub(10),
                    KeyCode::Char(' ') | KeyCode::Insert if count > 0 => {
                        let index = session.change_index(session.rows()[session.cursor]);
                        if !session.selected.remove(&index) {
                            session.selected.insert(index);
                        }
                    }
                    KeyCode::Enter | KeyCode::Right => {
                        self.mode = Mode::Files(session);
                        self.show_diff();
                        return Ok(false);
                    }
                    _ => {}
                }
                self.mode = Mode::Files(session);
            }
            Mode::Diff {
                session,
                content,
                mut scroll,
            } => {
                match key.code {
                    KeyCode::Esc | KeyCode::Left => {
                        self.mode = Mode::Files(session);
                        self.sync_bar();
                        return Ok(false);
                    }
                    _ => scroll_key(&mut scroll, key),
                }
                self.mode = Mode::Diff {
                    session,
                    content,
                    scroll,
                };
            }
            Mode::CommitMessage {
                plan,
                mut editor,
                mut scroll,
            } => {
                match key.code {
                    KeyCode::Esc => {
                        self.sync_bar();
                        return Ok(false);
                    }
                    KeyCode::PageDown => scroll = scroll.saturating_add(10),
                    KeyCode::PageUp => scroll = scroll.saturating_sub(10),
                    KeyCode::Enter => {
                        if editor.text.trim().is_empty() {
                            self.message = i18n::tr!(EnterACommitMessage).into()
                        } else {
                            self.mode = Mode::CommitConfirm {
                                choice: Default::default(),
                                plan,
                                message: editor.text,
                                scroll: 0,
                            };
                            self.message.clear();
                            return Ok(false);
                        }
                    }
                    _ => editor.key(key),
                }
                self.mode = Mode::CommitMessage {
                    plan,
                    editor,
                    scroll,
                };
            }
            Mode::CommitConfirm {
                mut choice,
                plan,
                message,
                mut scroll,
            } => {
                match choice.input(key.code) {
                    confirmation::Input::Execute => {
                        self.start(
                            i18n::tr!(CommitStagedContents),
                            Effect::Commit,
                            move |service, cancel, _| {
                                Ok(Outcome::Committed(
                                    service.commit(&plan, &message, &cancel)?,
                                ))
                            },
                        );
                        return Ok(false);
                    }
                    confirmation::Input::Return => {
                        self.mode = Mode::CommitMessage {
                            plan,
                            editor: Editor::new(message),
                            scroll,
                        };
                        self.message.clear();
                        self.sync_bar();
                        return Ok(false);
                    }
                    confirmation::Input::Pending => scroll_key(&mut scroll, key),
                }
                self.mode = Mode::CommitConfirm {
                    choice,
                    plan,
                    message,
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
                        self.start(
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
        }
        Ok(false)
    }
}
