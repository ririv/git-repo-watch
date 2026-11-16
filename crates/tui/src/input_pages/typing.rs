//! Temporary typing captures an action once and restores it when text becomes empty.
use super::{ActionPage, Preference, Scene};
use crate::{Mode, Screen, bar::InputBar};

impl Screen {
    pub(crate) fn begin_temporary_input(&mut self) {
        if self.tasks.session.open() || self.focus_mode != crate::focus::State::Inactive {
            return;
        }
        match &mut self.mode {
            Mode::Management(session) => {
                if let Some(draft) = session
                    .focus
                    .begin_typing(|controls| (controls.selected, controls.draft))
                {
                    self.bar = *draft;
                    if let InputBar::Command { menu, .. } = &mut self.bar
                        && matches!(menu, crate::command::completion::Menu::Loading(_))
                    {
                        *menu = Default::default();
                        self.refresh_option_candidates();
                    }
                }
            }
            Mode::CommitMessage { editor, .. } if self.bar.is_plain() => {
                if let crate::commit_form::Draft::Editing { focus, .. } = editor {
                    focus.start_typing();
                }
            }
            Mode::StashSave(session) => session.focus.start_typing(),
            Mode::Files(_) | Mode::Changes(_) => {
                let resume = if let Some(action) = self.focused_conflict_control() {
                    Some(ActionPage::Conflict(action))
                } else {
                    self.focused_replay_control().map(ActionPage::Replay)
                };
                if let Some(resume) = resume
                    && let Some(scene) = self.input_scene()
                {
                    let files = match &mut self.mode {
                        Mode::Files(files) => files,
                        Mode::Changes(session) => &mut session.files,
                        _ => unreachable!(),
                    };
                    files.input_preference = Preference::Typing { scene, resume };
                }
            }
            _ => {}
        }
    }
    pub(crate) fn restore_temporary_input(&mut self) {
        let empty = self.bar.is_plain() && self.bar.editor().text.is_empty();
        match &mut self.mode {
            Mode::Management(session) => {
                session.focus.restore_empty(empty, |selected| {
                    (
                        crate::management::Controls {
                            selected,
                            draft: Box::new(std::mem::replace(
                                &mut self.bar,
                                InputBar::CommandsOnly,
                            )),
                        },
                        (),
                    )
                });
            }
            Mode::CommitMessage {
                editor: crate::commit_form::Draft::Editing { editor, focus },
                ..
            } => focus.finish_empty(editor.text().is_empty()),
            Mode::StashSave(session) => session.focus.finish_empty(session.message.text.is_empty()),
            Mode::Files(_) | Mode::Changes(_)
                if empty
                    && !self
                        .bar
                        .command_input()
                        .is_some_and(|input| input.prefix_selected()) =>
            {
                self.restore_file_actions()
            }
            _ => {}
        }
    }
    fn restore_file_actions(&mut self) {
        let files = match &mut self.mode {
            Mode::Files(files) => files,
            Mode::Changes(session) => &mut session.files,
            _ => unreachable!(),
        };
        if !matches!(files.input_preference, Preference::Typing { .. }) {
            return;
        }
        let Preference::Typing { scene, resume } = std::mem::take(&mut files.input_preference)
        else {
            unreachable!()
        };
        match scene {
            Scene::Conflict(path) => {
                let Mode::Changes(session) = &mut self.mode else {
                    return;
                };
                let index = session
                    .files
                    .rows()
                    .iter()
                    .position(|change| change.location.name() == path && change.kind == 'U');
                let Some(index) = index else {
                    return;
                };
                session.files.cursor = crate::files::Cursor::File(index);
                session.focus = crate::changes::Focus::Preview;
            }
            Scene::Recovery { kind, commit } => {
                if !self
                    .replay_controls()
                    .is_some_and(|(_, active)| active.kind == kind && active.commit == commit)
                {
                    return;
                }
            }
            Scene::List => return,
        }
        if self.input_pages().position(resume.page()).is_none() {
            return;
        }
        self.sync_bar();
        self.show_input_page(resume.page());
        let files = match &mut self.mode {
            Mode::Files(files) => files,
            Mode::Changes(session) => &mut session.files,
            _ => unreachable!(),
        };
        match resume {
            ActionPage::Conflict(action) => files.recovery.choose_conflict(action),
            ActionPage::Replay(action) => files.recovery.choose(action),
        }
        self.poll_changes();
    }
}
