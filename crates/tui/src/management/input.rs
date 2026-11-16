//! Registration actions preserve the review while configuration writes run in the background.
use super::*;
use crate::{Configuration, Effect, Mode, Outcome, Page, Screen, command};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

impl Screen {
    pub(crate) fn open_management(&mut self) {
        if matches!(&self.mode, Mode::Management(session) if matches!(session.source, Source::Managed))
        {
            return;
        }
        let origin = self.take_mode();
        self.enter_management(Session::managed(&self.config.repositories), origin);
    }
    pub(crate) fn enter_management(&mut self, session: Session, origin: Mode) {
        self.remember_view(origin);
        self.page = Page::Configuration(Configuration::Repositories);
        let initial: crate::input_pages::Selection<Action> =
            crate::input_pages::Selection::new(session.available_actions());
        self.mode = Mode::Management(Box::new(session));
        self.sync_bar();
        if let crate::input_pages::Selection::Actions(selected) = initial {
            self.open_management_actions(selected);
        }
    }
    pub(crate) fn scan_command(&mut self, raw: &str) -> bool {
        match command::scan::request(raw, self.service.scheduler.user_home()) {
            Ok(request) => {
                self.start(
                    i18n::tr!(ActionScan),
                    Effect::ReadOnly,
                    move |service, cancel, _| {
                        Ok(Outcome::Scanned(Box::new(service.scan(&request, &cancel)?)))
                    },
                );
                true
            }
            Err(error) => {
                self.message = format!("{error:#}");
                false
            }
        }
    }
    pub(crate) fn management_action(&mut self, action: Action) {
        match action {
            Action::Back => self.return_to_view(),
            Action::Apply => {
                let paths = match &self.mode {
                    Mode::Management(session) => session.selection(),
                    _ => Vec::new(),
                };
                if paths.is_empty() {
                    self.message = i18n::tr!(ScanEmptySelection).into();
                    return;
                }
                self.start(
                    i18n::tr!(ScanAddSelected),
                    Effect::Configuration,
                    move |service, _, _| Ok(Outcome::RegistrationsAdded(service.add(&paths)?)),
                );
            }
        }
    }
    fn open_management_actions(&mut self, selected: Action) {
        let Mode::Management(session) = &self.mode else {
            return;
        };
        if session.actions().is_empty() || !session.focus.is_input() {
            return;
        }
        let draft = Box::new(std::mem::replace(
            &mut self.bar,
            crate::bar::InputBar::CommandsOnly,
        ));
        let Mode::Management(session) = &mut self.mode else {
            unreachable!()
        };
        session.focus = Focus::Actions(Controls { selected, draft });
    }
    pub(crate) fn close_management_actions(&mut self) -> bool {
        let Mode::Management(session) = &mut self.mode else {
            return false;
        };
        let Some(Controls { draft, .. }) = session.focus.take_actions() else {
            return false;
        };
        self.bar = *draft;
        if let crate::bar::InputBar::Command { menu, .. } = &mut self.bar
            && matches!(menu, crate::command::completion::Menu::Loading(_))
        {
            *menu = Default::default();
            self.refresh_option_candidates();
        }
        true
    }
    pub(crate) fn management_controls_key(&mut self, key: KeyEvent) -> bool {
        let Mode::Management(session) = &self.mode else {
            return false;
        };
        let crate::input_pages::Availability::Actions(default_action) = session.available_actions()
        else {
            return false;
        };
        if key.code == KeyCode::BackTab {
            let next = session.focus.page().next();
            if next == crate::input_pages::Pair::Content {
                self.close_management_actions();
            } else {
                self.open_management_actions(default_action);
            }
            self.view_revision += 1;
            return true;
        }
        let Mode::Management(session) = &mut self.mode else {
            unreachable!()
        };
        let Focus::Actions(Controls { selected, .. }) = &mut session.focus else {
            return false;
        };
        match key.code {
            KeyCode::Left | KeyCode::Right => *selected = selected.next(),
            KeyCode::Enter | KeyCode::Char(' ') => {
                let action = *selected;
                self.management_action(action);
            }
            KeyCode::Esc => {
                self.close_management_actions();
            }
            KeyCode::Tab => {}
            KeyCode::Up => session.advance(-1),
            KeyCode::Down => session.advance(1),
            KeyCode::PageUp => session.advance(-10),
            KeyCode::PageDown => session.advance(10),
            KeyCode::Home => session
                .state
                .select((!session.rows().is_empty()).then_some(0)),
            KeyCode::End => session.state.select(session.rows().len().checked_sub(1)),
            _ => {
                self.close_management_actions();
                return false;
            }
        }
        self.view_revision += 1;
        true
    }
    pub(crate) fn management_key(&mut self, mut session: Box<Session>, key: KeyEvent) {
        if key.code == KeyCode::Esc {
            self.mode = Mode::Management(session);
            self.return_to_view();
            return;
        }
        if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('a') {
            session.toggle_all();
        } else {
            match key.code {
                KeyCode::Up => session.advance(-1),
                KeyCode::Down => session.advance(1),
                KeyCode::PageUp => session.advance(-10),
                KeyCode::PageDown => session.advance(10),
                KeyCode::Home => session
                    .state
                    .select((!session.rows().is_empty()).then_some(0)),
                KeyCode::End => session.state.select(session.rows().len().checked_sub(1)),
                KeyCode::Enter | KeyCode::Insert | KeyCode::Char(' ')
                    if matches!(session.source, Source::Scan { .. }) =>
                {
                    if let Some(index) = session.state.selected() {
                        session.toggle(index);
                    }
                }
                _ => {}
            }
        }
        self.mode = Mode::Management(session);
    }
}
