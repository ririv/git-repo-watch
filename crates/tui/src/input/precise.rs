//! Apply selections from rendered hit regions without executing repository operations.
use crate::{
    Mode, Page, Screen, branches, changes, document, history,
    pointer::{Action, ScrollTarget, Target},
    stashes,
};

impl Screen {
    pub(super) fn precise_action(&mut self, action: Action) -> bool {
        match action {
            Action::DragRows {
                target,
                index,
                offset,
            } => {
                if !self.precise_action(Action::Select(target, index)) {
                    return false;
                }
                match target {
                    Target::Repositories => *self.repository_viewport.offset_mut() = offset,
                    Target::Files => {
                        let files = match &mut self.mode {
                            Mode::Files(files) => files,
                            Mode::Changes(session) => &mut session.files,
                            _ => return false,
                        };
                        let staged =
                            files.rows()[files.cursor].area == engine::local::files::Area::Staged;
                        files.offsets[usize::from(staged)] = offset;
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
                }
            }

            Action::ToggleFile(index) => {
                let files = match &mut self.mode {
                    Mode::Files(files) => files,
                    Mode::Changes(session) => &mut session.files,
                    _ => return false,
                };
                let Some(change) = files.rows().get(index).copied() else {
                    return false;
                };
                let identity = files.change_index(change);
                files.cursor = index;
                if !files.selected.remove(&identity) {
                    files.selected.insert(identity);
                }
                if matches!(self.mode, Mode::Changes(_)) {
                    self.poll_changes();
                }
            }

            Action::Select(target, index) => match target {
                Target::Repositories if self.page == Page::Repositories => self.select(index),
                Target::Settings if matches!(self.page, Page::Configuration(_)) => {
                    self.setting_selected = index
                }
                Target::Files => match &mut self.mode {
                    Mode::Files(files) if index < files.rows().len() => files.cursor = index,
                    Mode::Changes(session) if index < session.files.rows().len() => {
                        session.files.cursor = index;
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
                ScrollTarget::Overview if self.page == Page::Detail => self.scroll = offset,
                ScrollTarget::HistoryMessage => match &mut self.mode {
                    Mode::History(view) => match view.as_mut() {
                        history::View::Commit(commit) => commit.scroll = offset,
                        _ => return false,
                    },
                    _ => return false,
                },
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
                ScrollTarget::Text => match &mut self.mode {
                    Mode::Branches(view) => match view.as_mut() {
                        branches::View::Detail { scroll, .. } => *scroll = offset,
                        _ => return false,
                    },
                    Mode::Stashes(view) => match view.as_mut() {
                        stashes::View::Detail { scroll, .. } => *scroll = offset,
                        _ => return false,
                    },
                    Mode::CommitMessage { scroll, .. }
                    | Mode::CommitConfirm { scroll, .. }
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
