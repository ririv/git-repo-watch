//! Apply selections from rendered hit regions without executing repository operations.
use crate::{
    Mode, Page, Screen, branches, changes, document, history,
    pointer::{Action, ScrollTarget, Target},
    stashes,
};

impl Screen {
    pub(super) fn precise_action(&mut self, action: Action) -> bool {
        match action {
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
                        if let Some(group) = files.group() {
                            files.offsets[group.index()] = offset;
                        }
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
                    Target::HistoryFiles => {
                        if let Mode::History(view) = &mut self.mode
                            && let history::View::Commit(commit) = view.as_mut()
                        {
                            *commit.files.offset_mut() = offset;
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
                Target::History | Target::HistoryFiles => match &mut self.mode {
                    Mode::History(view) => match (view.as_mut(), target) {
                        (history::View::Graph(session), Target::History)
                            if index < session.records.len() =>
                        {
                            session.state.select(Some(index))
                        }
                        (history::View::Commit(commit), Target::HistoryFiles)
                            if index < commit.rows().len() =>
                        {
                            commit.files.select(Some(index))
                        }
                        _ => return false,
                    },
                    _ => return false,
                },
                _ => return false,
            },
            Action::Scroll(target, offset) => match target {
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
                ScrollTarget::Overview if self.page == Page::Detail => self.scroll = offset,
                ScrollTarget::HistoryMessage => match &mut self.mode {
                    Mode::History(view) => match view.as_mut() {
                        history::View::Commit(commit) => commit.scroll = offset,
                        _ => return false,
                    },
                    _ => return false,
                },
                ScrollTarget::HistoryReview | ScrollTarget::HistoryReviewHorizontal => {
                    match &mut self.mode {
                        Mode::History(view) => match view.as_mut() {
                            history::View::Commit(commit) => match target {
                                ScrollTarget::HistoryReview => commit.review.seek(offset),
                                ScrollTarget::HistoryReviewHorizontal => {
                                    commit.review.horizontal.seek(offset)
                                }
                                _ => unreachable!(),
                            },
                            _ => return false,
                        },
                        _ => return false,
                    }
                }
                ScrollTarget::HistoryHorizontal => match &mut self.mode {
                    Mode::History(view) => match view.as_mut() {
                        history::View::Graph(session) => session.horizontal.seek(offset),
                        _ => return false,
                    },
                    _ => return false,
                },
                ScrollTarget::Diff | ScrollTarget::DiffHorizontal => {
                    let (content, scroll) = match &mut self.mode {
                        Mode::Changes(session) => match &mut session.preview {
                            changes::Preview::Ready {
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
                        ScrollTarget::DiffHorizontal => match content.as_mut() {
                            document::View::Text(view) => view.horizontal.seek(offset),
                            _ => return false,
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
                    Mode::Stashes(view) => match view.as_mut() {
                        stashes::View::Detail { scroll, .. } => *scroll = offset,
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
