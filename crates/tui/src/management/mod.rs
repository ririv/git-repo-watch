//! Repository registration and one-shot discovery share a selectable list.
mod input;
use engine::discovery::{Record, Report};
use ratatui::widgets::ListState;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Selection {
    Included,
    Skipped,
}
#[derive(Debug)]
pub(crate) enum Status {
    New(Selection),
    Managed,
    Failed(String),
}
#[derive(Debug)]
pub(crate) struct Entry {
    pub path: PathBuf,
    pub status: Status,
}
#[derive(Debug)]
pub(crate) enum Source {
    Managed,
    Scan {
        root: PathBuf,
        excluded: Vec<PathBuf>,
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Action {
    Apply,
    Back,
}
impl Action {
    pub const ALL: [Self; 2] = [Self::Apply, Self::Back];
    pub fn next(self) -> Self {
        match self {
            Self::Apply => Self::Back,
            Self::Back => Self::Apply,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Apply => i18n::tr!(ScanAddSelected),
            Self::Back => i18n::tr!(ManageBack),
        }
    }
}
#[derive(Debug)]
pub(crate) struct Controls {
    pub selected: Action,
    pub draft: Box<crate::bar::InputBar>,
}
pub(crate) type Focus = crate::input_pages::Selection<Controls>;
#[derive(Debug)]
pub(crate) struct Session {
    pub source: Source,
    pub entries: Vec<Entry>,
    pub state: ListState,
    pub query: String,
    pub focus: Focus,
}
impl Session {
    pub fn managed(paths: &[PathBuf]) -> Self {
        Self::new(
            Source::Managed,
            paths
                .iter()
                .map(|path| Entry {
                    path: path.clone(),
                    status: Status::Managed,
                })
                .collect(),
        )
    }
    pub fn scanned(report: Report, registered: &[PathBuf]) -> Self {
        let entries = report
            .repositories
            .into_iter()
            .map(|record| match record {
                Record::Scanned { path } => {
                    let status = if registered.contains(&path) {
                        Status::Managed
                    } else {
                        Status::New(Selection::Included)
                    };
                    Entry { path, status }
                }
                Record::Failed { path, error } => Entry {
                    path,
                    status: Status::Failed(error),
                },
            })
            .collect();
        Self::new(
            Source::Scan {
                root: report.root,
                excluded: report.excluded_roots,
            },
            entries,
        )
    }
    fn new(source: Source, entries: Vec<Entry>) -> Self {
        let state = ListState::default().with_selected((!entries.is_empty()).then_some(0));
        Self {
            source,
            entries,
            state,
            query: String::new(),
            focus: Focus::new(crate::input_pages::Availability::InputOnly),
        }
    }
    pub fn title(&self) -> &'static str {
        match self.source {
            Source::Managed => i18n::tr!(RepositoryManagement),
            Source::Scan { .. } => i18n::tr!(ScanResults),
        }
    }
    pub fn available_actions(&self) -> crate::input_pages::Availability<Action> {
        use crate::input_pages::Availability;
        match self.source {
            Source::Managed => Availability::InputOnly,
            Source::Scan { .. } => Availability::Actions(if self.selection().is_empty() {
                Action::Back
            } else {
                Action::Apply
            }),
        }
    }
    pub fn actions(&self) -> &'static [Action] {
        match self.source {
            Source::Managed => &[],
            Source::Scan { .. } => &Action::ALL,
        }
    }
    pub fn rows(&self) -> Vec<usize> {
        let query = self.query.to_lowercase();
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.path.to_string_lossy().to_lowercase().contains(&query))
            .map(|(i, _)| i)
            .collect()
    }
    pub fn search(&mut self, query: String) {
        if self.query == query {
            return;
        }
        self.query = query;
        self.state.select((!self.rows().is_empty()).then_some(0));
    }
    pub fn selected(&self) -> Option<&Entry> {
        self.entries.get(*self.rows().get(self.state.selected()?)?)
    }
    pub fn registered_path(&self) -> Option<&Path> {
        self.selected()
            .filter(|entry| matches!(entry.status, Status::Managed))
            .map(|entry| entry.path.as_path())
    }
    pub fn advance(&mut self, delta: isize) {
        let rows = self.rows();
        self.state.select(rows.len().checked_sub(1).map(|last| {
            self.state
                .selected()
                .unwrap_or(0)
                .saturating_add_signed(delta)
                .min(last)
        }));
    }
    pub fn toggle(&mut self, visible: usize) {
        let Some(&index) = self.rows().get(visible) else {
            return;
        };
        if let Status::New(selection) = &mut self.entries[index].status {
            *selection = match selection {
                Selection::Included => Selection::Skipped,
                Selection::Skipped => Selection::Included,
            };
        }
    }
    pub fn toggle_all(&mut self) {
        let rows = self.rows();
        let choice = if rows
            .iter()
            .any(|&i| matches!(self.entries[i].status, Status::New(Selection::Skipped)))
        {
            Selection::Included
        } else {
            Selection::Skipped
        };
        for index in rows {
            if let Status::New(selection) = &mut self.entries[index].status {
                *selection = choice;
            }
        }
    }
    pub fn selection(&self) -> Vec<PathBuf> {
        self.entries
            .iter()
            .filter(|entry| matches!(entry.status, Status::New(Selection::Included)))
            .map(|entry| entry.path.clone())
            .collect()
    }
    pub fn refresh(&mut self, registered: &[PathBuf]) {
        match self.source {
            Source::Managed => {
                self.entries
                    .retain(|entry| registered.contains(&entry.path));
                for path in registered {
                    if !self.entries.iter().any(|entry| &entry.path == path) {
                        self.entries.push(Entry {
                            path: path.clone(),
                            status: Status::Managed,
                        });
                    }
                }
            }
            Source::Scan { .. } => {
                for entry in &mut self.entries {
                    match entry.status {
                        Status::Failed(_) => {}
                        _ if registered.contains(&entry.path) => entry.status = Status::Managed,
                        Status::Managed => entry.status = Status::New(Selection::Skipped),
                        Status::New(_) => {}
                    }
                }
            }
        }
        self.advance(0);
    }
}
