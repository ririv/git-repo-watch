//! Execute concrete arguments captured from the command's ready target.
use crate::{Action, Effect, Mode, Outcome, Screen, clipboard, command::target::Execution};
use engine::cancel::Cancel;

impl Screen {
    pub(crate) fn execute_bound_command(
        &mut self,
        action: Action,
        execution: Execution,
        selection: clipboard::Selection,
        root: &Cancel,
    ) -> anyhow::Result<bool> {
        match execution {
            Execution::Local { path } => return self.activate_local(action, path, selection),
            Execution::Editor { root, target } => self.open_editor_target(root, target),
            Execution::Terminal { root, directory } => {
                self.start_repository(
                    root.clone(),
                    i18n::tr!(OpenRepositoryTerminal),
                    Effect::Terminal,
                    move |service, cancel, _| {
                        service.open_terminal(&root, &directory, &cancel)?;
                        Ok(Outcome::TerminalOpened(directory))
                    },
                );
            }
            Execution::Copy(payload) => self.submit_clipboard(Ok(payload)),
            Execution::Replay {
                repository,
                commits,
            } => {
                let Action::Replay(kind) = action else {
                    unreachable!("replay command target")
                };
                let mut draft = self.mutation_draft(action, Some(repository));
                draft.selection = selection;
                self.prepare_replay(kind, commits, draft);
            }
            Execution::Refresh(target) => self.refresh_target(target, root),
            Execution::Index(request) => self.execute_index_request(request),
            Execution::Discard(plan) => self.apply_partial_plan(plan),
            Execution::DiscardFiles { snapshot, changes } => {
                self.prepare_discard_files(snapshot, changes)
            }
            Execution::MoreHistory(graph) => self.more_history_target(graph),
            Execution::HistoryParent(commit) => self.cycle_history_parent_target(commit),
            Execution::BrowserDiff { root, entry } => self.open_browser_diff_target(root, entry),
            Execution::StashPreview {
                scope,
                session,
                entry,
                previous,
            } => self.open_stash_detail_target(scope, session, entry, Some(previous)),
            Execution::ConflictAccept {
                action,
                repository,
                resolution,
            } => self.accept_conflict_target(action, repository, resolution),
            Execution::ConflictReset {
                repository,
                resolution,
            } => self.reset_conflict_target(repository, resolution),
            Execution::Presentation(target) => {
                if let Mode::Browser(session) = &mut self.mode {
                    match action {
                        Action::BrowserLayout(layout) => {
                            session.present(&target, target.presentation.with_layout(layout))
                        }
                        Action::BrowserMode(mode) => session.open_with(&target, mode),
                        _ => unreachable!("file presentation command target"),
                    }
                    self.poll_browser();
                }
            }
        }
        Ok(false)
    }
}
