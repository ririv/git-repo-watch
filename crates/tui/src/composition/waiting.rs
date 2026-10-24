//! Explicit generation state before an editable commit plan exists.
use crate::{Mode, Screen, composition, spinner::Spinner};
use crossterm::event::{KeyCode, KeyEvent};
use std::{
    path::{Path, PathBuf},
    time::Instant,
};

#[derive(Debug)]
pub(crate) enum State {
    Running(Spinner),
    Failed { detail: String, scroll: usize },
}
#[derive(Debug)]
pub(crate) struct Session {
    pub path: PathBuf,
    pub origin: Box<Mode>,
    pub state: State,
}
impl Session {
    pub fn new(path: PathBuf, origin: Box<Mode>) -> Self {
        Self {
            path,
            origin,
            state: State::Running(Spinner::new(Instant::now())),
        }
    }
    pub fn hint(&self) -> &'static str {
        match self.state {
            State::Running(_) => i18n::tr!(ComposeWaitingHint),
            State::Failed { .. } => i18n::tr!(ComposeRetryHint),
        }
    }
}
pub(crate) fn complete(
    mode: &mut Mode,
    path: &Path,
    prepared: &engine::composition::Prepared,
) -> Option<Mode> {
    match mode {
        Mode::CompositionWaiting(waiting) if waiting.path == path => {
            let Mode::CompositionWaiting(waiting) = std::mem::replace(
                mode,
                Mode::Composition(Box::new(composition::Session::new(prepared.clone()))),
            ) else {
                unreachable!()
            };
            Some(*waiting.origin)
        }
        Mode::Error { origin, .. } => complete(origin, path, prepared),
        Mode::Busy(pending) => complete(&mut pending.origin, path, prepared),
        _ => None,
    }
}
pub(crate) fn fail(mode: &mut Mode, path: &Path, detail: &str) {
    match mode {
        Mode::CompositionWaiting(waiting) if waiting.path == path => {
            waiting.state = State::Failed {
                detail: detail.into(),
                scroll: 0,
            };
        }
        Mode::Error { origin, .. } => fail(origin, path, detail),
        Mode::Busy(pending) => fail(&mut pending.origin, path, detail),
        _ => {}
    }
}
impl Screen {
    pub(crate) fn composition_waiting_key(&mut self, mut waiting: Box<Session>, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Left => {
                if let Some(task) = self.repository_tasks.get(&waiting.path) {
                    task.cancel.cancel();
                }
                self.mode = *waiting.origin;
                self.sync_bar();
                return;
            }
            KeyCode::Enter if matches!(waiting.state, State::Failed { .. }) => {
                let path = waiting.path.clone();
                self.mode = Mode::CompositionWaiting(waiting);
                self.compose_action(composition::Action::Regenerate, Some(path));
                return;
            }
            KeyCode::PageUp | KeyCode::PageDown => {
                if let State::Failed { scroll, .. } = &mut waiting.state {
                    *scroll = if key.code == KeyCode::PageUp {
                        scroll.saturating_sub(10)
                    } else {
                        scroll.saturating_add(10)
                    };
                }
            }
            _ => {}
        }
        self.mode = Mode::CompositionWaiting(waiting);
    }
}
