//! Background loading returns complete UI states without touching repository contents or reports.
use super::*;
use crate::{Effect, Mode, Outcome, Screen};
use engine::local::history::{Parent, Request};
use std::path::PathBuf;

impl Screen {
    pub(crate) fn open_history(&mut self, path: PathBuf) {
        self.start(
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
        let Mode::History(view) = &self.mode else {
            self.message = i18n::tr!(HistoryRequiresOpen).into();
            return;
        };
        let mut session = view.session().clone();
        let Some(cursor) = session.cursor() else {
            self.message = i18n::tr!(HistoryNoMore).into();
            return;
        };
        self.start(
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
        self.start(
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
    pub(crate) fn cycle_history_parent(&mut self) {
        let Mode::History(view) = &self.mode else {
            return;
        };
        let View::Commit(commit) = view.as_ref() else {
            return;
        };
        let detail = &commit.detail;
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
        self.start(
            i18n::tr!(HistoryReadCommit),
            Effect::ReadOnly,
            move |service, cancel, _| {
                let detail = service.history_commit(
                    &session.references.path,
                    oid,
                    Parent::Commit(parent),
                    &cancel,
                )?;
                Ok(Outcome::History(Box::new(View::Commit(Commit::new(
                    session, detail,
                )))))
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
                self.message = i18n::tr!(HistoryOpenCommitFirst).into();
                return;
            }
        };
        let Some(change) = commit
            .files
            .selected()
            .and_then(|index| commit.rows().get(index).map(|change| (*change).clone()))
        else {
            self.message = i18n::tr!(HistoryNoChanges).into();
            return;
        };
        self.start(
            i18n::tr!(HistoryReadDiff),
            Effect::ReadOnly,
            move |service, cancel, _| {
                let document = service.history_diff(&commit.detail, &change, &cancel)?;
                let content = crate::document::prepare_history(
                    &change.location.display(),
                    document,
                    commit.detail.base,
                    commit.detail.oid,
                    &cancel,
                )?;
                Ok(Outcome::History(Box::new(View::Diff {
                    commit: Box::new(commit),
                    content,
                    scroll: 0,
                })))
            },
        );
    }
}
