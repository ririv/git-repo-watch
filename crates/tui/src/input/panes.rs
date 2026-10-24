//! Commands switch existing panes without changing their navigation depth.
use crate::{
    Mode, Page, Screen,
    workspace::{Arrangement, Section, Workspace},
};

impl Screen {
    pub(crate) fn cycle_pane(&mut self, backwards: bool) {
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
        let arrangement = match &mut self.mode {
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
    pub(super) fn pan_graph(&mut self, backwards: bool) {
        if let Mode::History(view) = &mut self.mode
            && let crate::history::View::Graph(session) = view.as_mut()
        {
            session.pan = if backwards {
                session.pan.saturating_sub(2)
            } else {
                (session.pan + 2).min(session.width.saturating_sub(1))
            };
        } else {
            self.message = i18n::tr!(PaneUnavailable).into();
        }
    }
}
