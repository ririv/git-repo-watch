//! Resolve repository, content and view capabilities without reading external state.
use super::{
    CommandContext, File, Target,
    resolved::{Composition, Conflict, Destination, Stash},
};
use crate::{Action, Mode, clipboard};
use std::path::Path;

impl<'a> CommandContext<'a> {
    pub fn repository(&self) -> Target<&'a Path> {
        match self.repository {
            Some(root) => Target::Ready(root),
            None => Target::Unavailable(i18n::tr!(SelectARepositoryFirst).into()),
        }
    }
    pub fn editor(&self) -> Target<Destination<'a>> {
        match self.repository.and_then(|root| {
            self.selection
                .editor_target(root)
                .map(|path| Destination { root, path })
        }) {
            Some(target) => Target::Ready(target),
            None => Target::Unavailable(i18n::tr!(ExternalEditorSelectionRequired).into()),
        }
    }
    pub fn directory(&self, unavailable: &'static str) -> Target<Destination<'a>> {
        match self.repository.and_then(|root| {
            self.selection
                .directory_target(root)
                .map(|path| Destination { root, path })
        }) {
            Some(target) => Target::Ready(target),
            None => Target::Unavailable(unavailable.into()),
        }
    }
    pub fn clipboard(&self) -> Target<clipboard::Scope> {
        match self.selection.copy_scope() {
            Some(scope) => Target::Ready(scope),
            None => Target::Unavailable(i18n::tr!(CopySelectionRequired).into()),
        }
    }
    pub fn file(&self) -> Target<File<'a>> {
        match self.selection.file_target() {
            Some(target) => Target::Ready(target),
            None => match self.selection {
                clipboard::Selection::Browser { .. }
                | clipboard::Selection::Diff { .. }
                | clipboard::Selection::HistoricalDiff { .. } => {
                    Target::Unavailable(i18n::tr!(CommandRequiresFileTarget).into())
                }
                _ => Target::OutOfScope(i18n::tr!(CommandRequiresFileTarget).into()),
            },
        }
    }
    pub fn search(&self, action: Action) -> Target<crate::search::Support> {
        let support = self.screen.search_support();
        if support.admits(action) {
            Target::Ready(support)
        } else {
            Target::OutOfScope(i18n::tr!(SearchUnavailable).into())
        }
    }
    pub fn refresh(&self) -> Target<crate::view_refresh::Target> {
        match crate::view_refresh::Target::current(self.screen) {
            Ok(target) => Target::Ready(target),
            Err(reason) => Target::Unavailable(reason.into()),
        }
    }
    pub fn conflict(&self) -> Target<Conflict<'a>> {
        let Some(view) = self.screen.conflict_view() else {
            return Target::OutOfScope(i18n::tr!(ConflictFileRequired).into());
        };
        let Some(resolution) = self.selection.conflict_resolution() else {
            return Target::Unavailable(i18n::tr!(ConflictFileRequired).into());
        };
        self.repository().map(|repository| Conflict {
            view,
            resolution,
            repository,
        })
    }
    pub fn branches(&self) -> Target<&'a crate::branches::Session> {
        match &self.screen.mode {
            Mode::Branches(view) => match view.as_ref() {
                crate::branches::View::List(session) => Target::Ready(session),
                _ => Target::OutOfScope(i18n::tr!(BranchFilterRequiresView).into()),
            },
            _ => Target::OutOfScope(i18n::tr!(BranchFilterRequiresView).into()),
        }
    }
    pub fn stash(&self) -> Target<Stash<'a>> {
        match &self.screen.mode {
            Mode::Stashes(view) => match view.as_ref() {
                crate::stashes::View::Detail { session, inspect } => {
                    Target::Ready(Stash { session, inspect })
                }
                _ => Target::OutOfScope(i18n::tr!(StashPreviewRequiresDetail).into()),
            },
            _ => Target::OutOfScope(i18n::tr!(StashPreviewRequiresDetail).into()),
        }
    }
    pub fn browser(&self) -> Target<&'a crate::browser::Session> {
        match &self.screen.mode {
            Mode::Browser(session) => Target::Ready(session),
            _ => Target::OutOfScope(i18n::tr!(CommandRequiresBrowser).into()),
        }
    }
    pub fn previewable(&self) -> Target<&'a crate::browser::opening::OpenTarget> {
        match self.selection.opener() {
            Some(target) => Target::Ready(target),
            None => Target::OutOfScope(i18n::tr!(CommandRequiresPreviewableFile).into()),
        }
    }
    pub fn composition(&self, action: Action) -> Target<Composition<'a>> {
        match &self.screen.mode {
            Mode::Composition(session) => match &session.state {
                crate::composition::State::Browsing => Target::Ready(Composition::Plan(session)),
                _ => Target::Unavailable(i18n::tr!(ComposeSelected).into()),
            },
            Mode::CompositionWaiting(waiting)
                if matches!(
                    action,
                    Action::Compose(crate::composition::Action::Regenerate)
                ) =>
            {
                Target::Ready(Composition::Waiting(waiting))
            }
            Mode::CompositionWaiting(_) => Target::Unavailable(i18n::tr!(ComposeGenerating).into()),
            _ => Target::OutOfScope(i18n::tr!(CommandRequiresComposition).into()),
        }
    }
}
