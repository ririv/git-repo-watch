//! Content navigation for the changes workspace.
use crate::{
    Mode, Screen,
    changes::{Focus, Preview},
    workspace::Arrangement,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

impl Screen {
    pub(crate) fn changes_key(&mut self, mut session: Box<crate::changes::Session>, key: KeyEvent) {
        let empty = self.bar.editor().text.is_empty();
        if empty && matches!(key.code, KeyCode::Esc | KeyCode::Left) {
            if session.arrangement == Arrangement::Maximized {
                session.arrangement = Arrangement::Tiled;
            } else if session.focus == Focus::Preview {
                session.focus = Focus::Files;
                self.mode = Mode::Changes(session);
                self.show_input_page(crate::input_pages::Page::Input);
                self.poll_changes();
                return;
            } else {
                if matches!(self.page, crate::Page::View(_)) {
                    self.mode = Mode::Changes(session);
                    self.return_to_view();
                    return;
                }
                self.mode = match session.return_to {
                    crate::changes::ReturnTo::Files => Mode::Files(session.combined_files()),
                    crate::changes::ReturnTo::Repositories => Mode::Browsing,
                };
                self.sync_bar();
                return;
            }
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
                KeyCode::Enter if files.toggle_directory(self.file_layout) => {}
                KeyCode::Enter | KeyCode::Right if files.current().is_some() => {
                    session.focus = Focus::Preview
                }
                _ => {}
            }
        } else if let Preview::Ready {
            content, scroll, ..
        } = &mut session.preview
            && let crate::document::View::Conflict(view) = content.as_mut()
            && (view.resolution.reason().is_none() || view.resolution.resolved())
        {
            match key.code {
                KeyCode::Enter | KeyCode::Char(' ') if key.code == KeyCode::Enter || empty => {
                    self.mode = Mode::Changes(session);
                    self.open_conflict_controls();
                    return;
                }
                _ => view.navigate(key, scroll),
            }
        } else if key.code == KeyCode::Char(' ') && empty {
            self.mode = Mode::Changes(session);
            self.toggle_partial();
            return;
        } else if let Preview::Ready {
            scroll, editing, ..
        } = &mut session.preview
        {
            match (editing, key.code) {
                (crate::changes::edit::State::Ready(editing), KeyCode::Up | KeyCode::Down) => {
                    editing.advance(
                        if key.code == KeyCode::Up { -1 } else { 1 },
                        key.modifiers.contains(KeyModifiers::SHIFT),
                    );
                }
                (crate::changes::edit::State::Ready(editing), KeyCode::Home | KeyCode::End) => {
                    editing.advance(
                        if key.code == KeyCode::Home {
                            isize::MIN
                        } else {
                            isize::MAX
                        },
                        key.modifiers.contains(KeyModifiers::SHIFT),
                    )
                }
                (_, KeyCode::End) => *scroll = usize::MAX,
                _ => super::scroll_key(scroll, key),
            }
        }
        self.mode = Mode::Changes(session);
        self.poll_changes();
    }
}
