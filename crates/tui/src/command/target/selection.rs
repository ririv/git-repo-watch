//! List selection commands edit the marks owned by their resolved view.
use super::{CommandContext, ExecutionPermit, Target};
use crate::{
    FileSession, Mode, Screen, changes, clipboard, command::SelectionAction, files::MarkEdit,
    history,
};
use anyhow::{Result, anyhow, ensure};
use engine::local::{files::Files, history::Oid};
use std::sync::Arc;

pub(super) enum Selection<'a> {
    History(&'a history::Session),
    Files(&'a FileSession),
}

pub(super) enum HistoryEdit {
    Toggle(Oid),
    All,
    Clear,
}

pub(super) enum Prepared {
    History {
        source: u64,
        edit: HistoryEdit,
    },
    Files {
        snapshot: Arc<Files>,
        edit: MarkEdit,
    },
}

impl<'a> CommandContext<'a> {
    pub(super) fn list_selection(&self, action: SelectionAction) -> Target<Selection<'a>> {
        match self.selection {
            clipboard::Selection::Log(
                clipboard::CommitSelection::Graph { .. }
                | clipboard::CommitSelection::Unavailable(_),
            ) => {
                let Some(graph) = self.screen.history_graph_target() else {
                    return Target::OutOfScope(i18n::tr!(CommandRequiresSelectableList).into());
                };
                if Some(graph.references.path.as_path()) != self.repository {
                    return Target::Unavailable(i18n::tr!(CommandSelectionChanged).into());
                }
                if action != SelectionAction::Clear && self.selection.commit().is_none() {
                    return Target::Unavailable(i18n::tr!(ReplaySelectCommit).into());
                }
                if action == SelectionAction::All
                    && graph.records.len() > engine::operations::replay::batch::MAX_COMMITS
                {
                    return Target::Unavailable(i18n::tr!(ReplayBatchTooLarge).into());
                }
                Target::Ready(Selection::History(graph))
            }
            clipboard::Selection::Diff { .. } => {
                let Some(files) = self.screen.worktree_file_list() else {
                    return Target::OutOfScope(i18n::tr!(CommandRequiresSelectableList).into());
                };
                if Some(files.snapshot.path.as_path()) != self.repository {
                    return Target::Unavailable(i18n::tr!(CommandSelectionChanged).into());
                }
                if action != SelectionAction::Clear && files.group().is_none() {
                    return Target::Unavailable(i18n::tr!(NoActionableFilesInThisArea).into());
                }
                Target::Ready(Selection::Files(files))
            }
            _ => Target::OutOfScope(i18n::tr!(CommandRequiresSelectableList).into()),
        }
    }
}

impl Selection<'_> {
    pub(super) fn prepare(
        self,
        action: SelectionAction,
        selection: &clipboard::Selection,
    ) -> Result<Prepared> {
        let changed = || anyhow!(i18n::tr!(CommandSelectionChanged));
        Ok(match self {
            Self::History(graph) => Prepared::History {
                source: graph.source,
                edit: match action {
                    SelectionAction::Toggle => {
                        let oid = selection.commit().ok_or_else(changed)?;
                        ensure!(
                            graph.records.iter().any(|row| row.entry.oid == oid),
                            i18n::tr!(CommandSelectionChanged)
                        );
                        HistoryEdit::Toggle(oid)
                    }
                    SelectionAction::All => HistoryEdit::All,
                    SelectionAction::Clear => HistoryEdit::Clear,
                },
            },
            Self::Files(session) => Prepared::Files {
                snapshot: session.snapshot.clone(),
                edit: match action {
                    SelectionAction::Toggle => {
                        MarkEdit::Toggle(session.current_mark_targets().ok_or_else(changed)?)
                    }
                    SelectionAction::All => {
                        MarkEdit::Add(session.visible_mark_targets().ok_or_else(changed)?)
                    }
                    SelectionAction::Clear => MarkEdit::Clear,
                },
            },
        })
    }
}

impl Prepared {
    pub(super) fn execute(self, screen: &mut Screen, _permit: &ExecutionPermit) -> Result<()> {
        let changed = || anyhow!(i18n::tr!(CommandSelectionChanged));
        match self {
            Self::History { source, edit } => {
                let Mode::History(view) = &mut screen.mode else {
                    return Err(changed());
                };
                let history::View::Graph(graph) = view.as_mut() else {
                    return Err(changed());
                };
                ensure!(graph.source == source, i18n::tr!(CommandSelectionChanged));
                match edit {
                    HistoryEdit::Toggle(oid) => graph.toggle_mark(oid),
                    HistoryEdit::All => graph.mark(history::SelectionAction::All),
                    HistoryEdit::Clear => graph.mark(history::SelectionAction::Clear),
                }
            }
            Self::Files { snapshot, edit } => {
                let files = match &mut screen.mode {
                    Mode::Files(files) => files,
                    Mode::Changes(session) if session.focus == changes::Focus::Files => {
                        &mut session.files
                    }
                    _ => return Err(changed()),
                };
                ensure!(
                    Arc::ptr_eq(&files.snapshot, &snapshot),
                    i18n::tr!(CommandSelectionChanged)
                );
                files.edit_marks(edit);
                Ok(())
            }
        }
    }
}
