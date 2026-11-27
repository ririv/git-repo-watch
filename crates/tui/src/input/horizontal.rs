//! Shared horizontal routes for keyboard, wheel and scrollbar input.
use crate::{
    Mode, Screen, branches, browser, changes, composition, document, history,
    horizontal::{Pane, Viewport},
    inspection, stashes, task_center,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

impl Screen {
    pub(crate) fn horizontal_key(&mut self, key: KeyEvent) -> bool {
        if key.modifiers != KeyModifiers::NONE
            || !self.active_bar().is_plain()
            || !self.active_bar().editor().text.is_empty()
        {
            return false;
        }
        let Some(pane) = self.focused_horizontal_pane() else {
            return false;
        };
        let delta = match key.code {
            KeyCode::Left => -4,
            KeyCode::Right => 4,
            _ => return false,
        };
        if self.pan_horizontal(pane, delta) {
            return true;
        }
        // 这些预览始终保留左右键，异步加载、换行和图片适配时也不会误触导航。
        matches!(
            pane,
            Pane::Browser
                | Pane::BrowserText(_)
                | Pane::HistoryGraph
                | Pane::InspectionDiff
                | Pane::Diff
                | Pane::Conflict(_)
        )
    }

    pub(crate) fn focused_horizontal_pane(&self) -> Option<Pane> {
        if self.tasks.session.open() {
            return matches!(self.tasks.session, task_center::Session::List).then_some(Pane::Tasks);
        }
        if let Some(inspect) = self.mode.inspection() {
            return match inspect.focus {
                inspection::Focus::Files => Some(Pane::Inspection),
                inspection::Focus::Preview => Some(Pane::InspectionDiff),
                inspection::Focus::Message => None,
            };
        }
        match &self.mode {
            Mode::Files(_) => Some(Pane::Worktree),
            Mode::Changes(session) => match session.focus {
                changes::Focus::Files => Some(Pane::Worktree),
                changes::Focus::Preview => {
                    if let changes::Preview::Ready { content, .. } = &session.preview
                        && let document::View::Conflict(view) = content.as_ref()
                    {
                        return Some(Pane::Conflict(view.focus()));
                    }
                    Some(Pane::Diff)
                }
            },
            Mode::Browser(session) => Some(match session.focus {
                browser::Focus::Tree => Pane::Browser,
                browser::Focus::Preview => Pane::BrowserText(session.presentation().active()),
            }),
            Mode::History(view) => match view.as_ref() {
                history::View::Graph(session) => match session.focus {
                    history::GraphFocus::Graph => Some(Pane::HistoryGraph),
                    history::GraphFocus::Files if self.history_preview.area.is_some() => {
                        Some(Pane::HistoryPreview)
                    }
                    history::GraphFocus::Files => None,
                },
                history::View::Diff { .. } => Some(Pane::Diff),
                history::View::Commit(_) => None,
            },
            Mode::Branches(view) if matches!(view.as_ref(), branches::View::List(_)) => {
                Some(Pane::Branches)
            }
            Mode::Stashes(view) => match view.as_ref() {
                stashes::View::List(_) => Some(Pane::Stashes),
                stashes::View::Diff { .. } => Some(Pane::Diff),
                stashes::View::Detail { .. } => None,
            },
            Mode::Worktrees(_) => Some(Pane::Worktrees),
            Mode::Confirm {
                operation: crate::Confirmation::ReplayParents { .. },
                ..
            } => Some(Pane::ReplayParents),
            Mode::Browsing if self.page == crate::Page::Repositories => Some(Pane::Repositories),
            Mode::Management(session) if session.focus.is_input() => Some(Pane::Management),
            Mode::Composition(session) if matches!(session.state, composition::State::Browsing) => {
                Some(match session.focus {
                    composition::Focus::Commits => Pane::CompositionCommits,
                    composition::Focus::Units => Pane::CompositionUnits,
                })
            }
            _ => None,
        }
    }

    fn horizontal_viewport(&mut self, pane: Pane) -> Option<&mut Viewport> {
        if self.focused_horizontal_pane() != Some(pane) {
            return None;
        }
        Some(match (pane, &mut self.mode) {
            (Pane::Tasks, _) => &mut self.tasks.horizontal,
            (
                Pane::ReplayParents,
                Mode::Confirm {
                    operation: crate::Confirmation::ReplayParents { horizontal, .. },
                    ..
                },
            ) => horizontal,
            (Pane::Worktree, Mode::Files(files)) => &mut files.horizontal,
            (Pane::Worktree, Mode::Changes(session)) => &mut session.files.horizontal,
            (Pane::Browser, Mode::Browser(session)) => &mut session.horizontal,
            (Pane::BrowserText(mode), Mode::Browser(session)) => {
                &mut session.panes.get_mut(mode).position.horizontal
            }
            (Pane::HistoryPreview, _) => &mut self.history_preview.file_paths,
            (Pane::HistoryGraph, Mode::History(view)) => match view.as_mut() {
                history::View::Graph(session) => &mut session.horizontal,
                _ => return None,
            },
            (Pane::Branches, Mode::Branches(view)) => match view.as_mut() {
                branches::View::List(session) => &mut session.horizontal,
                _ => return None,
            },
            (Pane::Stashes, Mode::Stashes(view)) => match view.as_mut() {
                stashes::View::List(session) => &mut session.horizontal,
                _ => return None,
            },
            (Pane::Worktrees, Mode::Worktrees(session)) => &mut session.horizontal,
            (Pane::Repositories, Mode::Browsing) => &mut self.repository_horizontal,
            (Pane::Management, Mode::Management(session)) => &mut session.horizontal,
            (Pane::CompositionCommits, Mode::Composition(session)) => {
                &mut session.commits_horizontal
            }
            (Pane::CompositionUnits, Mode::Composition(session)) => &mut session.units_horizontal,
            (Pane::Inspection, mode) => &mut mode.inspection_mut()?.file_paths,
            (Pane::InspectionDiff, mode) => &mut mode.inspection_mut()?.review.horizontal,
            (Pane::Diff | Pane::Conflict(_), mode) => return document_viewport(mode),
            _ => return None,
        })
    }

    pub(super) fn pan_horizontal(&mut self, pane: Pane, delta: isize) -> bool {
        let Some(viewport) = self.horizontal_viewport(pane).filter(|v| v.overflowing()) else {
            return false;
        };
        viewport.advance(delta);
        true
    }

    pub(super) fn seek_horizontal(&mut self, pane: Pane, offset: usize) -> bool {
        let Some(viewport) = self.horizontal_viewport(pane) else {
            return false;
        };
        viewport.seek(offset);
        true
    }
}

fn document_viewport(mode: &mut Mode) -> Option<&mut Viewport> {
    let content = match mode {
        Mode::Changes(session) => match &mut session.preview {
            changes::Preview::Ready { content, .. } => content,
            _ => return None,
        },
        Mode::Stashes(view) => match view.as_mut() {
            stashes::View::Diff { content, .. } => content,
            _ => return None,
        },
        Mode::History(view) => match view.as_mut() {
            history::View::Diff { content, .. } => content,
            _ => return None,
        },
        _ => return None,
    };
    content.horizontal()
}
