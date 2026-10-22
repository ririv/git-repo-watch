//! Adjacent home-row shortcuts control focus and the repository viewport.
use crate::{
    Mode, Page, Screen,
    bar::InputBar,
    workspace::{Arrangement, Section, Workspace},
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

impl Screen {
    pub(super) fn repository_pane_key(&mut self, key: KeyEvent) -> bool {
        if self.page != Page::Detail
            || !matches!(self.bar, InputBar::Search(_))
            || self.workspace.session().is_none()
        {
            return false;
        }
        match (key.modifiers, key.code) {
            (KeyModifiers::CONTROL, KeyCode::Char('s' | 'd'))
                if matches!(
                    self.mode,
                    Mode::Browsing
                        | Mode::Panel(_)
                        | Mode::Files(_)
                        | Mode::Diff { .. }
                        | Mode::History(_)
                ) =>
            {
                self.switch_repository_section(
                    Section::of(&self.mode).advance(key.code == KeyCode::Char('s')),
                );
                true
            }
            (KeyModifiers::CONTROL, KeyCode::Char('f')) if Workspace::is_root(&self.mode) => {
                let session = self.workspace.session_mut().expect("open repository");
                session.arrangement = match session.arrangement {
                    Arrangement::Tiled => Arrangement::Maximized,
                    Arrangement::Maximized => Arrangement::Tiled,
                };
                true
            }
            (KeyModifiers::NONE, KeyCode::Esc)
                if Workspace::is_root(&self.mode)
                    && self.bar.editor().text.is_empty()
                    && self
                        .workspace
                        .session()
                        .expect("open repository")
                        .arrangement
                        == Arrangement::Maximized =>
            {
                self.workspace
                    .session_mut()
                    .expect("open repository")
                    .arrangement = Arrangement::Tiled;
                true
            }
            _ => false,
        }
    }
}
