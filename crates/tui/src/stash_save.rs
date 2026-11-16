//! Stash review owns optional text and a frozen file scope before any repository write.
use crate::{Action, Editor, Mode, Screen, bar, confirmation};
use crossterm::event::{KeyCode, KeyEvent};
use engine::{
    local::stash::{self, SaveMode, SavePlan},
    operations::mutation::{Plan, Request},
};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Scope {
    Tracked,
    All,
    Selected,
}
impl Scope {
    pub fn token(self) -> &'static str {
        match self {
            Self::Tracked => "tracked",
            Self::All => "all",
            Self::Selected => "selected",
        }
    }
}
pub(crate) type Focus = crate::input_pages::Selection<confirmation::Choice>;
#[derive(Debug)]
pub(crate) struct Session {
    pub plan: SavePlan,
    pub message: Editor,
    pub focus: Focus,
    pub scroll: usize,
    pub origin: Box<Mode>,
    pub draft: bar::CommandDraft,
}
impl Session {
    pub fn new(plan: SavePlan, origin: Mode, draft: bar::CommandDraft) -> Self {
        Self {
            message: Editor::new(plan.message.input().into()),
            plan,
            focus: Focus::new(crate::input_pages::Availability::Actions(
                confirmation::Choice::Execute,
            )),
            scroll: 0,
            origin: Box::new(origin),
            draft,
        }
    }
    pub fn toggle_mode(&mut self) {
        self.focus.toggle(crate::input_pages::Availability::Actions(
            confirmation::Choice::Execute,
        ));
    }
}
pub(crate) fn review(plan: &SavePlan) -> String {
    format!(
        "{}: {}\n{}: {}\n\n{}\n{}",
        i18n::tr!(Path),
        crate::view::safe(&plan.path.display().to_string()),
        i18n::tr!(StashScopeLabel),
        plan.mode.label(),
        i18n::tf!(StashFilesCount, count = plan.files.len()),
        plan.files
            .iter()
            .map(|path| path
                .display()
                .to_string()
                .chars()
                .flat_map(|ch| if ch.is_control() {
                    ch.escape_default().collect::<Vec<_>>()
                } else {
                    vec![ch]
                })
                .collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    )
}
impl Screen {
    pub(crate) fn prepare_stash_save(
        &mut self,
        scope: Scope,
        path: Option<PathBuf>,
        selection: crate::clipboard::Selection,
    ) {
        let mode = match scope {
            Scope::Tracked => SaveMode::Tracked,
            Scope::All => SaveMode::IncludeUntracked,
            Scope::Selected => {
                match stash::SelectedFiles::new(selection.marked_files().iter().cloned()) {
                    Ok(files) => SaveMode::Selected { files },
                    Err(error) => {
                        self.message = format!("{error:#}");
                        return;
                    }
                }
            }
        };
        let mut draft = self.mutation_draft(Action::StashSave, path);
        draft.input = crate::command::Input::from_canonical(
            None,
            Editor::new(format!("/repo stash save {}", scope.token())),
        );
        draft.selection = selection;
        self.prepare_mutation(
            Request::Stash(stash::Request::Save {
                mode,
                message: stash::Message::Automatic,
            }),
            draft,
        );
    }
    pub(crate) fn stash_save_key(&mut self, mut session: Box<Session>, key: KeyEvent) {
        match key.code {
            KeyCode::Esc if matches!(session.focus, Focus::Actions(_)) => {
                session.focus = Focus::Input
            }
            KeyCode::Esc => {
                self.cancel_stash_save(*session);
                return;
            }
            KeyCode::BackTab => session.toggle_mode(),
            KeyCode::Tab => {}
            KeyCode::PageUp => session.scroll = session.scroll.saturating_sub(10),
            KeyCode::PageDown => session.scroll = session.scroll.saturating_add(10),
            KeyCode::Enter => match session.focus {
                Focus::Input => session.focus = Focus::Actions(confirmation::Choice::Execute),
                Focus::Actions(confirmation::Choice::Return) => {
                    self.cancel_stash_save(*session);
                    return;
                }
                Focus::Actions(confirmation::Choice::Execute) => {
                    session.plan.message = session.message.text.into();
                    self.execute_mutation(
                        Plan::Stash(stash::Plan::Save(session.plan)),
                        *session.origin,
                    );
                    return;
                }
            },
            KeyCode::Left | KeyCode::Right if matches!(session.focus, Focus::Actions(_)) => {
                session.focus = Focus::Actions(if key.code == KeyCode::Left {
                    confirmation::Choice::Execute
                } else {
                    confirmation::Choice::Return
                });
            }
            KeyCode::Char(_) if matches!(session.focus, Focus::Actions(_)) => {
                session.focus = Focus::Input;
                session.message.key(key);
            }
            _ if session.focus == Focus::Input => session.message.key(key),
            _ => {}
        }
        self.mode = Mode::StashSave(session);
    }
    fn cancel_stash_save(&mut self, session: Session) {
        self.mode = *session.origin;
        self.bar = bar::InputBar::Command {
            input: session.draft.input,
            selected: 0,
            menu: Default::default(),
            path: session.draft.path,
            selection: session.draft.selection,
            repository: session.draft.repository,
        };
        self.refresh_option_candidates();
        self.message.clear();
    }
}

impl Screen {
    pub(crate) fn stash_save_mouse(
        &mut self,
        event: crossterm::event::MouseEvent,
        root: &engine::cancel::Cancel,
    ) -> anyhow::Result<Option<bool>> {
        use crossterm::event::{MouseButton, MouseEventKind};
        use ratatui::layout::Position;
        let Mode::StashSave(session) = &mut self.mode else {
            return Ok(None);
        };
        let point = Position::new(event.column, event.row);
        if event.kind == MouseEventKind::Down(MouseButton::Left) && self.config.mouse.precise_click
        {
            if let Some(input) = self.input_area
                && let Some((_, choice)) =
                    crate::view::stash_save::actions(crate::view::input::content_area(input))
                        .into_iter()
                        .find(|(area, _)| area.contains(point))
            {
                session.focus = Focus::Actions(choice);
                let execute = self.clicks.press(
                    crate::input::clicks::Identity::StashSave {
                        repository: session.plan.path.clone(),
                        choice,
                    },
                    point,
                    std::time::Instant::now(),
                );
                if execute {
                    self.key(
                        KeyEvent::new(KeyCode::Enter, crossterm::event::KeyModifiers::NONE),
                        root,
                    )?;
                }
                return Ok(Some(true));
            }
            if self
                .body_area
                .is_some_and(|body| crate::view::stash_save::message_area(body).contains(point))
            {
                session.focus = Focus::Input;
                self.clicks.reset();
                return Ok(Some(true));
            }
        }
        if self.body_area.is_some_and(|area| area.contains(point)) {
            match event.kind {
                MouseEventKind::ScrollUp => session.scroll = session.scroll.saturating_sub(3),
                MouseEventKind::ScrollDown => session.scroll = session.scroll.saturating_add(3),
                _ => return Ok(None),
            }
            return Ok(Some(true));
        }
        Ok(None)
    }
}
