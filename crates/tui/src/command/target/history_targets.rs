//! History providers distinguish an absent view from an empty displayed selection.
use super::{CommandContext, Target, resolved::Commits};
use crate::{Mode, clipboard, history};

impl<'a> CommandContext<'a> {
    pub fn history(&self) -> Target<&'a history::Session> {
        match self.screen.history_graph_target() {
            Some(graph) => Target::Ready(graph),
            None => Target::OutOfScope(i18n::tr!(CommandRequiresHistory).into()),
        }
    }
    pub fn commits(&self) -> Target<Commits<'a>> {
        match self.selection {
            clipboard::Selection::Log(_) => match self.selection.commit() {
                Some(selected) => self.repository().map(|repository| Commits {
                    repository,
                    selected,
                    marked: self.selection.commits(),
                }),
                None => Target::Unavailable(i18n::tr!(ReplaySelectCommit).into()),
            },
            _ => Target::OutOfScope(i18n::tr!(CommandRequiresHistory).into()),
        }
    }
    pub fn merge_commit(&self) -> Target<&'a history::Commit> {
        match &self.screen.mode {
            Mode::History(view) => match view.as_ref() {
                history::View::Commit(commit) => match &commit.inspect.detail {
                    crate::inspection::Source::Commit(detail) if detail.parents.len() > 1 => {
                        Target::Ready(commit)
                    }
                    _ => Target::Unavailable(i18n::tr!(HistoryParentRequiresMerge).into()),
                },
                _ => Target::OutOfScope(i18n::tr!(HistoryOpenCommitFirst).into()),
            },
            _ => Target::OutOfScope(i18n::tr!(HistoryOpenCommitFirst).into()),
        }
    }
}
