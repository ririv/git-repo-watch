//! Commands switch existing panes without changing their navigation depth.
use crate::{
    Mode, Page, Screen,
    workspace::{Arrangement, Section, Workspace},
};

impl Screen {
    #[cfg(test)]
    pub(crate) fn cycle_pane(&mut self, backwards: bool) {
        self.cycle_test_pane(backwards);
        self.sync_plain_bar();
    }
    #[cfg(test)]
    fn cycle_test_pane(&mut self, backwards: bool) {
        let wide = self
            .body_area
            .is_some_and(|area| area.width >= 100 && area.height >= 10);
        if let Some(inspect) = self.mode.inspection_mut() {
            use crate::inspection::Focus::{Files, Message, Preview};
            inspect.focus = match (inspect.focus, backwards) {
                (Message, false) | (Preview, true) => Files,
                (Files, false) if wide => Preview,
                (Message, true) if wide => Preview,
                (Message, true) => Files,
                _ => Message,
            };
            return;
        }
        match &mut self.mode {
            Mode::Composition(session) => {
                session.focus = match session.focus {
                    crate::composition::Focus::Commits => crate::composition::Focus::Units,
                    crate::composition::Focus::Units => crate::composition::Focus::Commits,
                }
            }
            Mode::Browser(session) => {
                session.cycle_focus(backwards);
                self.poll_browser();
            }
            Mode::Changes(session) => {
                session.focus = match session.focus {
                    crate::changes::Focus::Files => crate::changes::Focus::Preview,
                    crate::changes::Focus::Preview => crate::changes::Focus::Files,
                };
            }
            _ if self.page == Page::Detail && self.workspace.session().is_some() => {
                let target = match Section::focused(&self.mode) {
                    Some(section) => section.advance(backwards),
                    None if backwards => Section::History,
                    None => Section::Overview,
                };
                self.switch_repository_section(target);
            }
            _ => self.message = i18n::tr!(PaneUnavailable).into(),
        }
    }
    pub(super) fn zoom_pane(&mut self) {
        let arrangement = if let Some(inspect) = self.mode.inspection_mut() {
            Some(&mut inspect.arrangement)
        } else {
            match &mut self.mode {
                Mode::Changes(session) => Some(&mut session.arrangement),
                Mode::Browser(session) => Some(&mut session.arrangement),
                mode if self.page == Page::Detail
                    && Workspace::is_root(mode)
                    && Section::focused(mode).is_some() =>
                {
                    self.workspace
                        .session_mut()
                        .map(|session| &mut session.arrangement)
                }
                _ => None,
            }
        };
        if let Some(arrangement) = arrangement {
            *arrangement = match arrangement {
                Arrangement::Tiled => Arrangement::Maximized,
                Arrangement::Maximized => Arrangement::Tiled,
            };
        } else {
            self.message = i18n::tr!(PaneUnavailable).into();
        }
    }
}
