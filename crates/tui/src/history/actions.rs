//! Background loading returns complete UI states without touching repository contents or reports.
use super::*;
use crate::{Effect, Mode, Outcome, Screen};
use engine::local::history::{Parent, Request};
use std::path::PathBuf;

impl Screen {
    pub(crate) fn mark_history(&mut self, action: SelectionAction) {
        let result = match &mut self.mode {
            Mode::History(view) => match view.as_mut() {
                View::Graph(session) => session.mark(action),
                _ => return,
            },
            _ => return,
        };
        if let Err(error) = result {
            self.message = crate::message::Message::error(format!("{error:#}"));
        }
    }
    pub(crate) fn open_history(&mut self, path: PathBuf) {
        self.start_repository(
            path.clone(),
            i18n::tr!(HistoryRead),
            Effect::ReadOnly,
            move |service, cancel, _| {
                let page = service.history(&path, Request::Start, &cancel)?;
                Ok(Outcome::History(Box::new(View::Graph(Session::new(
                    page, &cancel,
                )?))))
            },
        );
    }
    pub(crate) fn more_history(&mut self) {
        let path = self.scope_path();
        let selection = self.copy_selection();
        if let Err(error) = self.activate(
            crate::Action::LogMore,
            path,
            selection,
            &engine::cancel::Cancel::default(),
        ) {
            self.message = crate::message::Message::error(format!("{error:#}"));
        }
    }
    pub(crate) fn more_history_target(
        &mut self,
        mut session: Session,
        _permit: &crate::command::target::ExecutionPermit,
    ) {
        let Some(cursor) = session.cursor() else {
            self.message = crate::message::Message::information(i18n::tr!(HistoryNoMore).into());
            return;
        };
        self.start_repository(
            session.references.path.clone(),
            i18n::tr!(HistoryRead),
            Effect::ReadOnly,
            move |service, cancel, _| {
                let page = service.history(
                    &session.references.path,
                    Request::Continue(cursor),
                    &cancel,
                )?;
                session.append(page, &cancel)?;
                Ok(Outcome::History(Box::new(View::Graph(session))))
            },
        );
    }
    pub(crate) fn open_history_commit(&mut self) {
        let Mode::History(view) = &self.mode else {
            return;
        };
        let session = view.session().clone();
        let Some(entry) = session.selected() else {
            return;
        };
        let oid = entry.oid;
        self.start_repository(
            session.references.path.clone(),
            i18n::tr!(HistoryReadCommit),
            Effect::ReadOnly,
            move |service, cancel, _| {
                let detail = service.history_commit(
                    &session.references.path,
                    oid,
                    Parent::First,
                    &cancel,
                )?;
                Ok(Outcome::History(Box::new(View::Commit(Commit::new(
                    session, detail,
                )))))
            },
        );
    }
    pub(crate) fn cycle_history_parent_target(
        &mut self,
        commit: Commit,
        _permit: &crate::command::target::ExecutionPermit,
    ) {
        let crate::inspection::Source::Commit(detail) = &commit.inspect.detail else {
            unreachable!("history commit source")
        };
        if detail.parents.len() < 2 {
            return;
        }
        let index = detail
            .parents
            .iter()
            .position(|oid| engine::local::history::Base::Commit(*oid) == detail.base)
            .expect("merge comparison belongs to a parent");
        let parent = detail.parents[(index + 1) % detail.parents.len()];
        let oid = detail.oid;
        let session = commit.graph.clone();
        let previous = commit.clone();
        self.start_repository(
            session.references.path.clone(),
            i18n::tr!(HistoryReadCommit),
            Effect::ReadOnly,
            move |service, cancel, _| {
                let detail = service.history_commit(
                    &session.references.path,
                    oid,
                    Parent::Commit(parent),
                    &cancel,
                )?;
                Ok(Outcome::History(Box::new(View::Commit(
                    previous.with_detail(detail),
                ))))
            },
        );
    }
    pub(crate) fn show_history_diff(&mut self) {
        let Mode::History(view) = &self.mode else {
            return;
        };
        let commit = match view.as_ref() {
            View::Commit(commit) => commit.clone(),
            View::Diff { commit, .. } => (**commit).clone(),
            View::Graph(_) => {
                self.message =
                    crate::message::Message::error(i18n::tr!(HistoryOpenCommitFirst).into());
                return;
            }
        };
        let Some(index) = commit.inspect.selected_index() else {
            self.message = crate::message::Message::information(i18n::tr!(HistoryNoChanges).into());
            return;
        };
        let theme = self.background.syntax_theme();
        self.start_repository(
            commit.inspect.detail.path().to_owned(),
            i18n::tr!(HistoryReadDiff),
            Effect::ReadOnly,
            move |service, cancel, _| {
                let content = commit
                    .inspect
                    .prepare_file(&service, index, theme, &cancel)?;
                Ok(Outcome::History(Box::new(View::Diff {
                    commit: Box::new(commit),
                    content,
                    scroll: 0,
                })))
            },
        );
    }
}
