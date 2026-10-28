//! A global navigation mode keeps pane movement separate from search and content keys.
use crate::{Mode, Page, Screen, workspace::Section};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum State {
    #[default]
    Inactive,
    Selecting,
}
#[derive(Debug, Clone, Copy)]
pub(crate) enum Direction {
    Up,
    Left,
    Down,
    Right,
}
impl Screen {
    pub(crate) fn focused_pane_name(&self) -> &'static str {
        match &self.mode {
            Mode::Composition(session) => match session.focus {
                crate::composition::Focus::Commits => i18n::tr!(ComposePending),
                crate::composition::Focus::Units => i18n::tr!(ComposeUnits),
            },
            Mode::Browser(session) => match session.focus {
                crate::browser::Focus::Tree => i18n::tr!(BrowserTitle),
                crate::browser::Focus::Preview => match session.presentation().active() {
                    crate::browser::opening::TextMode::Source => i18n::tr!(FileSource),
                    crate::browser::opening::TextMode::Preview => session
                        .open_target()
                        .expect("rendered preview capability")
                        .opening(crate::browser::opening::TextMode::Preview)
                        .label(),
                },
            },
            Mode::Changes(session) => match session.focus {
                crate::changes::Focus::Files => i18n::tr!(Worktree),
                crate::changes::Focus::Preview => i18n::tr!(FileDiff),
            },
            Mode::History(view) if matches!(view.as_ref(), crate::history::View::Commit(_)) => {
                let crate::history::View::Commit(commit) = view.as_ref() else {
                    unreachable!()
                };
                match commit.focus {
                    crate::history::Focus::Message => i18n::tr!(HistoryMessage),
                    crate::history::Focus::Files => i18n::tr!(HistoryChanges),
                    crate::history::Focus::Preview => i18n::tr!(FileDiff),
                }
            }
            Mode::Browsing if self.page == Page::Detail => self
                .workspace
                .session()
                .expect("repository pane selection")
                .selection
                .title(),
            _ => Section::of(&self.mode).title(),
        }
    }
    pub(crate) fn pane_navigation_available(&self) -> bool {
        matches!(self.mode, Mode::Browser(_) | Mode::Changes(_))
            || matches!(&self.mode, Mode::History(view) if matches!(view.as_ref(), crate::history::View::Commit(_)))
            || matches!(&self.mode, Mode::Composition(session) if matches!(session.state, crate::composition::State::Browsing))
            || (self.page == Page::Detail
                && self.workspace.session().is_some()
                && matches!(
                    self.mode,
                    Mode::Browsing
                        | Mode::Panel(_)
                        | Mode::Branches(_)
                        | Mode::Files(_)
                        | Mode::History(_)
                ))
    }
    pub(crate) fn enter_focus(&mut self) {
        if self.pane_navigation_available() {
            self.focus_mode = State::Selecting;
            self.message.clear();
        } else {
            self.message = i18n::tr!(PaneUnavailable).into();
        }
    }
    pub(crate) fn focus_key(&mut self, key: KeyEvent) -> bool {
        if self.focus_mode != State::Selecting {
            return false;
        }
        if !self.pane_navigation_available() {
            self.focus_mode = State::Inactive;
            return false;
        }
        match (key.code, key.modifiers) {
            (KeyCode::Enter | KeyCode::Esc, _) => self.focus_mode = State::Inactive,
            (KeyCode::Tab, KeyModifiers::NONE) => self.cycle_pane(false),
            (KeyCode::BackTab, _) => self.cycle_pane(true),
            (KeyCode::Up, KeyModifiers::NONE) => self.move_pane(Direction::Up),
            (KeyCode::Left, KeyModifiers::NONE) => self.move_pane(Direction::Left),
            (KeyCode::Down, KeyModifiers::NONE) => self.move_pane(Direction::Down),
            (KeyCode::Right, KeyModifiers::NONE) => self.move_pane(Direction::Right),
            (KeyCode::Char('/'), KeyModifiers::NONE) => {
                self.focus_mode = State::Inactive;
                self.bar_paste("//");
            }
            _ => {}
        }
        true
    }
    fn move_pane(&mut self, direction: Direction) {
        use Direction::*;
        match &mut self.mode {
            Mode::History(view) if matches!(view.as_ref(), crate::history::View::Commit(_)) => {
                use crate::history::Focus::{Files, Message, Preview};
                let crate::history::View::Commit(commit) = view.as_mut() else {
                    unreachable!()
                };
                let wide = self
                    .body_area
                    .is_some_and(|area| area.width >= 100 && area.height >= 10);
                commit.focus = match (commit.focus, direction) {
                    (Message, Down) | (Preview, Left) => Files,
                    (Files | Preview, Up) => Message,
                    (Files, Right) if wide => Preview,
                    (focus, _) => focus,
                };
            }
            Mode::Composition(session) => {
                use crate::composition::Focus::{Commits, Units};
                session.focus = match (session.focus, direction) {
                    (Commits, Right | Down) => Units,
                    (Units, Left | Up) => Commits,
                    (focus, _) => focus,
                };
            }
            Mode::Changes(session) => {
                use crate::changes::Focus::{Files, Preview};
                if let Some(area) = self.body_area {
                    session.focus = match (session.focus, direction, area.width >= 110) {
                        (Files, Right, true) | (Files, Down, false) => Preview,
                        (Preview, Left, true) | (Preview, Up, false) => Files,
                        _ => session.focus,
                    };
                }
            }
            Mode::Browser(session) => {
                use crate::browser::{
                    Focus,
                    opening::{Presentation, TextMode},
                };
                let Some(body) = self.body_area else {
                    return;
                };
                let preview = crate::view::browser::areas(body, session)
                    .into_iter()
                    .find_map(|(area, focus)| (focus == Focus::Preview).then_some(area))
                    .unwrap_or(body);
                let horizontal = preview.width.saturating_sub(2) >= 40;
                match (session.focus, direction, session.presentation()) {
                    (Focus::Tree, Right, _) => {
                        session.focus = Focus::Preview;
                        session.focus_mode(TextMode::Source);
                    }
                    (Focus::Preview, Right, Presentation::Split(TextMode::Source))
                        if horizontal =>
                    {
                        session.focus_mode(TextMode::Preview)
                    }
                    (Focus::Preview, Left, Presentation::Split(TextMode::Preview))
                        if horizontal =>
                    {
                        session.focus_mode(TextMode::Source)
                    }
                    (Focus::Preview, Left, _) => session.focus = Focus::Tree,
                    (Focus::Preview, Down, Presentation::Split(TextMode::Source))
                        if !horizontal =>
                    {
                        session.focus_mode(TextMode::Preview)
                    }
                    (Focus::Preview, Up, Presentation::Split(TextMode::Preview)) if !horizontal => {
                        session.focus_mode(TextMode::Source)
                    }
                    _ => {}
                }
                self.poll_browser();
            }
            _ => {
                let section = match Section::focused(&self.mode) {
                    Some(section) => section,
                    None => {
                        self.workspace
                            .session()
                            .expect("repository panes")
                            .selection
                    }
                };
                let target = section.towards(direction);
                if Section::focused(&self.mode) != Some(target) {
                    self.switch_repository_section(target);
                }
            }
        }
    }
}
