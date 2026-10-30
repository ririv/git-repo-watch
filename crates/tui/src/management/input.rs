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
        self.mode = Mode::Management(Box::new(session));
        self.sync_bar();
    }
    pub(crate) fn scan_command(&mut self, raw: &str) {
        match command::scan::request(raw, self.service.scheduler.user_home()) {
            Ok(request) => self.start(
                i18n::tr!(ActionScan),
                Effect::ReadOnly,
                move |service, cancel, _| {
                    Ok(Outcome::Scanned(Box::new(service.scan(&request, &cancel)?)))
                },
            ),
            Err(error) => self.message = format!("{error:#}"),
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
    pub(crate) fn management_key(&mut self, mut session: Box<Session>, key: KeyEvent) {
        if key.code == KeyCode::Esc {
            self.mode = Mode::Management(session);
            self.return_to_view();
            return;
        }
        if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('a') {
            session.toggle_all();
        } else {
            match (session.focus, key.code) {
                (Focus::List, KeyCode::Up) => session.advance(-1),
                (Focus::List, KeyCode::Down) => session.advance(1),
                (Focus::List, KeyCode::PageUp) => session.advance(-10),
                (Focus::List, KeyCode::PageDown) => session.advance(10),
                (Focus::List, KeyCode::Home) => session
                    .state
                    .select((!session.rows().is_empty()).then_some(0)),
                (Focus::List, KeyCode::End) => {
                    session.state.select(session.rows().len().checked_sub(1))
                }
                (Focus::List, KeyCode::Enter | KeyCode::Insert | KeyCode::Char(' '))
                    if matches!(session.source, Source::Scan { .. }) =>
                {
                    if let Some(index) = session.state.selected() {
                        session.toggle(index);
                    }
                }
                (Focus::List, KeyCode::Tab | KeyCode::BackTab)
                    if matches!(session.source, Source::Scan { .. }) =>
                {
                    session.focus = Focus::Actions(0)
                }
                (Focus::Actions(_), KeyCode::Up | KeyCode::Tab | KeyCode::BackTab) => {
                    session.focus = Focus::List
                }
                (Focus::Actions(index), KeyCode::Left)
                    if matches!(session.source, Source::Scan { .. }) =>
                {
                    session.focus = Focus::Actions(index.saturating_sub(1))
                }
                (Focus::Actions(index), KeyCode::Right)
                    if matches!(session.source, Source::Scan { .. }) =>
                {
                    session.focus = Focus::Actions((index + 1).min(session.actions().len() - 1))
                }
                (Focus::Actions(index), KeyCode::Enter) => {
                    let action = session.actions()[index];
                    self.mode = Mode::Management(session);
                    self.management_action(action);
                    return;
                }
                _ => {}
            }
        }
        self.mode = Mode::Management(session);
    }
}
