//! Apply selections from rendered hit regions without executing repository operations.
use crate::{
    Mode, Page, Screen, branches, changes, history,
    pointer::{Action, ScrollTarget, Target},
    stashes,
};

impl Screen {
    pub(super) fn precise_action(&mut self, action: Action) -> bool {
        if !matches!(action, Action::ReplayControl(_)) {
            self.close_replay_controls();
        }
        match action {
            Action::DiffLine(id) => {
                let Mode::Changes(session) = &mut self.mode else {
                    return false;
                };
                if session.focus != changes::Focus::Preview && !self.config.mouse.select_pane {
                    return false;
                }
                let changes::Preview::Ready {
                    editing: changes::edit::State::Ready(editing),
                    ..
                } = &mut session.preview
                else {
                    return false;
                };
                editing.choose(id);
                session.focus = changes::Focus::Preview;
            }
            Action::Management(action) => {
                self.management_action(action);
            }
            Action::ManagementToggle(index) => {
                let Mode::Management(session) = &mut self.mode else {
                    return false;
                };
                session.state.select(Some(index));
                session.toggle(index);
            }
            Action::BrowserDirectory(depth) => {
                let Mode::Browser(session) = &mut self.mode else {
                    return false;
                };
                if !session.visit_directory(depth) {
                    return false;
                }
                self.poll_browser();
            }
            Action::PushRemote(index) => {
                let Mode::PushTarget(session) = &mut self.mode else {
                    return false;
                };
                session.choose(index);
            }
            Action::HistoryMenu(input) => return self.history_menu_pointer(input),
            Action::ToggleCommit(index) => {
                let Mode::History(view) = &mut self.mode else {
                    return false;
                };
                let history::View::Graph(session) = view.as_mut() else {
                    return false;
                };
                if !session.selection.active() || index >= session.records.len() {
                    return false;
                }
                session.state.select(Some(index));
                self.mark_history(history::SelectionAction::Toggle);
            }
            Action::ReplayControl(control) => {
                self.preview_replay_control(control);
            }
            Action::RepositoryMenu(menu) => {
                if !self.repository_list_active() {
                    return false;
                }
                self.list_menu(menu);
            }
            Action::DragRows {
                target,
                index,
                offset,
            } => {
                if !self.precise_action(Action::Select(target, index)) {
                    return false;
                }
                match target {
                    Target::Management => {
                        if let Mode::Management(session) = &mut self.mode {
                            *session.state.offset_mut() = offset;
                        } else {
                            return false;
                        }
                    }
                    Target::CompositionCommits | Target::CompositionUnits => {
                        if let Mode::Composition(session) = &mut self.mode {
                            match target {
                                Target::CompositionCommits => {
                                    *session.commits.offset_mut() = offset
                                }
                                Target::CompositionUnits => *session.units.offset_mut() = offset,
                                _ => unreachable!(),
                            }
                        }
                    }
                    Target::Browser => {
                        if let Mode::Browser(session) = &mut self.mode {
                            *session.state.offset_mut() = offset;
                        }
                    }
                    Target::Repositories => *self.repository_viewport.offset_mut() = offset,
                    Target::Files => {
                        let files = match &mut self.mode {
                            Mode::Files(files) => files,
                            Mode::Changes(session) => &mut session.files,
                            _ => return false,
                        };
                        files.offset = offset;
                    }
                    Target::Branches => {
                        if let Mode::Branches(view) = &mut self.mode
                            && let branches::View::List(session) = view.as_mut()
                        {
                            *session.state.offset_mut() = offset;
                        }
                    }
                    Target::Stashes => {
                        if let Mode::Stashes(view) = &mut self.mode
                            && let stashes::View::List(session) = view.as_mut()
                        {
                            *session.state.offset_mut() = offset;
                        }
                    }
                    Target::History => {
                        if let Mode::History(view) = &mut self.mode
                            && let history::View::Graph(session) = view.as_mut()
                        {
                            *session.state.offset_mut() = offset;
                        }
                    }
                    Target::InspectionFiles => {
                        if let Some(inspect) = self.mode.inspection_mut() {
                            *inspect.files.offset_mut() = offset;
                        }
                    }
                    Target::Settings => self.scroll = offset,
                    Target::ReplayParent
                    | Target::ScheduleButton
                    | Target::WorkflowSteps
                    | Target::WorkflowToggle
                    | Target::WorkflowActions => {
                        return false;
                    }
                }
            }

            Action::ToggleFile(index) => {
                let files = match &mut self.mode {
                    Mode::Files(files) => files,
                    Mode::Changes(session) => &mut session.files,
                    _ => return false,
                };
                if !files.select_row(index, self.file_layout) {
                    return false;
                }
                files.toggle_current();
                if matches!(self.mode, Mode::Changes(_)) {
                    self.poll_changes();
                }
            }

            Action::Select(target, index) => match target {
                Target::Management => {
                    let Mode::Management(session) = &mut self.mode else {
                        return false;
                    };
                    if index >= session.rows().len() {
                        return false;
                    }
                    session.state.select(Some(index));
                    session.focus = crate::management::Focus::List;
                }

                Target::ReplayParent => {
                    let Mode::Confirm {
                        operation:
                            crate::Confirmation::ReplayParents {
                                candidate,
                                selected,
                                ..
                            },
                        ..
                    } = &mut self.mode
                    else {
                        return false;
                    };
                    if index >= candidate.parents.len() {
                        return false;
                    }
                    *selected = index;
                }
                Target::ScheduleButton => return false,
                Target::WorkflowSteps | Target::WorkflowToggle | Target::WorkflowActions => {
                    if !matches!(self.mode, Mode::Workflow(_)) {
                        return false;
                    }
                    let Mode::Workflow(mut session) = self.take_mode() else {
                        return false;
                    };
                    let result = match target {
                        Target::WorkflowSteps if session.select(index) => {
                            Ok(crate::workflow::Input::Pending)
                        }
                        Target::WorkflowToggle if index < 3 && session.select(index) => {
                            session.apply(crate::workflow::Action::Toggle)
                        }
                        Target::WorkflowActions if index < crate::workflow::Action::ALL.len() => {
                            session.apply(crate::workflow::Action::ALL[index])
                        }
                        _ => {
                            self.mode = Mode::Workflow(session);
                            return false;
                        }
                    };
                    self.workflow_input(session, result);
                }
                Target::CompositionCommits | Target::CompositionUnits => {
                    let Mode::Composition(session) = &mut self.mode else {
                        return false;
                    };
                    if !matches!(session.state, crate::composition::State::Browsing) {
                        return false;
                    }
                    match target {
                        Target::CompositionCommits
                            if index < session.prepared.plan.commits.len() =>
                        {
                            session.select(index)
                        }
                        Target::CompositionUnits if index < session.ids().len() => {
                            session.unit = index;
                            session.scroll = 0;
                        }
                        _ => return false,
                    }
                }
                Target::Browser => {
                    if let Mode::Browser(session) = &mut self.mode {
                        session.select(index);
                        self.poll_browser();
                    } else {
                        return false;
                    }
                }
                Target::Repositories if self.page == Page::Repositories => self.select(index),
                Target::Settings if matches!(self.page, Page::Configuration(_)) => {
                    self.setting_selected = index
                }
                Target::Files => match &mut self.mode {
                    Mode::Files(files) => {
                        if !files.select_row(index, self.file_layout) {
                            return false;
                        }
                    }
                    Mode::Changes(session) => {
                        if !session.files.select_row(index, self.file_layout) {
                            return false;
                        }
                        self.poll_changes();
                    }
                    _ => return false,
                },
                Target::Branches => match &mut self.mode {
                    Mode::Branches(view) => match view.as_mut() {
                        branches::View::List(session) if index < session.rows().len() => {
                            session.state.select(Some(index))
                        }
                        _ => return false,
                    },
                    _ => return false,
                },
                Target::Stashes => match &mut self.mode {
                    Mode::Stashes(view) => match view.as_mut() {
                        stashes::View::List(session) if index < session.rows().len() => {
                            session.state.select(Some(index))
                        }
                        _ => return false,
                    },
                    _ => return false,
                },
                Target::InspectionFiles => {
                    let Some(inspect) = self.mode.inspection_mut() else {
                        return false;
                    };
                    if index >= inspect.rows().len() {
                        return false;
                    }
                    inspect.files.select(Some(index));
                }
                Target::History => match &mut self.mode {
                    Mode::History(view) => match view.as_mut() {
                        history::View::Graph(session) if index < session.records.len() => {
                            session.selection.settle();
                            session.state.select(Some(index));
                        }
                        _ => return false,
                    },
                    _ => return false,
                },
                _ => return false,
            },
            Action::Scroll(target, offset) => match target {
                ScrollTarget::Files => {
                    let files = match &mut self.mode {
                        Mode::Files(files) => files,
                        Mode::Changes(session) => &mut session.files,
                        _ => return false,
                    };
                    files.seek(offset, self.file_layout);
                    if matches!(self.mode, Mode::Changes(_)) {
                        self.poll_changes();
                    }
                }
                ScrollTarget::BrowserTreeHorizontal => {
                    let Mode::Browser(session) = &mut self.mode else {
                        return false;
                    };
                    session.horizontal.seek(offset);
                }
                ScrollTarget::BrowserPreview(mode) | ScrollTarget::BrowserHorizontal(mode) => {
                    if let Mode::Browser(session) = &mut self.mode {
                        match target {
                            ScrollTarget::BrowserPreview(_) => {
                                session.panes.get_mut(mode).position.scroll = offset
                            }
                            ScrollTarget::BrowserHorizontal(_) => {
                                session.panes.get_mut(mode).position.horizontal.seek(offset)
                            }
                            _ => unreachable!(),
                        }
                    } else {
                        return false;
                    }
                }
                ScrollTarget::HistoryActions => return self.seek_history_action(offset),
                ScrollTarget::Overview if self.page == Page::Detail => self.scroll = offset,
                ScrollTarget::InspectionMessage
                | ScrollTarget::InspectionReview
                | ScrollTarget::InspectionReviewHorizontal => {
                    let Some(inspect) = self.mode.inspection_mut() else {
                        return false;
                    };
                    match target {
                        ScrollTarget::InspectionMessage => inspect.scroll = offset,
                        ScrollTarget::InspectionReview => inspect.review.seek(offset),
                        ScrollTarget::InspectionReviewHorizontal => {
                            inspect.review.horizontal.seek(offset)
                        }
                        _ => unreachable!(),
                    }
                }
                ScrollTarget::HistoryHorizontal => match &mut self.mode {
                    Mode::History(view) => match view.as_mut() {
                        history::View::Graph(session) => session.horizontal.seek(offset),
                        _ => return false,
                    },
                    _ => return false,
                },
                ScrollTarget::Conflict(focus) | ScrollTarget::ConflictHorizontal(focus) => {
                    let Mode::Changes(session) = &mut self.mode else {
                        return false;
                    };
                    let changes::Preview::Ready {
                        content, scroll, ..
                    } = &mut session.preview
                    else {
                        return false;
                    };
                    let crate::document::View::Conflict(view) = content.as_mut() else {
                        return false;
                    };
                    if (session.focus != changes::Focus::Preview || view.focus() != focus)
                        && !self.config.mouse.select_pane
                    {
                        return false;
                    }
                    session.focus = changes::Focus::Preview;
                    view.select(focus, scroll);
                    match target {
                        ScrollTarget::Conflict(_) => *scroll = offset,
                        ScrollTarget::ConflictHorizontal(_) => {
                            if let Some(horizontal) = content.horizontal() {
                                horizontal.seek(offset);
                            }
                        }
                        _ => unreachable!(),
                    }
                }
                ScrollTarget::Diff | ScrollTarget::DiffHorizontal => {
                    let (content, scroll) = match &mut self.mode {
                        Mode::Changes(session) => match &mut session.preview {
                            changes::Preview::Ready {
                                content, scroll, ..
                            } => (content, scroll),
                            _ => return false,
                        },
                        Mode::Stashes(view) => match view.as_mut() {
                            stashes::View::Diff {
                                content, scroll, ..
                            } => (content, scroll),
                            _ => return false,
                        },
                        Mode::History(view) => match view.as_mut() {
                            history::View::Diff {
                                content, scroll, ..
                            } => (content, scroll),
                            _ => return false,
                        },
                        _ => return false,
                    };
                    match target {
                        ScrollTarget::Diff => *scroll = offset,
                        ScrollTarget::DiffHorizontal => match content.horizontal() {
                            Some(view) => view.seek(offset),
                            None => return false,
                        },
                        _ => unreachable!(),
                    }
                }
                ScrollTarget::CommitMessage => match &mut self.mode {
                    Mode::CommitMessage { editor, .. } => editor.display().scroll = offset,
                    Mode::Composition(session) => match &mut session.state {
                        crate::composition::State::Editing(editor) => editor.scroll = offset,
                        _ => session.message_scroll = offset,
                    },
                    _ => return false,
                },
                ScrollTarget::Text => match &mut self.mode {
                    Mode::StashSave(session) => session.scroll = offset,
                    Mode::Workflow(session) => session.scroll = offset,
                    Mode::CompositionWaiting(waiting) => match &mut waiting.state {
                        crate::composition::waiting::State::Failed { scroll, .. } => {
                            *scroll = offset
                        }
                        _ => return false,
                    },
                    Mode::PromptEditor(session) => session.editor.scroll = offset,
                    Mode::Branches(view) => match view.as_mut() {
                        branches::View::Detail { scroll, .. } => *scroll = offset,
                        _ => return false,
                    },
                    Mode::Composition(session) => session.scroll = offset,
                    Mode::CommitMessage { scroll, .. }
                    | Mode::PushConfirm { scroll, .. }
                    | Mode::Error { scroll, .. } => *scroll = offset,
                    Mode::Browsing if matches!(self.page, Page::Help | Page::Configuration(_)) => {
                        self.scroll = offset
                    }
                    _ => return false,
                },
                _ => return false,
            },
        }
        true
    }
}
