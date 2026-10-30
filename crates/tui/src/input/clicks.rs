//! A completed pair consumes itself; item identity survives redraws and excludes row reuse.
use crate::{
    Configuration, Mode, Page, Screen, branches, history, pointer::Target, settings, stashes,
};
use ratatui::layout::Position;
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

// 终端协议只报告按下／松开，不提供系统双击计数；由 TUI 使用 400ms 窗口判定。
const INTERVAL: Duration = Duration::from_millis(400);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Identity {
    Task(String),
    Composition {
        repository: PathBuf,
        unit: String,
    },
    CommitAction {
        repository: PathBuf,
        action: crate::commit_form::Action,
    },
    Browser {
        repository: PathBuf,
        path: PathBuf,
    },
    Repository(PathBuf),
    Layout(crate::diff::Preference),
    WorktreeDirectory {
        repository: PathBuf,
        group: crate::files::Group,
        path: PathBuf,
    },
    File {
        repository: PathBuf,
        change: engine::local::files::Change,
    },
    Branch {
        repository: PathBuf,
        reference: String,
    },
    Stash {
        repository: PathBuf,
        oid: String,
    },
    Commit {
        repository: PathBuf,
        oid: String,
    },
    HistoryFile {
        repository: PathBuf,
        commit: String,
        location: engine::local::files::Location,
    },
    Setting(Configuration, settings::Item),
}
#[derive(Debug, Default)]
pub(crate) enum Clicks {
    #[default]
    Idle,
    Armed {
        item: Identity,
        position: Position,
        at: Instant,
    },
}
impl Clicks {
    pub fn reset(&mut self) {
        *self = Self::Idle;
    }
    pub fn press(&mut self, item: Identity, position: Position, at: Instant) -> bool {
        if matches!(self, Self::Armed { item: previous, position: point, at: time }
            if *previous == item && point.y == position.y && point.x.abs_diff(position.x) <= 1 && at.duration_since(*time) <= INTERVAL)
        {
            self.reset();
            true
        } else {
            *self = Self::Armed { item, position, at };
            false
        }
    }
}
impl Screen {
    pub(super) fn clicked_identity(&self, target: Target) -> Option<Identity> {
        match target {
            Target::Management => None,
            Target::ScheduleButton
            | Target::WorkflowSteps
            | Target::WorkflowToggle
            | Target::WorkflowActions => None,
            Target::CompositionCommits | Target::CompositionUnits => {
                let Mode::Composition(session) = &self.mode else {
                    return None;
                };
                let unit = if target == Target::CompositionCommits {
                    session.ids().first()?.clone()
                } else {
                    session.selected_unit()?.id.clone()
                };
                Some(Identity::Composition {
                    repository: session.prepared.snapshot.path.clone(),
                    unit,
                })
            }
            Target::Browser => match &self.mode {
                Mode::Browser(session) => Some(Identity::Browser {
                    repository: session.root.clone(),
                    path: session.selected_row()?.head.to_owned(),
                }),
                _ => None,
            },
            Target::Repositories => self.selected_path().map(Identity::Repository),
            Target::Files => {
                let files = self.mode.file_session()?;
                match &files.cursor {
                    crate::files::Cursor::Directory { group, path } => {
                        Some(Identity::WorktreeDirectory {
                            repository: files.snapshot.path.clone(),
                            group: *group,
                            path: path.clone(),
                        })
                    }
                    crate::files::Cursor::File(_) => Some(Identity::File {
                        repository: files.snapshot.path.clone(),
                        change: files.current()?.clone(),
                    }),
                    crate::files::Cursor::Empty => None,
                }
            }
            Target::Branches => match &self.mode {
                Mode::Branches(view) => match view.as_ref() {
                    branches::View::List(session) => Some(Identity::Branch {
                        repository: session.snapshot.path.clone(),
                        reference: session.selected()?.reference.clone(),
                    }),
                    _ => None,
                },
                _ => None,
            },
            Target::Stashes => match &self.mode {
                Mode::Stashes(view) => match view.as_ref() {
                    stashes::View::List(session) => Some(Identity::Stash {
                        repository: session.snapshot.path.clone(),
                        oid: session.selected()?.oid.to_string(),
                    }),
                    _ => None,
                },
                _ => None,
            },
            Target::ReplayParent => match &self.mode {
                Mode::Confirm {
                    operation:
                        crate::Confirmation::ReplayParents {
                            candidate,
                            selected,
                            ..
                        },
                    ..
                } => Some(Identity::Commit {
                    repository: candidate.path.clone(),
                    oid: candidate.parents[*selected].oid.to_string(),
                }),
                _ => None,
            },
            Target::History | Target::HistoryFiles => match &self.mode {
                Mode::History(view) => match (view.as_ref(), target) {
                    (history::View::Graph(session), Target::History) => Some(Identity::Commit {
                        repository: session.references.path.clone(),
                        oid: session.selected()?.oid.to_string(),
                    }),
                    (history::View::Commit(commit), Target::HistoryFiles) => {
                        Some(Identity::HistoryFile {
                            repository: commit.detail.path.clone(),
                            commit: commit.detail.oid.to_string(),
                            location: commit
                                .rows()
                                .get(commit.files.selected()?)?
                                .location
                                .clone(),
                        })
                    }
                    _ => None,
                },
                _ => None,
            },
            Target::Settings => match self.page {
                Page::Configuration(section) => Some(Identity::Setting(
                    section,
                    *section
                        .items(self.snapshot.registration, &self.config.ai)
                        .get(self.setting_selected)?,
                )),
                _ => None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn repo(name: &str) -> Identity {
        Identity::Repository(PathBuf::from(name))
    }
    #[test]
    fn pairs_require_matching_identity_location_and_time_and_consume_the_second_press() {
        let mut clicks = Clicks::default();
        let now = Instant::now();
        let point = Position::new(5, 4);
        assert!(!clicks.press(repo("one"), point, now));
        assert!(clicks.press(
            repo("one"),
            Position::new(6, 4),
            now + Duration::from_millis(150)
        ));
        assert!(!clicks.press(repo("one"), point, now + Duration::from_millis(250)));
        assert!(!clicks.press(repo("two"), point, now + Duration::from_millis(300)));
        assert!(!clicks.press(
            repo("two"),
            Position::new(5, 5),
            now + Duration::from_millis(350)
        ));
        assert!(!clicks.press(
            repo("two"),
            Position::new(5, 5),
            now + Duration::from_millis(800)
        ));
        clicks.reset();
        assert!(!clicks.press(
            repo("two"),
            Position::new(5, 5),
            now + Duration::from_millis(850)
        ));
    }
}
