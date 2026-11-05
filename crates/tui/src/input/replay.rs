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
        let selected = previous.selected().is_some();
        if let Focus::CommandAction { draft, cursor, .. } = previous {
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
    pub(super) fn replay_controls_key(&mut self, key: KeyEvent) -> bool {
        if !matches!(self.mode, Mode::Files(_) | Mode::Changes(_))
            || !matches!(
                self.bar,
                InputBar::Search(_) | InputBar::CommandsOnly | InputBar::Command { .. }
            )
            || self.focus_mode != crate::focus::State::Inactive
            || self.replay_controls().is_none()
        {
            return false;
        }
        if self.focused_replay_control().is_some() {
            match key.code {
                KeyCode::Left | KeyCode::Right if key.modifiers.is_empty() => {
                    self.replay_focus_mut()
                        .unwrap()
                        .step(key.code == KeyCode::Left);
                    self.message.clear();
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
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    self.preview_replay_control(self.focused_replay_control().unwrap());
                }
                KeyCode::Esc | KeyCode::BackTab => {
                    self.close_replay_controls();
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
            return true;
        }
        let enter = key.code == KeyCode::BackTab;
        if enter {
            if let InputBar::Command {
                input, selected, ..
            } = &self.bar
            {
                let layout =
                    crate::command::argument_input(&input.query()).is_some_and(|(argument, _)| {
                        argument.options() == Some(crate::command::options::Kind::DiffLayout)
                    }) || self
                        .search_commands()
                        .and_then(|commands| commands.get(*selected).copied())
                        .is_some_and(|command| command.action == Action::DiffLayoutMenu);
                if layout {
                    return false;
                }
            }
            let focus = if matches!(self.bar, InputBar::Command { .. }) {
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
                Focus::CommandAction {
                    selected: Control::Continue,
                    draft: Box::new(CommandDraft {
                        input,
                        path,
                        selection,
                        repository,
                        return_to: ReturnTo::Command,
                    }),
                    cursor: selected,
                }
            } else {
                Focus::Action(Control::Continue)
            };
            *self.replay_focus_mut().unwrap() = focus;
            self.message.clear();
        }
        enter
    }
}
