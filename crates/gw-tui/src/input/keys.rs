//! Interaction state transitions.
use super::super::*;
use super::{ctrl_c, scroll_key, validate_input};
use crossterm::event::{KeyCode, KeyEvent};
impl Screen {
    pub(crate) fn paste(&mut self, text: &str) {
        match &mut self.mode {
            Mode::Input { editor, .. } | Mode::CommitMessage { editor, .. } => editor.insert(text),
            Mode::Browsing | Mode::Files(_) | Mode::Diff { .. } => self.bar_paste(text),
            _ => {}
        }
    }
    pub(crate) fn key(&mut self, key: KeyEvent, root: &Cancel) -> Result<bool> {
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
        match self.bar_key(key, root)? {
            crate::bar::BarEvent::Handled => return Ok(false),
            crate::bar::BarEvent::Quit => return Ok(true),
            crate::bar::BarEvent::Pass => {}
        }
        let mode = self.take_mode();
        match mode {
            Mode::Browsing => return self.navigate(key, root),
            Mode::Input { action, mut editor } => {
                match key.code {
                    KeyCode::Esc => {
                        self.sync_bar();
                        return Ok(false);
                    }
                    KeyCode::Enter => match validate_input(action, &editor.text) {
                        Ok(()) => {
                            self.mode = Mode::Confirm(Confirmation::Configure {
                                action,
                                value: editor.text,
                            });
                            self.message.clear();
                            return Ok(false);
                        }
                        Err(e) => self.message = format!("{e:#}"),
                    },
                    _ => editor.key(key),
                }
                self.mode = Mode::Input { action, editor };
            }
            Mode::Confirm(confirmation) => match key.code {
                KeyCode::Char('y') => self.confirm(confirmation),
                KeyCode::Char('n') | KeyCode::Esc => {
                    self.message = "已取消".into();
                    self.sync_bar();
                }
                _ => self.mode = Mode::Confirm(confirmation),
            },
            Mode::Files(mut session) => {
                let count = session.rows().len();
                match key.code {
                    KeyCode::Esc => {
                        self.message.clear();
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
                    KeyCode::Enter => {
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
                    KeyCode::Esc => {
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
                            self.message = "请输入提交说明".into()
                        } else {
                            self.mode = Mode::CommitConfirm {
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
                plan,
                message,
                mut scroll,
            } => {
                match key.code {
                    KeyCode::Char('y') => {
                        self.start(
                            "提交暂存内容",
                            Effect::Commit,
                            move |service, cancel, _| {
                                Ok(Outcome::Committed(
                                    service.commit(&plan, &message, &cancel)?,
                                ))
                            },
                        );
                        return Ok(false);
                    }
                    KeyCode::Esc | KeyCode::Char('n') => {
                        self.message = "已取消".into();
                        self.sync_bar();
                        return Ok(false);
                    }
                    _ => scroll_key(&mut scroll, key),
                }
                self.mode = Mode::CommitConfirm {
                    plan,
                    message,
                    scroll,
                };
            }
            Mode::PushConfirm { plan, mut scroll } => {
                match key.code {
                    KeyCode::Char('y') => {
                        self.start(
                            "推送当前分支",
                            Effect::Push,
                            move |service, cancel, _| {
                                Ok(Outcome::Published(service.push(&plan, &cancel)?))
                            },
                        );
                        return Ok(false);
                    }
                    KeyCode::Esc | KeyCode::Char('n') => {
                        self.message = "已取消".into();
                        self.sync_bar();
                        return Ok(false);
                    }
                    _ => scroll_key(&mut scroll, key),
                }
                self.mode = Mode::PushConfirm { plan, scroll };
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
