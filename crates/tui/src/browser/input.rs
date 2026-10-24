//! Directory navigation and preview focus keep command editing independent.
use super::{Focus, Session};
use crate::{Mode, Screen, workspace::Arrangement};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
impl Screen {
    pub(crate) fn browser_shortcut(&mut self, key: KeyEvent) -> bool {
        if !matches!(self.bar, crate::bar::InputBar::Search(_))
            || key.modifiers != KeyModifiers::CONTROL
            || !matches!(key.code, KeyCode::Char('s' | 'd' | 'f'))
        {
            return false;
        }
        if !matches!(self.mode, Mode::Browser(_)) {
            return false;
        }
        let Mode::Browser(session) = self.take_mode() else {
            unreachable!()
        };
        self.browser_key(session, key);
        true
    }
    pub(crate) fn browser_key(&mut self, mut session: Box<Session>, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                if session.focus == Focus::Preview {
                    session.focus = Focus::Tree;
                } else {
                    self.mode = Mode::Browser(session);
                    self.return_to_view();
                    return;
                }
            }
            KeyCode::Tab => session.cycle_focus(false),
            KeyCode::BackTab => session.cycle_focus(true),
            KeyCode::Char('s' | 'd') if key.modifiers == KeyModifiers::CONTROL => {
                session.cycle_focus(key.code == KeyCode::Char('s'));
            }
            KeyCode::Char('f') if key.modifiers == KeyModifiers::CONTROL => {
                session.arrangement = match session.arrangement {
                    Arrangement::Tiled => Arrangement::Maximized,
                    Arrangement::Maximized => Arrangement::Tiled,
                }
            }
            KeyCode::Up if session.focus == Focus::Tree => session.advance(-1),
            KeyCode::Down if session.focus == Focus::Tree => session.advance(1),
            KeyCode::PageUp if session.focus == Focus::Tree => session.advance(-10),
            KeyCode::PageDown if session.focus == Focus::Tree => session.advance(10),
            KeyCode::Home if session.focus == Focus::Tree => session.select(0),
            KeyCode::End if session.focus == Focus::Tree => session.select(usize::MAX),
            KeyCode::Enter | KeyCode::Right if session.focus == Focus::Tree => session.toggle(),
            KeyCode::Left if session.focus == Focus::Tree => {
                let within_tree = session.parent();
                if !within_tree {
                    self.mode = Mode::Browser(session);
                    self.return_to_view();
                    return;
                }
            }
            KeyCode::Up => {
                session.pane().position.scroll = session.pane().position.scroll.saturating_sub(1)
            }
            KeyCode::Down => {
                session.pane().position.scroll = session.pane().position.scroll.saturating_add(1)
            }
            KeyCode::PageUp => {
                session.pane().position.scroll = session.pane().position.scroll.saturating_sub(10)
            }
            KeyCode::PageDown => {
                session.pane().position.scroll = session.pane().position.scroll.saturating_add(10)
            }
            KeyCode::Home => session.pane().position.scroll = 0,
            KeyCode::End => session.pane().position.scroll = usize::MAX,
            _ => {}
        }
        self.mode = Mode::Browser(session);
        self.poll_browser();
    }
}
