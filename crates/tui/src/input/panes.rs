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
                        | Mode::Branches(_)
                        | Mode::Files(_)
                        | Mode::History(_)
                ) =>
            {
                let backwards = key.code == KeyCode::Char('s');
                let section = match Section::focused(&self.mode) {
                    Some(section) => section.advance(backwards),
                    None if backwards => Section::History,
                    None => Section::Overview,
                };
                self.switch_repository_section(section);
                true
            }
            (KeyModifiers::CONTROL, KeyCode::Char('f'))
                if Workspace::is_root(&self.mode) && Section::focused(&self.mode).is_some() =>
            {
                let session = self.workspace.session_mut().expect("open repository");
                session.arrangement = match session.arrangement {
                    Arrangement::Tiled => Arrangement::Maximized,
                    Arrangement::Maximized => Arrangement::Tiled,
                };
                true
            }
            _ => false,
        }
    }
}
