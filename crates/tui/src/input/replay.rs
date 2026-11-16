//! Route recovery actions before content scrolling and command input.
use crate::{
    Action, Mode, Screen,
    bar::ReturnTo,
    bar::{CommandDraft, InputBar},
    changes,
    replay::controls::{self, Focus},
};
use crossterm::event::{KeyCode, KeyEvent};
use engine::operations::{
    mutation::Request,
    replay::{Active, Control, State},
};

impl Screen {
    pub(crate) fn replay_controls(&self) -> Option<(&crate::FileSession, &Active)> {
        fn files(mode: &Mode) -> Option<&crate::FileSession> {
            match mode {
                Mode::Files(files) => Some(files),
                Mode::Changes(session) => Some(&session.files),
                Mode::Busy(pending) => files(&pending.origin),
                _ => None,
            }
        }
        let files = files(&self.mode)?;
        match &files.snapshot.operation.state {
            State::Active(active) => Some((files, active)),
            _ => None,
        }
    }
    pub(crate) fn focused_replay_control(&self) -> Option<Control> {
        if !matches!(self.mode, Mode::Files(_) | Mode::Changes(_))
            || !self.bar.is_plain()
            || self.focus_mode != crate::focus::State::Inactive
        {
            return None;
        }
        self.replay_controls()
            .and_then(|(files, _)| files.recovery.selected())
    }
    fn replay_focus_mut(&mut self) -> Option<&mut Focus> {
        match &mut self.mode {
            Mode::Files(files) => Some(&mut files.recovery),
            Mode::Changes(session) => Some(&mut session.files.recovery),
            _ => None,
        }
    }
    pub(crate) fn close_replay_controls(&mut self) -> bool {
        let Some(focus) = self.replay_focus_mut() else {
            return false;
        };
        let previous = std::mem::take(focus);
        let selected = previous.active();
        if let Focus::CommandAction { draft, cursor, .. }
        | Focus::CommandConflict { draft, cursor, .. } = previous
        {
            self.bar = InputBar::Command {
                input: draft.input,
                selected: cursor,
                menu: Default::default(),
                path: draft.path,
                selection: draft.selection,
                repository: draft.repository,
            };
            self.refresh_option_candidates();
        }
        selected
    }
    pub(crate) fn resume_replay_search(&mut self) {
        if self.close_replay_controls() && !matches!(self.bar, InputBar::Command { .. }) {
            if let Mode::Changes(session) = &mut self.mode {
                session.focus = changes::Focus::Files;
            }
            self.sync_plain_bar();
        }
    }
    pub(crate) fn preview_replay_control(&mut self, control: Control) {
        if !matches!(self.mode, Mode::Files(_) | Mode::Changes(_)) {
            return;
        }
        let Some((files, active)) = self.replay_controls() else {
            return;
        };
        if !controls::enabled(active, control) {
            self.message = i18n::tr!(ReplayResolveFirst).into();
            return;
        }
        let path = files.snapshot.path.clone();
        self.replay_focus_mut()
            .expect("recovery file view")
            .choose(control);
        let mut draft = self.mutation_draft(Action::ReplayControl(control), Some(path));
        draft.return_to = ReturnTo::View;
        self.prepare_mutation(Request::ReplayControl(control), draft);
    }
    pub(crate) fn has_conflict_page(&self) -> bool {
        matches!(&self.mode, Mode::Changes(session) if session.files.current().is_some_and(|change| change.kind == 'U'))
    }
    pub(crate) fn input_pages(&self) -> crate::input_pages::Pages {
        use crate::input_pages::Pages;
        match (self.has_conflict_page(), self.replay_controls().is_some()) {
            (true, true) => Pages::ConflictReplay,
            (true, false) => Pages::Conflict,
            (false, true) => Pages::Replay,
            (false, false) => Pages::Input,
        }
    }
    pub(crate) fn input_page(&self) -> crate::input_pages::Page {
        match self.mode.file_session() {
            Some(files) => files.recovery.page(),
            None => crate::input_pages::Page::Input,
        }
    }
    pub(crate) fn input_scene(&self) -> Option<crate::input_pages::Scene> {
        use crate::input_pages::Scene;
        if !matches!(self.mode, Mode::Files(_) | Mode::Changes(_)) {
            return None;
        }
        if let Some((_, active)) = self.replay_controls()
            && active.conflicts.is_empty()
        {
            return Some(Scene::Recovery {
                kind: active.kind,
                commit: active.commit,
            });
        }
        match &self.mode {
            Mode::Changes(session)
                if session.focus == changes::Focus::Preview
                    && session
                        .files
                        .current()
                        .is_some_and(|change| change.kind == 'U') =>
            {
                Some(Scene::Conflict(
                    session
                        .files
                        .current()
                        .expect("conflicted file")
                        .location
                        .name()
                        .to_owned(),
                ))
            }
            Mode::Changes(_) | Mode::Files(_) => match self.replay_controls() {
                Some((_, active)) => Some(Scene::Recovery {
                    kind: active.kind,
                    commit: active.commit,
                }),
                None => Some(Scene::List),
            },
            _ => None,
        }
    }
    fn preference_mut(&mut self) -> Option<&mut crate::input_pages::Preference> {
        match &mut self.mode {
            Mode::Files(files) => Some(&mut files.input_preference),
            Mode::Changes(session) => Some(&mut session.files.input_preference),
            _ => None,
        }
    }
    pub(crate) fn dismiss_input_actions(&mut self) -> bool {
        if self.close_management_actions() {
            return true;
        }
        let closed = self.close_replay_controls();
        if closed {
            self.show_input_page(crate::input_pages::Page::Input);
        }
        closed
    }
    pub(crate) fn show_input_page(&mut self, page: crate::input_pages::Page) {
        if self.input_pages().position(page).is_none() {
            return;
        }
        if let Some(scene) = self.input_scene()
            && let Some(preference) = self.preference_mut()
        {
            *preference = crate::input_pages::Preference::Manual { scene, page };
        }
        self.apply_input_page(page);
    }
    fn apply_input_page(&mut self, page: crate::input_pages::Page) {
        use crate::input_pages::Page;
        if self.input_pages().position(page).is_none() {
            return;
        }
        if page == Page::Input {
            self.close_replay_controls();
            return;
        }
        let conflict_default = if self
            .conflict_view()
            .is_some_and(|view| view.resolution.resolved())
        {
            crate::conflict::controls::Action::Stage
        } else {
            crate::conflict::controls::Action::Accept(
                engine::local::files::conflict::resolve::Choice::Left,
            )
        };
        let previous = std::mem::take(self.replay_focus_mut().expect("file input pages"));
        let saved = match previous {
            Focus::CommandAction { draft, cursor, .. }
            | Focus::CommandConflict { draft, cursor, .. } => Some((draft, cursor)),
            _ => {
                if matches!(self.bar, InputBar::Command { .. }) {
                    let InputBar::Command {
                        input,
                        selected,
                        path,
                        selection,
                        repository,
                        ..
                    } = std::mem::replace(&mut self.bar, InputBar::CommandsOnly)
                    else {
                        unreachable!()
                    };
                    self.command_reader.clear();
                    Some((
                        Box::new(CommandDraft {
                            input,
                            path,
                            selection,
                            repository,
                            return_to: ReturnTo::Command,
                        }),
                        selected,
                    ))
                } else {
                    None
                }
            }
        };
        let focus = match (page, saved) {
            (Page::Replay, Some((draft, cursor))) => Focus::CommandAction {
                selected: Control::Continue,
                draft,
                cursor,
            },
            (Page::Replay, None) => Focus::Action(Control::Continue),
            (Page::Conflict, Some((draft, cursor))) => Focus::CommandConflict {
                selected: conflict_default,
                draft,
                cursor,
            },
            (Page::Conflict, None) => Focus::Conflict(conflict_default),
            (Page::Input, _) => unreachable!("input restores its saved draft"),
        };
        *self.replay_focus_mut().expect("file input pages") = focus;
        self.message.clear();
    }
    pub(crate) fn normalize_input_page(&mut self) -> bool {
        let before = (self.input_page(), self.focused_conflict_control());
        let page = self.input_page();
        if self.input_pages().position(page).is_none() {
            self.close_replay_controls();
        }
        if self.bar.is_plain()
            && self.bar.editor().text.is_empty()
            && self.focus_mode == crate::focus::State::Inactive
            && let Some(scene) = self.input_scene()
        {
            let preferred = self
                .preference_mut()
                .expect("file input preference")
                .resolve(&scene);
            if self.input_pages().position(preferred).is_some() && self.input_page() != preferred {
                self.apply_input_page(preferred);
            }
        }
        before != (self.input_page(), self.focused_conflict_control())
    }

