//! Shared file selection with explicit staged and unstaged identities.
mod cursor;
mod folding;
pub(crate) use folding::Expansion;
pub(super) mod tree;
use crate::{Area, Change, Files};
pub(crate) use cursor::{CheckState, Cursor, Group};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Layout {
    #[default]
    Tree,
    Flat,
}
#[derive(Debug, Clone, Copy)]
pub(crate) struct Options {
    pub layout: Layout,
    pub icons: crate::file_icons::Display,
}
impl Layout {
    pub fn label(self) -> &'static str {
        match self {
            Self::Tree => i18n::tr!(FilesTree),
            Self::Flat => i18n::tr!(FilesFlat),
        }
    }
}
use std::collections::BTreeSet;

#[derive(Debug, Clone)]
pub(super) struct FileSession {
    pub snapshot: Files,
    pub cursor: Cursor,
    pub offsets: [usize; 2],
    pub selected: BTreeSet<usize>,
    pub query: String,
    pub search_mode: crate::search::Mode,
    folding: Box<folding::Folding>,
}
impl FileSession {
    pub fn editor_path(&self) -> Option<std::path::PathBuf> {
        match &self.cursor {
            Cursor::Directory { path, .. } => Some(path.clone()),
            _ => self
                .current()
                .map(|change| change.location.name().to_owned()),
        }
    }

    pub fn new(snapshot: Files) -> Self {
        let cursor = if snapshot.changes.is_empty() {
            Cursor::Empty
        } else {
            Cursor::File(0)
        };
        Self {
            snapshot,
            cursor,
            offsets: [0; 2],
            selected: BTreeSet::new(),
            query: String::new(),
            search_mode: Default::default(),
            folding: Default::default(),
        }
    }
    pub fn refreshed(&self, snapshot: Files) -> Self {
        let current = self
            .current()
            .map(|change| (change.area, change.location.clone()));
        let selected = self
            .snapshot
            .changes
            .iter()
            .enumerate()
            .filter(|(index, _)| self.selected.contains(index))
            .map(|(_, change)| (change.area, change.location.clone()))
            .collect::<Vec<_>>();
        let mut next = Self::new(snapshot);
        next.offsets = self.offsets;
        next.query = self.query.clone();
        next.search_mode = self.search_mode;
        next.folding = self.folding.clone();
        next.folding.retain(&next.snapshot.changes);
        if next.rows().is_empty() {
            next.cursor = Cursor::Empty;
        }
        next.selected = next
            .snapshot
            .changes
            .iter()
            .enumerate()
            .filter(|(_, change)| {
                selected
                    .iter()
                    .any(|(area, location)| *area == change.area && *location == change.location)
            })
            .map(|(index, _)| index)
            .collect();
        next.cursor = current
            .and_then(|(area, location)| {
                next.rows()
                    .iter()
                    .position(|change| change.area == area && change.location == location)
                    .or_else(|| {
                        next.rows()
                            .iter()
                            .position(|change| change.location == location)
                    })
            })
            .map(Cursor::File)
            .unwrap_or_else(|| match self.cursor.index() {
                Some(index) if !next.rows().is_empty() => {
                    Cursor::File(index.min(next.rows().len() - 1))
                }
                _ => next.cursor.clone(),
            });
        if let Cursor::Directory { group, .. } = &self.cursor {
            let previous = self.entries(Layout::Tree);
            if let Some(tree::Row::Directory { head, .. }) = previous[group.index()]
                .iter()
                .find(|row| row.cursor(*group) == self.cursor)
            {
                next.cursor = Cursor::Directory {
                    group: *group,
                    path: head.clone(),
                };
                next.ensure_visible();
            }
        }
        next
    }
    pub fn rows(&self) -> Vec<&Change> {
        let mut rows = self
            .snapshot
            .changes
            .iter()
            .filter(|change| self.search_mode.includes(self.matches_change(change)))
            .collect::<Vec<_>>();
        rows.sort_by(|a, b| {
            (a.area == Area::Staged)
                .cmp(&(b.area == Area::Staged))
                .then_with(|| a.location.name().cmp(b.location.name()))
        });
        rows
    }
    pub fn matches_change(&self, change: &Change) -> bool {
        change
            .location
            .name()
            .to_string_lossy()
            .to_lowercase()
            .contains(&self.query.to_lowercase())
    }
    pub fn matches(&self) -> crate::search::Matches {
        crate::search::Matches::new(self.rows().iter().map(|change| self.matches_change(change)))
    }
    pub fn search(&mut self, query: String) {
        if query == self.query {
            return;
        }
        self.query = query;
        self.folding.reset_search();
        self.cursor = match self.search_mode {
            crate::search::Mode::Filter => {
                if self.rows().is_empty() {
                    Cursor::Empty
                } else {
                    Cursor::File(0)
                }
            }
            crate::search::Mode::Locate => self
                .matches()
                .nearest(self.cursor.index())
                .map(Cursor::File)
                .unwrap_or_else(|| self.cursor.clone()),
        };
    }
    pub fn set_search_mode(&mut self, mode: crate::search::Mode) {
        let current = self.current().map(|change| self.change_index(change));
        self.search_mode = mode;
        self.cursor = current
            .and_then(|index| {
                self.rows()
                    .iter()
                    .position(|change| self.change_index(change) == index)
            })
            .map(Cursor::File)
            .unwrap_or_else(|| {
                if self.rows().is_empty() {
                    Cursor::Empty
                } else {
                    Cursor::File(0)
                }
            });
        if mode == crate::search::Mode::Locate {
            self.cursor = self
                .matches()
                .nearest(self.cursor.index())
                .map(Cursor::File)
                .unwrap_or_else(|| self.cursor.clone());
        }
    }
    pub fn change_index(&self, change: &Change) -> usize {
        self.snapshot
            .changes
            .iter()
            .position(|c| c == change)
            .expect("visible change belongs to snapshot")
    }
    pub fn selection(&self) -> Vec<Change> {
        if self.selected.is_empty() {
            self.focused_changes().into_iter().cloned().collect()
        } else {
            self.snapshot
                .changes
                .iter()
                .enumerate()
                .filter(|(i, _)| self.selected.contains(i))
                .map(|(_, c)| c.clone())
                .collect()
        }
    }
}
