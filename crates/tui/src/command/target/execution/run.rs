//! Execute concrete arguments captured from the command's ready target.
use super::{Execution, Permit, Request};
use crate::{Action, Effect, Mode, Outcome, Screen};
use engine::cancel::Cancel;

impl Execution {
    pub(crate) fn execute(self, screen: &mut Screen, root: &Cancel) -> anyhow::Result<bool> {
        let Execution {
            action,
            selection,
            request,
            return_to,
        } = self;
        let permit = Permit { _private: () };
        match request {
            Request::Task(operation) => screen.execute_task(operation, &permit),
            Request::Selection(operation) => {
                if let Err(error) = operation.execute(screen, &permit) {
                    screen.message = crate::message::Message::error(error.to_string());
                }
            }
            Request::Worktree { repository, entry } => match action {
                Action::WorktreeOpen => {
                    match std::sync::Arc::make_mut(&mut screen.service).authorize_worktree(
                        &repository,
                        &entry,
                        root,
                    ) {
                        Ok(()) => screen.open_repository(entry.path),
                        Err(error) => {
                            screen.message = crate::message::Message::error(format!("{error:#}"));
                        }
                    }
                }
                Action::WorktreeRemove => {
                    let mut draft = screen.mutation_draft(action, Some(repository));
                    draft.selection = selection;
                    draft.return_to = return_to;
                    screen.prepare_mutation(
                        engine::operations::mutation::Request::Worktree(
                            engine::local::worktrees::Request::Remove(*entry),
                        ),
                        draft,
                    );
                }
                _ => unreachable!("worktree entry command"),
            },
            Request::Local { path } => {
                if action != Action::Tasks {
                    screen.close_tasks();
                }
                return screen.activate_local(action, path, selection, return_to);
            }
            Request::Editor { root, target } => screen.open_editor_target(root, target, &permit),
            Request::Terminal { root, directory } => {
                screen.start_repository(
                    root.clone(),
                    i18n::tr!(OpenRepositoryTerminal),
                    Effect::Terminal,
                    move |service, cancel, _| {
                        service.open_terminal(&root, &directory, &cancel)?;
                        Ok(Outcome::TerminalOpened(directory))
                    },
                );
            }
            Request::Folder(target) => screen.request_external_open(std::sync::Arc::new(target)),
            Request::Copy(payload) => screen.submit_clipboard(Ok(payload)),
            Request::Replay {
                repository,
                commits,
            } => {
                let Action::Replay(kind) = action else {
                    unreachable!("replay command target")
                };
                let mut draft = screen.mutation_draft(action, Some(repository));
                draft.selection = selection;
                draft.return_to = return_to;
                screen.prepare_replay(kind, commits, draft, &permit);
            }
            Request::ResetBranch {
                mode,
                repository,
                target,
            } => {
                let mut draft = screen.mutation_draft(Action::ResetMenu, Some(repository));
                draft.input.replace_canonical(
                    crate::command::named::RESET_MODE.value_editor(mode.token()),
                );
                draft.selection = selection;
                draft.return_to = return_to;
                screen.prepare_mutation(
                    engine::operations::mutation::Request::Reset { target, mode },
                    draft,
                );
            }
            Request::Refresh(target) => screen.refresh_target(target, root, &permit),
            Request::Index(request) => request.execute(screen),
            Request::Discard(plan) => plan.review(screen),
            Request::DiscardFiles { snapshot, changes } => {
                screen.prepare_discard_files(snapshot, changes, &permit)
            }
            Request::MoreHistory(graph) => screen.more_history_target(graph, &permit),
            Request::HistoryParent(commit) => screen.cycle_history_parent_target(commit, &permit),
            Request::BrowserDiff { root, entry } => {
                screen.open_browser_diff_target(root, entry, &permit)
            }
            Request::StashPreview {
                scope,
                session,
                entry,
                previous,
            } => screen.open_stash_detail_target(scope, session, entry, Some(previous), &permit),
            Request::ConflictAccept {
                action,
                repository,
                resolution,
            } => screen.accept_conflict_target(action, repository, resolution, &permit),
            Request::ConflictReset {
                repository,
                resolution,
            } => screen.reset_conflict_target(repository, resolution, &permit),
            Request::Presentation(target) => {
                if let Mode::Browser(session) = &mut screen.mode {
                    match action {
                        Action::BrowserLayout(layout) => {
                            session.present(&target, target.presentation.with_layout(layout))
                        }
                        Action::BrowserMode(mode) => session.open_with(&target, mode),
                        _ => unreachable!("file presentation command target"),
                    }
                    screen.poll_browser();
                }
            }
        }
        Ok(false)
    }
}
