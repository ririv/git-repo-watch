//! Content navigation for the changes workspace.
use crate::{
    Mode, Screen,
    changes::{Focus, Preview},
    workspace::Arrangement,
};
use crossterm::event::{KeyCode, KeyEvent};

impl Screen {
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
            match key.code {
                KeyCode::Up => files.move_cursor(-1, self.file_layout),
                KeyCode::Down => files.move_cursor(1, self.file_layout),
                KeyCode::PageUp => files.move_cursor(-10, self.file_layout),
                KeyCode::PageDown => files.move_cursor(10, self.file_layout),
                KeyCode::Home => files.edge(false, self.file_layout),
                KeyCode::End => files.edge(true, self.file_layout),
                KeyCode::Char(' ') | KeyCode::Insert => files.toggle_current(),
                KeyCode::Enter | KeyCode::Right if files.current().is_some() => {
                    session.focus = Focus::Preview
                }
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
