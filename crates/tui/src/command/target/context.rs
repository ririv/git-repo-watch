//! Build command dependencies from the current view and the captured command selection.
use super::{Requirement, Resolved, Target, resolved::Value};
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
            .map(Resolved::new)
    }
    fn provide(&self, action: Action) -> Target<Value<'a>> {
        use Requirement::*;
        match action.requirement() {
            Global => Target::Ready(Value::Global),
            Repository => self.repository().map(Value::Repository),
            EditorDestination => self.editor().map(Value::Destination),
            TerminalDirectory => self.terminal().map(Value::Destination),
            Clipboard => self.clipboard().map(Value::Clipboard),
            FilePath => self.file().map(Value::File),
            Search => self.search(action).map(Value::Search),
            LocalRefresh => self.refresh().map(Value::Refresh),
            IndexSelection => self.index(action).map(Value::Index),
            DiffSelection => self.partial(action).map(Value::Partial),
            WorktreeFiles => self.worktree_files().map(Value::WorktreeFiles),
            Conflict => self.conflict().map(Value::Conflict),
            BranchList => self.branches().map(Value::Branches),
            HistoryGraph => self.history().map(Value::History),
            HistorySelection => self.history_selection().map(Value::History),
            CommitSelection => self.commits().map(Value::Commits),
            MergeCommit => self.merge_commit().map(Value::MergeCommit),
            StashDetail => self.stash().map(Value::Stash),
            Browser => self.browser().map(Value::Browser),
            PreviewableFile => self.previewable().map(Value::Previewable),
            Composition => self.composition(action).map(Value::Composition),
        }
    }
}
