//! Contextual index actions and adjacent focus shortcuts for the changes workspace.
use crate::{
    Mode, Screen,
    bar::InputBar,
    changes::{Focus, Preview},
    workspace::Arrangement,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use engine::local::files::IndexAction;

impl Screen {
    pub(super) fn changes_shortcut(&mut self, key: KeyEvent) -> bool {
        if !matches!(self.bar, InputBar::Search(_)) {
            return false;
        }
        if matches!(self.mode, Mode::Files(_) | Mode::Changes(_))
            && key.modifiers == KeyModifiers::ALT
        {
            let action = match key.code {
                KeyCode::Char('s') => Some(IndexAction::Stage),
                KeyCode::Char('u') => Some(IndexAction::Unstage),
                _ => None,
            };
            if let Some(action) = action {
                self.edit_files(action);
                return true;
            }
        }
        let Mode::Changes(session) = &mut self.mode else {
            return false;
        };
        if key.modifiers == KeyModifiers::CONTROL {
            match key.code {
                KeyCode::Char('s' | 'd') => {
                    session.focus = match session.focus {
                        Focus::Files => Focus::Preview,
                        Focus::Preview => Focus::Files,
                    };
                    return true;
                }
                KeyCode::Char('f') => {
                    session.arrangement = match session.arrangement {
                        Arrangement::Tiled => Arrangement::Maximized,
                        Arrangement::Maximized => Arrangement::Tiled,
                    };
                    return true;
                }
                _ => {}
            }
        }
        false
    }
    pub(super) fn changes_key(&mut self, mut session: Box<crate::changes::Session>, key: KeyEvent) {
        let empty = self.bar.editor().text.is_empty();
        if empty && matches!(key.code, KeyCode::Esc | KeyCode::Left) {
            if session.arrangement == Arrangement::Maximized {
                session.arrangement = Arrangement::Tiled;
            } else if session.focus == Focus::Preview {
                session.focus = Focus::Files;
            } else {
                if matches!(self.page, crate::Page::View(_)) {
                    self.mode = Mode::Changes(session);
                    self.return_to_view();
                    return;
                }
                self.mode = match session.return_to {
                    crate::changes::ReturnTo::Files => Mode::Files(session.files),
                    crate::changes::ReturnTo::Repositories => Mode::Browsing,
                };
                self.sync_bar();
                return;
            }
        } else if matches!(key.code, KeyCode::Tab | KeyCode::BackTab) {
            session.focus = match session.focus {
                Focus::Files => Focus::Preview,
                Focus::Preview => Focus::Files,
            };
        } else if session.focus == Focus::Files {
            let files = &mut session.files;
            let count = files.rows().len();
            match key.code {
                KeyCode::Up => files.cursor = files.cursor.saturating_sub(1),
                KeyCode::Down => files.cursor = (files.cursor + 1).min(count.saturating_sub(1)),
                KeyCode::PageUp => files.cursor = files.cursor.saturating_sub(10),
                KeyCode::PageDown => {
                    files.cursor = (files.cursor + 10).min(count.saturating_sub(1))
                }
                KeyCode::Home => files.cursor = 0,
                KeyCode::End => files.cursor = count.saturating_sub(1),
                KeyCode::Char(' ') | KeyCode::Insert if count > 0 => {
                    let index = files.change_index(files.rows()[files.cursor]);
                    if !files.selected.remove(&index) {
                        files.selected.insert(index);
                    }
                }
                KeyCode::Enter | KeyCode::Right => session.focus = Focus::Preview,
                _ => {}
            }
        } else if let Preview::Ready { scroll, .. } = &mut session.preview {
            if key.code == KeyCode::End {
                *scroll = usize::MAX;
            } else {
                super::scroll_key(scroll, key);
            }
        }
        self.mode = Mode::Changes(session);
        self.poll_changes();
    }
}