    pub(super) fn replay_controls_key(&mut self, key: KeyEvent) -> bool {
        if !matches!(self.mode, Mode::Files(_) | Mode::Changes(_))
            || !matches!(
                self.bar,
                InputBar::Search(_) | InputBar::CommandsOnly | InputBar::Command { .. }
            )
            || self.focus_mode != crate::focus::State::Inactive
        {
            return false;
        }
        self.normalize_input_page();
        if self.input_pages().all().len() == 1 {
            return false;
        }
        if key.code == KeyCode::BackTab {
            let next = self.input_pages().next(self.input_page());
            self.show_input_page(next);
            self.message.clear();
            return true;
        }
        if self.conflict_control_key(key) {
            return true;
        }
        if self.focused_replay_control().is_some() {
            match key.code {
                KeyCode::Left | KeyCode::Right if key.modifiers.is_empty() => {
                    self.replay_focus_mut()
                        .unwrap()
                        .step(key.code == KeyCode::Left);
                    self.message.clear();
                    return true;
                }
                KeyCode::Home | KeyCode::End => {
                    self.replay_focus_mut()
                        .unwrap()
                        .choose(if key.code == KeyCode::Home {
                            Control::Continue
                        } else {
                            Control::Abort
                        });
                    self.message.clear();
                    return true;
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    self.preview_replay_control(self.focused_replay_control().unwrap());
                    return true;
                }
                _ => {}
            }
        }
        let active =
            self.focused_replay_control().is_some() || self.focused_conflict_control().is_some();
        if active {
            match key.code {
                KeyCode::Esc => {
                    self.dismiss_input_actions();
                    self.message.clear();
                }
                KeyCode::Char(_) | KeyCode::Backspace | KeyCode::Delete => {
                    if self.command_trigger_key(key).code == KeyCode::Char('/') {
                        self.close_replay_controls();
                        self.open_command_bar();
                        return true;
                    }
                    self.resume_replay_search();
                    return false;
                }
                _ => {}
            }
        }
        active
    }
}
