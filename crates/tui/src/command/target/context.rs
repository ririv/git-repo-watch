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
    fn task(&self, action: Action) -> Target<crate::task_center::Operation> {
        if !self.screen.tasks.session.open() {
            return Target::OutOfScope(i18n::tr!(TaskCommandRequiresView).into());
        }
        use crate::task_center::Operation;
        match action {
            Action::TaskFilterMenu => return Target::Ready(Operation::FilterMenu),
            Action::TaskFilter(filter) => return Target::Ready(Operation::Filter(filter)),
            _ => {}
        }
        let clipboard::Selection::Task(id) = self.selection else {
            return Target::Unavailable(i18n::tr!(TaskCommandRequiresSelection).into());
        };
        let Some(record) = self
            .screen
            .tasks
            .records
            .iter()
            .find(|record| &record.id == id)
        else {
            return Target::Unavailable(i18n::tr!(TaskCommandRequiresSelection).into());
        };
        match crate::task_center::Operation::prepare(action, record) {
            Ok(operation) => Target::Ready(operation),
            Err(reason) => Target::Unavailable(reason.into()),
        }
    }
    fn provide(&self, action: Action) -> Target<Value<'a>> {
        use Requirement::*;
        let requirement = action.requirement();
        if self.screen.tasks.session.open() {
            match requirement {
                Global if matches!(action, Action::Tasks | Action::Quit) => {}
                Global if matches!(action, Action::Focus | Action::Zoom) => {
                    return Target::OutOfScope(i18n::tr!(PaneUnavailable).into());
                }
                Global if matches!(self.screen.mode, crate::Mode::Busy(_)) => {
                    return Target::Unavailable(i18n::tr!(TaskCommandBusy).into());
                }
                Global | TaskView | LocalRefresh => {}
                _ => return Target::OutOfScope(i18n::tr!(TaskCommandOutsideView).into()),
            }
        }
        match requirement {
            TaskView => self.task(action).map(Value::Task),
            Global => Target::Ready(Value::Global),
            Repository => self.repository().map(Value::Repository),
            EditorDestination => self.editor().map(Value::Destination),
            DirectoryDestination => {
                let unavailable = match action {
                    Action::Terminal => i18n::tr!(TerminalSelectionRequired),
                    Action::OpenMenu | Action::OpenFolder => i18n::tr!(FolderSelectionRequired),
                    _ => unreachable!("directory command"),
                };
                self.directory(unavailable).map(Value::Destination)
            }
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
