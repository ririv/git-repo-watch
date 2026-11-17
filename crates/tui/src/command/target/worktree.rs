//! Worktree targets bind displayed snapshots to captured files or partial selections.
use super::{CommandContext, IndexSelection, Reason, Target};
use crate::{Action, changes, files::Group};
use engine::local::files::{Area, Change, IndexAction};

pub(crate) struct Partial<'a> {
    pub session: &'a changes::Session,
    pub editing: &'a changes::edit::Session,
}
pub(crate) struct Files<'a> {
    pub session: &'a crate::FileSession,
    pub changes: &'a [Change],
}
pub(crate) enum Index<'a> {
    Files {
        session: &'a crate::FileSession,
        changes: Vec<Change>,
        group: Group,
    },
    Partial(Partial<'a>),
    Conflict {
        session: &'a crate::FileSession,
        view: &'a crate::conflict::View,
    },
}
impl Index<'_> {
    pub fn label(&self) -> String {
        match self {
            Self::Files { changes, group, .. } => format!(
                "{} · {}",
                group.label(),
                i18n::tf!(CommandTargetFiles, count = changes.len())
            ),
            Self::Partial(target) => {
                i18n::tf!(CommandTargetLines, count = target.editing.selected().len())
            }
            Self::Conflict { .. } => format!(
                "{} · {}",
                Group::Unstaged.label(),
                i18n::tf!(CommandTargetFiles, count = 1)
            ),
        }
    }
}
impl<'a> CommandContext<'a> {
    pub fn partial(&self, action: Action) -> Target<Partial<'a>> {
        let Some(session) = self
            .screen
            .diff_selection_target()
            .filter(|_| !self.screen.tasks.session.open())
        else {
            return Target::OutOfScope(i18n::tr!(CommandRequiresWorktreePreview).into());
        };
        if action == Action::Discard && session.active_group() == Group::Staged {
            return Target::OutOfScope(i18n::tr!(CommandNeedsUnstaged).into());
        }
        partial(session)
    }
    pub fn index(&self, action: Action) -> Target<Index<'a>> {
        let staged = action == Action::Unstage;
        let Some(target) = IndexSelection::current(self.screen) else {
            return Target::OutOfScope(i18n::tr!(ChangesSelectionRequired).into());
        };
        let repository = match &target {
            IndexSelection::Files(files) => files.snapshot.path.as_path(),
            IndexSelection::Diff(session) => session.files.snapshot.path.as_path(),
            IndexSelection::Conflict(_) => self
                .screen
                .mode
                .file_session()
                .expect("conflict file session")
                .snapshot
                .path
                .as_path(),
        };
        if Some(repository) != self.repository {
            return Target::Unavailable(i18n::tr!(PartialChanged).into());
        }
        let group = match &target {
            IndexSelection::Files(files) => files.scope().or_else(|| files.group()),
            IndexSelection::Diff(session) => Some(session.active_group()),
            IndexSelection::Conflict(_) => None,
        };
        match (group, staged) {
            (Some(Group::Staged), false) => {
                return Target::OutOfScope(i18n::tr!(CommandNeedsUnstaged).into());
            }
            (Some(Group::Unstaged), true) => {
                return Target::OutOfScope(i18n::tr!(CommandNeedsStaged).into());
            }
            _ => {}
        }
        let wrong_area = || {
            Target::Unavailable(Reason::WrongArea(if staged {
                IndexAction::Unstage
            } else {
                IndexAction::Stage
            }))
        };
        match target {
            IndexSelection::Files(session) => {
                let changes = session.selection();
                if changes.is_empty() {
                    return Target::Unavailable(i18n::tr!(NoActionableFilesInThisArea).into());
                }
                if changes
                    .iter()
                    .any(|change| (change.area == Area::Staged) != staged)
                {
                    return wrong_area();
                }
                let group = Group::of(changes[0].area);
                Target::Ready(Index::Files {
                    session,
                    changes,
                    group,
                })
            }
            IndexSelection::Diff(session) => partial(session).and_then(|partial| {
                if partial.editing.selected().is_empty() {
                    Target::Unavailable(i18n::tr!(PartialSelectionIdle).into())
                } else if (partial.editing.snapshot.change().area == Area::Staged) != staged {
                    wrong_area()
                } else {
                    Target::Ready(Index::Partial(partial))
                }
            }),
            IndexSelection::Conflict(view) => {
                if staged {
                    return wrong_area();
                }
                if !view.resolution.resolved() {
                    return Target::Unavailable(i18n::tr!(ReplayResolveFirst).into());
                }
                let session = self
                    .screen
                    .mode
                    .file_session()
                    .expect("conflict view owns its file session");
                Target::Ready(Index::Conflict { session, view })
            }
        }
    }
    pub fn worktree_files(&self) -> Target<Files<'a>> {
        let Some(session) = self
            .screen
            .worktree_file_list()
            .filter(|_| !self.screen.tasks.session.open())
        else {
            return Target::OutOfScope(i18n::tr!(DiscardFileSelect).into());
        };
        if session.scope().or_else(|| session.group()) == Some(Group::Staged) {
            return Target::OutOfScope(i18n::tr!(CommandNeedsUnstaged).into());
        }
        let changes = self.selection.worktree_files();
        if changes.is_empty() {
            return Target::Unavailable(i18n::tr!(DiscardFileSelect).into());
        }
        if Some(session.snapshot.path.as_path()) != self.repository
            || session.discard_selection() != changes
        {
            return Target::Unavailable(i18n::tr!(PartialChanged).into());
        }
        if changes.iter().any(|change| change.area == Area::Staged) {
            return Target::Unavailable(i18n::tr!(CommandNeedsUnstaged).into());
        }
        if changes
            .iter()
            .any(|change| engine::local::files::discard::Action::for_change(change).is_none())
        {
            return Target::Unavailable(i18n::tr!(DiscardFileUnsupported).into());
        }
        Target::Ready(Files { session, changes })
    }
}
fn partial(session: &changes::Session) -> Target<Partial<'_>> {
    match &session.preview {
        changes::Preview::Ready {
            editing: changes::edit::State::Ready(editing),
            ..
        } => Target::Ready(Partial { session, editing }),
        changes::Preview::Ready {
            editing: changes::edit::State::Unavailable(reason),
            ..
        } => Target::Unavailable(reason.clone().into()),
        _ => Target::Unavailable(i18n::tr!(PartialSelectRequired).into()),
    }
}
