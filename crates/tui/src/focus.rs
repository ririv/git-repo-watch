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
        if let Some(inspect) = self.mode.inspection() {
            return match inspect.focus {
                crate::inspection::Focus::Message => inspect.detail.message_title(),
                crate::inspection::Focus::Files => i18n::tr!(HistoryChanges),
                crate::inspection::Focus::Preview => i18n::tr!(FileDiff),
            };
        }
        if let Some(session) = self.history_graph_target()
            && session.focus == crate::history::GraphFocus::Files
        {
            return i18n::tr!(HistoryChanges);
        }
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
                crate::changes::Focus::Preview => match &session.preview {
                    crate::changes::Preview::Ready { content, .. } => match content.as_ref() {
                        crate::document::View::Conflict(view) => match view.focus() {
                            crate::conflict::Focus::Compare => i18n::tr!(ConflictCompare),
                            crate::conflict::Focus::Result => i18n::tr!(ConflictResult),
                        },
                        _ => i18n::tr!(FileDiff),
                    },
                    _ => i18n::tr!(FileDiff),
                },
            },
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
        (self.history_graph_target().is_some() && self.history_preview.area.is_some())
            || matches!(self.mode, Mode::Browser(_) | Mode::Changes(_))
            || self.mode.inspection().is_some()
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
            self.message = crate::message::Message::error(i18n::tr!(PaneUnavailable).into());
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
        let wide = self
            .body_area
            .is_some_and(|area| area.width >= 100 && area.height >= 10);
        if let Some(inspect) = self.mode.inspection_mut() {
            use crate::inspection::Focus::{Files, Message, Preview};
            inspect.focus = match (inspect.focus, direction) {
                (Message, Down) | (Preview, Left) => Files,
                (Files | Preview, Up) => Message,
                (Files, Right) if wide => Preview,
                (focus, _) => focus,
            };
            return;
        }
        if self.history_preview.area.is_some()
            && let Mode::History(view) = &mut self.mode
            && let crate::history::View::Graph(session) = view.as_mut()
        {
            use crate::history::GraphFocus::{Files, Graph};
            match (session.focus, direction) {
                (Graph, Right) => {
                    session.focus = Files;
                    return;
                }
                (Files, Left) => {
                    session.focus = Graph;
                    return;
                }
                _ if self.page != Page::Detail || session.focus == Files => return,
                _ => {}
            }
        }
        match &mut self.mode {
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
                    let wide = area.width >= 110;
                    if session.focus == Preview
                        && let crate::changes::Preview::Ready {
                            content, scroll, ..
                        } = &mut session.preview
                        && let crate::document::View::Conflict(view) = content.as_mut()
                    {
                        use crate::conflict::Focus::{Compare, Result};
                        match (direction, view.focus()) {
                            (Left, _) if wide => session.focus = Files,
                            (Up, Result) => view.select(Compare, scroll),
                            (Up, Compare) if !wide => session.focus = Files,
                            (Down, Compare) => view.select(Result, scroll),
                            _ => {}
                        }
                    } else {
                        session.focus = match (session.focus, direction, wide) {
                            (Files, Right, true) | (Files, Down, false) => Preview,
                            (Preview, Left, true) | (Preview, Up, false) => Files,
                            _ => session.focus,
                        };
                    }
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
