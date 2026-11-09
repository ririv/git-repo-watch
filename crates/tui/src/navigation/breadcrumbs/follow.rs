//! Advertised destinations drive navigation and departure cleanup for every view.
use super::*;

impl Screen {
    pub(crate) fn follow_breadcrumb(&mut self, destination: Destination) {
        if !self
            .breadcrumbs()
            .iter()
            .any(|crumb| crumb.destination == Some(destination))
        {
            return;
        }
        self.view_revision += 1;
        self.message.clear();
        let level = match destination {
            Destination::Home => {
                while !self.history.is_empty() {
                    self.cancel_breadcrumb_generation(&self.mode);
                    self.return_to_view();
                }
                self.cancel_breadcrumb_generation(&self.mode);
                self.open_repository_list(self.scope_path());
                return;
            }
            Destination::Saved { index, level } => {
                while self.history.len() > index {
                    self.cancel_breadcrumb_generation(&self.mode);
                    self.return_to_view();
                }
                if level == Level::Page {
                    self.sync_bar();
                    return;
                }
                level
            }
            Destination::Current(level) => level,
        };
        if level == Level::Page {
            self.mode = match self.take_mode() {
                Mode::Error { origin, .. } => *origin,
                mode => mode,
            };
            self.sync_bar();
            return;
        }
        self.cancel_breadcrumb_generation(&self.mode);
        self.restore_breadcrumb_origin();
        match level {
            Level::Page => {}
            Level::PageRoot => match self.page {
                Page::View(view) => self.open_breadcrumb_view(view),
                Page::Detail => self.leave_repository_section(),
                Page::Repositories | Page::Configuration(_) | Page::Help => {
                    if !matches!(self.mode, Mode::Management(_)) {
                        self.mode = Mode::Browsing;
                    }
                }
            },
            Level::Repository => self.leave_repository_section(),
            Level::Pane(section) => self.switch_repository_section(section),
            Level::History => self.open_breadcrumb_view(View::History),
            Level::Commit => {
                if let Mode::History(view) = &self.mode
                    && let history::View::Diff { commit, .. } = view.as_ref()
                {
                    self.mode = Mode::History(Box::new(history::View::Commit((**commit).clone())));
                }
            }
            Level::Branches => self.open_breadcrumb_view(View::Branches),
            Level::Stashes => self.open_breadcrumb_view(View::Stashes),
            Level::StashEntry => {
                if let Mode::Stashes(view) = &self.mode
                    && let crate::stashes::View::Diff {
                        session, inspect, ..
                    } = view.as_ref()
                {
                    self.mode = Mode::Stashes(Box::new(crate::stashes::View::Detail {
                        session: session.clone(),
                        inspect: (**inspect).clone(),
                    }));
                }
            }
        }
        self.sync_bar();
    }

    fn open_breadcrumb_view(&mut self, view: View) {
        let path = self
            .scope_path()
            .expect("repository breadcrumb has a repository");
        self.open_view(view, path);
    }

    fn cancel_breadcrumb_generation(&self, mode: &Mode) {
        match mode {
            Mode::CommitMessage { .. } => self.commit_generator.cancel(),
            Mode::CompositionWaiting(waiting) => {
                if let Some(task) = self.repository_tasks.get(&waiting.path) {
                    task.cancel.cancel();
                }
            }
            Mode::Error { origin, .. } => self.cancel_breadcrumb_generation(origin),
            _ => {}
        }
    }

    fn restore_breadcrumb_origin(&mut self) {
        loop {
            self.mode = match self.take_mode() {
                Mode::Error { origin, .. } => *origin,
                Mode::CompositionWaiting(waiting) => *waiting.origin,
                Mode::StashSave(session) => *session.origin,
                Mode::Confirm { operation, .. } => match operation {
                    crate::Confirmation::DiscardFile { origin, .. }
                    | crate::Confirmation::Partial { origin, .. }
                    | crate::Confirmation::Remove { origin, .. }
                    | crate::Confirmation::Configure { origin, .. }
                    | crate::Confirmation::Mutation { origin, .. }
                    | crate::Confirmation::ReplayParents { origin, .. } => *origin,
                    crate::Confirmation::Schedule(_) => Mode::Browsing,
                },
                mode => {
                    self.mode = mode;
                    return;
                }
            };
        }
    }
}
