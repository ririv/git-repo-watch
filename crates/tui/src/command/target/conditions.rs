//! Action conditions consume resolved capabilities instead of querying view targets again.
use super::{CommandContext, Resolved, Target, resolved::Composition};
use crate::Action;
use engine::local::files::Area;

impl<'a> CommandContext<'a> {
    pub fn validate(&self, action: Action, target: Resolved<'a>) -> Target<Resolved<'a>> {
        // 暂存条件已经在索引目标中解析，包括按文件、按行与冲突暂存。
        if matches!(target, Resolved::Index(_)) {
            return Target::Ready(target);
        }
        if let Some(reason) = crate::replay::unavailable(
            self.repository
                .and_then(|root| self.screen.replay_state(root)),
            action,
        ) {
            return Target::Unavailable(reason.into());
        }
        let reason = match (&target, action) {
            (
                Resolved::Conflict(view),
                Action::ConflictAcceptMenu
                | Action::ConflictAccept(_)
                | Action::ConflictBlockMenu
                | Action::ConflictBlock(_),
            ) => view.view.resolution.reason().map(str::to_owned),
            (Resolved::Conflict(view), Action::ConflictReset) => {
                view.view.resolution.reset_reason().map(str::to_owned)
            }
            (Resolved::Conflict(view), Action::ConflictUndo)
                if view.view.resolution.undo().is_none() =>
            {
                Some(i18n::tr!(ConflictUndoUnavailable).into())
            }
            (
                Resolved::Conflict(view),
                Action::ConflictView(crate::conflict::DisplayMode::Pair(pair)),
            ) => view.view.pairs.reason(pair).map(str::to_owned),
            (Resolved::File(file), Action::CopyPath) if file.relative_path().to_str().is_none() => {
                Some(i18n::tr!(CopyPathNotUtf8).into())
            }
            (_, Action::StashSaveMode(crate::stash_save::Scope::Selected))
                if self.selection.marked_file_count() == 0 =>
            {
                Some(i18n::tr!(StashSelectFiles).into())
            }
            (
                Resolved::Composition(Composition::Plan(session)),
                Action::Compose(crate::composition::Action::Up),
            ) if session.selected == 0 => Some(i18n::tr!(ComposeFirstStep).into()),
            (
                Resolved::Composition(Composition::Plan(session)),
                Action::Compose(crate::composition::Action::Down),
            ) if session.selected + 1 >= session.prepared.plan.commits.len() => {
                Some(i18n::tr!(ComposeLastStep).into())
            }
            (Resolved::Browser(session), Action::BrowserDiff)
                if !session
                    .selected()
                    .is_some_and(|entry| entry.status().is_some_and(|status| status.changed())) =>
            {
                Some(i18n::tr!(BrowserNoDiff).into())
            }
            (Resolved::Clipboard(scope), Action::Copy) => self
                .selection
                .unavailable(*scope, self.repository)
                .map(str::to_owned),
            (Resolved::History(graph), Action::LogMore) if graph.cursor().is_none() => {
                Some(i18n::tr!(HistoryNoMore).into())
            }
            (Resolved::History(_), Action::LogSelect | Action::LogSelectAll)
                if self.selection.commit().is_none() =>
            {
                Some(i18n::tr!(ReplaySelectCommit).into())
            }
            (Resolved::History(graph), Action::LogSelectAll)
                if graph.records.len() > engine::operations::replay::batch::MAX_COMMITS =>
            {
                Some(i18n::tr!(ReplayBatchTooLarge).into())
            }
            (Resolved::Partial(partial), Action::Discard)
                if partial.editing.selected().is_empty() =>
            {
                Some(i18n::tr!(PartialSelectionIdle).into())
            }
            (Resolved::Partial(partial), Action::Discard)
                if partial.editing.snapshot.change().area == Area::Staged =>
            {
                Some(i18n::tr!(CommandNeedsUnstaged).into())
            }
            (Resolved::Partial(partial), Action::Discard)
                if Some(partial.editing.snapshot.files().path.as_path()) != self.repository
                    || partial.session.selection_key().as_ref()
                        != self.selection.diff_selection() =>
            {
                Some(i18n::tr!(PartialChanged).into())
            }
            (Resolved::Previewable(target), Action::BrowserMode(mode)) => {
                use crate::browser::opening::{Presentation, TextMode};
                match target.presentation {
                    Presentation::Single(current) if current == mode => Some(match mode {
                        TextMode::Source => i18n::tr!(CommandSourceCurrent).into(),
                        TextMode::Preview => i18n::tr!(CommandPreviewCurrent).into(),
                    }),
                    _ => None,
                }
            }
            _ => None,
        };
        match reason {
            Some(reason) => Target::Unavailable(reason.into()),
            None => Target::Ready(target),
        }
    }
}
