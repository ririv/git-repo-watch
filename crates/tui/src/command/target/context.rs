//! Build command dependencies from the current view and the captured command selection.
use super::{Requirement, Resolved, Target};
use crate::{Action, Screen, clipboard};
use std::path::Path;

pub(crate) struct CommandContext<'a> {
    pub(super) screen: &'a Screen,
    pub(super) repository: Option<&'a Path>,
    pub(super) selection: &'a clipboard::Selection,
}
impl<'a> CommandContext<'a> {
    pub fn new(
        screen: &'a Screen,
        repository: Option<&'a Path>,
        selection: &'a clipboard::Selection,
    ) -> Self {
        Self {
            screen,
            repository,
            selection,
        }
    }
    pub fn resolve(&self, action: Action) -> Target<Resolved<'a>> {
        self.provide(action)
            .and_then(|target| self.validate(action, target))
    }
    fn provide(&self, action: Action) -> Target<Resolved<'a>> {
        use Requirement::*;
        match action.requirement() {
            Global => Target::Ready(Resolved::Global),
            Repository => self.repository().map(Resolved::Repository),
            EditorDestination => self.editor().map(Resolved::Destination),
            TerminalDirectory => self.terminal().map(Resolved::Destination),
            Clipboard => self.clipboard().map(Resolved::Clipboard),
            FilePath => self.file().map(Resolved::File),
            Search => self.search(action).map(Resolved::Search),
            LocalRefresh => self.refresh().map(Resolved::Refresh),
            IndexSelection => self.index(action).map(Resolved::Index),
            DiffSelection => self.partial(action).map(Resolved::Partial),
            WorktreeFiles => self.worktree_files().map(Resolved::WorktreeFiles),
            Conflict => self.conflict().map(Resolved::Conflict),
            BranchList => self.branches().map(Resolved::Branches),
            HistoryGraph => self.history().map(Resolved::History),
            HistorySelection => self.history_selection().map(Resolved::History),
            CommitSelection => self.commits().map(Resolved::Commits),
            MergeCommit => self.merge_commit().map(Resolved::MergeCommit),
            StashDetail => self.stash().map(Resolved::Stash),
            Browser => self.browser().map(Resolved::Browser),
            PreviewableFile => self.previewable().map(Resolved::Previewable),
            Composition => self.composition(action).map(Resolved::Composition),
        }
    }
}
