//! Branch list navigation never changes HEAD or repository contents.
use crate::{Mode, Page, Screen, branches::View};
use crossterm::event::{KeyCode, KeyEvent};
impl Screen {
    pub(super) fn branch_key(&mut self, mut view: Box<View>, key: KeyEvent) {
        match view.as_mut() {
            View::List(session) => match key.code {
                KeyCode::Esc | KeyCode::Left => {
                    if matches!(self.page, Page::View(_)) {
                        self.mode = Mode::Branches(view);
                        self.return_to_view();
                        return;
                    }
                    if self.page == Page::Detail {
                        self.mode = Mode::Branches(view);
                        self.leave_repository_section();
                    } else {
                        self.sync_bar();
                    }
                    return;
                }
                KeyCode::Up => session.advance(-1),
                KeyCode::Down => session.advance(1),
                KeyCode::PageUp => session.advance(-10),
                KeyCode::PageDown => session.advance(10),
                KeyCode::Home => session
                    .state
                    .select((!session.rows().is_empty()).then_some(0)),
                KeyCode::End => session.state.select(session.rows().len().checked_sub(1)),
                KeyCode::Enter | KeyCode::Right if session.selected().is_some() => {
                    let entry =
                        std::sync::Arc::new(session.selected().expect("selected branch").clone());
                    let session = session.clone();
                    *view = View::Detail {
                        session,
                        entry,
                        scroll: 0,
                    };
                }
                _ => {}
            },
            View::Detail {
                session, scroll, ..
            } => match key.code {
                KeyCode::Esc | KeyCode::Left => {
                    *view = View::List(session.clone());
                }
                _ => super::scroll_key(scroll, key),
            },
        }
        self.mode = Mode::Branches(view);
        self.sync_bar();
    }
}
