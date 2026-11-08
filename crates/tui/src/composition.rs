//! Editable commit ordering and unit assignments, independent of background Git work.
pub(crate) mod waiting;
use crate::{Effect, Mode, Outcome, Screen, confirmation, multiline_editor};
use anyhow::{Result, ensure};
use engine::{
    cancel::Cancel,
    composition::{Prepared, Progress},
};
use ratatui::widgets::ListState;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Action {
    Open,
    Next,
    Regenerate,
    Edit,
    Up,
    Down,
    Merge,
    Split,
    Move,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Direction {
    Up,
    Down,
}
impl Direction {
    pub fn action(self) -> Action {
        match self {
            Self::Up => Action::Up,
            Self::Down => Action::Down,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Focus {
    Commits,
    Units,
}
#[derive(Debug)]
pub(crate) enum State {
    Browsing,
    Editing(multiline_editor::Editor),
    Moving { unit: String, target: usize },
    Confirm(confirmation::Choice),
    Submitting,
}
#[derive(Debug)]
pub(crate) struct Session {
    pub prepared: Prepared,
    pub selected: usize,
    pub unit: usize,
    pub focus: Focus,
    pub state: State,
    pub commits: ListState,
    pub units: ListState,
    pub scroll: usize,
    pub message_scroll: usize,
}
impl Session {
    pub fn new(prepared: Prepared) -> Self {
        Self {
            prepared,
            selected: 0,
            unit: 0,
            focus: Focus::Commits,
            state: State::Browsing,
            commits: ListState::default(),
            units: ListState::default(),
            scroll: 0,
            message_scroll: 0,
        }
    }
    pub fn ids(&self) -> &[String] {
        &self.prepared.plan.commits[self.selected].units
    }
    pub fn selected_unit(&self) -> Option<&engine::composition::Unit> {
        let id = self.ids().get(self.unit)?;
        self.prepared
            .snapshot
            .units
            .iter()
            .find(|unit| unit.id == *id)
    }
    pub fn selected_message(&self) -> &str {
        &self.prepared.plan.commits[self.selected].message
    }
    pub fn select(&mut self, index: usize) {
        self.selected = index.min(self.prepared.plan.commits.len() - 1);
        self.unit = 0;
        self.scroll = 0;
        self.message_scroll = 0;
    }
    pub fn restore(&mut self) {
        self.state = State::Browsing;
    }
    pub fn edit(&mut self) {
        let mut editor = multiline_editor::Editor::new(
            self.selected_message().into(),
            engine::commit_message::MESSAGE_LIMIT,
        );
        editor.start();
        self.state = State::Editing(editor);
    }
    pub fn change(&mut self, action: Action) -> Result<()> {
        let mut next = self.prepared.clone();
        let index = self.selected;
        let mut selection = index;
        match action {
            Action::Up if index > 0 => {
                next.plan.commits.swap(index, index - 1);
                selection -= 1;
            }
            Action::Down if index + 1 < next.plan.commits.len() => {
                next.plan.commits.swap(index, index + 1);
                selection += 1;
            }
            Action::Merge if index + 1 < next.plan.commits.len() => {
                let other = next.plan.commits.remove(index + 1);
                next.plan.commits[index].units.extend(other.units);
                next.plan.commits[index]
                    .message
                    .push_str(&format!("\n\n{}", other.message));
            }
            Action::Split => {
                ensure!(self.ids().len() > 1, i18n::tr!(ComposeSelected));
                let id = next.plan.commits[index].units.remove(self.unit);
                next.plan.commits.insert(
                    index + 1,
                    engine::composition::Step {
                        message: self.selected_message().into(),
                        units: vec![id],
                    },
                );
                selection += 1;
            }
            _ => return Ok(()),
        }
        next.validate_structure()?;
        self.prepared = next;
        self.select(selection);
        Ok(())
    }
    pub fn move_unit(&mut self, id: &str, target: usize) -> Result<()> {
        let mut next = self.prepared.clone();
        let index = self.selected;
        if target == index {
            return Ok(());
        }
        next.plan.commits[index].units.retain(|unit| unit != id);
        next.plan.commits[target].units.push(id.into());
        let selection = if next.plan.commits[index].units.is_empty() {
            next.plan.commits.remove(index);
            target - usize::from(target > index)
        } else {
            target
        };
        next.validate_structure()?;
        self.prepared = next;
        self.select(selection);
        Ok(())
    }
}
impl Screen {
    pub(crate) fn compose_action(&mut self, action: Action, path: Option<std::path::PathBuf>) {
        if !self.config.ai.enabled || !self.config.experimental_features {
            return;
        }
        if matches!(action, Action::Open | Action::Regenerate) {
            if action == Action::Open
                && (matches!(self.mode, Mode::Composition(_))
                    || matches!(&self.mode, Mode::CompositionWaiting(waiting) if matches!(waiting.state, waiting::State::Running(_))))
            {
                return;
            }
            let Some(path) = path else {
                self.message = i18n::tr!(SelectARepositoryFirst).into();
                return;
            };
            if self.repository_tasks.contains_key(&path) {
                self.message = i18n::tr!(RepositoryTaskBusy).into();
                return;
            }
            if self.repository_tasks.len() >= self.config.max_concurrency {
                self.message = i18n::tr!(RepositoryTaskLimit).into();
                return;
            }
            let origin = match self.take_mode() {
                Mode::CompositionWaiting(waiting) => waiting.origin,
                origin => Box::new(origin),
            };
            self.mode =
                Mode::CompositionWaiting(Box::new(waiting::Session::new(path.clone(), origin)));
            self.sync_bar();
            let config = self.config.ai.clone();
            self.start_repository(
                path.clone(),
                i18n::tr!(ComposeGenerating),
                Effect::ReadOnly,
                move |service, cancel, _| {
                    Ok(Outcome::Composed(Box::new(
                        service.compose(&path, &config, &cancel)?,
                    )))
                },
            );
            return;
        }
        let Mode::Composition(session) = &mut self.mode else {
            self.message = i18n::tr!(ComposeSelected).into();
            return;
        };
        if !matches!(session.state, State::Browsing) {
            return;
        }
        let result = match action {
            Action::Next => {
                session.select(0);
                session.state = State::Confirm(Default::default());
                Ok(())
            }
            Action::Edit => {
                session.edit();
                Ok(())
            }
            Action::Move => {
                if let Some(unit) = session.selected_unit() {
                    session.state = State::Moving {
                        unit: unit.id.clone(),
                        target: session.selected,
                    };
                }
                Ok(())
            }
            _ => session.change(action),
        };
        self.message = result
            .err()
            .map(|error| format!("{error:#}"))
            .unwrap_or_default();
        self.sync_bar();
    }
    pub(crate) fn compose_submit(&mut self) {
        if !self.config.ai.enabled || !self.config.experimental_features {
            return;
        }
        let Mode::Composition(session) = &self.mode else {
            return;
        };
        let path = session.prepared.snapshot.path.clone();
        if self.repository_tasks.contains_key(&path) {
            self.message = i18n::tr!(RepositoryTaskBusy).into();
            return;
        }
        if self.repository_tasks.len() >= self.config.max_concurrency {
            self.message = i18n::tr!(RepositoryTaskLimit).into();
            return;
        }
        let prepared = session.prepared.clone();
        self.start_repository(
            path,
            i18n::tr!(ComposeNext),
            Effect::Commit,
            move |service, cancel, _| {
                Ok(Outcome::CompositionCommitted(Box::new(
                    service.compose_commit(prepared, &cancel)?,
                )))
            },
        );
        if let Mode::Composition(session) = &mut self.mode {
            session.state = State::Submitting;
        }
        self.sync_bar();
    }
    pub(crate) fn finish_composition(&mut self, progress: Progress) {
        if !self.config.ai.enabled || !self.config.experimental_features {
            self.mode = Mode::Browsing;
            self.sync_bar();
            return;
        }
        match progress {
            Progress::More { prepared, result } => {
                self.mode = Mode::Composition(Box::new(Session::new(prepared)));
                self.message = i18n::tf!(CommittedRepositoryUpdated, &result.oid[..12], "");
            }
            Progress::Complete { .. } => {
                self.mode = Mode::Browsing;
                self.message = i18n::tr!(ComposeDone).into();
            }
        }
        self.sync_bar();
    }
    pub(crate) fn compose_key(
        &mut self,
        mut session: Box<Session>,
        key: crossterm::event::KeyEvent,
        _root: &Cancel,
    ) {
        use crossterm::event::{KeyCode, KeyModifiers};
        if !self.config.ai.enabled || !self.config.experimental_features {
            self.mode = Mode::Browsing;
            self.sync_bar();
            return;
        }
        match &mut session.state {
            State::Editing(editor) => match (key.code, key.modifiers) {
                (KeyCode::Esc, _) => session.restore(),
                (KeyCode::Char('s'), KeyModifiers::CONTROL) => {
                    if editor.text().trim().is_empty() {
                        self.message = i18n::tr!(EnterACommitMessage).into();
                    } else {
                        session.prepared.plan.commits[session.selected].message =
                            editor.text().into();
                        session.message_scroll = editor.scroll;
                        session.restore();
                    }
                }
                _ => {
                    if editor.key(key).is_err() {
                        self.message = i18n::tr!(AgentMessageInvalid).into();
                    }
                }
            },
            State::Confirm(choice) => match choice.input(key.code) {
                confirmation::Input::Execute => {
                    self.mode = Mode::Composition(session);
                    self.compose_submit();
                    return;
                }
                confirmation::Input::Return => session.restore(),
                confirmation::Input::Pending => {}
            },
            State::Moving { unit, target } => match key.code {
                KeyCode::Esc => session.restore(),
                KeyCode::Up => *target = target.saturating_sub(1),
                KeyCode::Down => {
                    *target = (*target + 1).min(session.prepared.plan.commits.len() - 1)
                }
                KeyCode::Enter => {
                    let id = unit.clone();
                    let destination = *target;
                    self.message = session
                        .move_unit(&id, destination)
                        .err()
                        .map(|error| format!("{error:#}"))
                        .unwrap_or_default();
                    session.restore();
                }
                _ => {}
            },
            State::Browsing => match key.code {
                KeyCode::Up | KeyCode::Down => {
                    if session.focus == Focus::Commits {
                        let next = if key.code == KeyCode::Up {
                            session.selected.saturating_sub(1)
                        } else {
                            session.selected + 1
                        };
                        session.select(next);
                    } else {
                        session.unit = if key.code == KeyCode::Up {
                            session.unit.saturating_sub(1)
                        } else {
                            (session.unit + 1).min(session.ids().len() - 1)
                        };
                        session.scroll = 0;
                    }
                }
                KeyCode::Enter => session.edit(),
                KeyCode::PageDown => session.scroll += 10,
                KeyCode::PageUp => session.scroll = session.scroll.saturating_sub(10),
                KeyCode::Esc | KeyCode::Left => {
                    self.mode = Mode::Browsing;
                    if matches!(self.page, crate::Page::View(_)) {
                        self.return_to_view();
                    }
                    self.sync_bar();
                    return;
                }
                _ => {}
            },
            State::Submitting => {
                if matches!(key.code, KeyCode::Esc | KeyCode::Enter) {
                    self.mode = Mode::Browsing;
                    if matches!(self.page, crate::Page::View(_)) {
                        self.return_to_view();
                    }
                    self.sync_bar();
                    return;
                }
            }
        }
        self.mode = Mode::Composition(session);
    }
}
