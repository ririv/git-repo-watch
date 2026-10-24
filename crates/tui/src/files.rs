//! Shared file selection with explicit staged and unstaged identities.
pub(super) mod tree;
use crate::{Area, Change, Files};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Layout {
    #[default]
    Tree,
    Flat,
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
    pub cursor: usize,
    pub offsets: [usize; 2],
    pub selected: BTreeSet<usize>,
    pub query: String,
    pub search_mode: crate::search::Mode,
}
impl FileSession {
    pub fn new(snapshot: Files) -> Self {
        Self {
            snapshot,
            cursor: 0,
            offsets: [0; 2],
            selected: BTreeSet::new(),
            query: String::new(),
            search_mode: Default::default(),
        }
    }
    pub fn refreshed(&self, snapshot: Files) -> Self {
        let current = self
            .rows()
            .get(self.cursor)
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
        next.query = self.query.clone();
        next.search_mode = self.search_mode;
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
            .unwrap_or_else(|| self.cursor.min(next.rows().len().saturating_sub(1)));
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
        self.cursor = match self.search_mode {
            crate::search::Mode::Filter => 0,
            crate::search::Mode::Locate => self
                .matches()
                .nearest(Some(self.cursor))
                .unwrap_or(self.cursor),
        };
    }
    pub fn set_search_mode(&mut self, mode: crate::search::Mode) {
        let current = self
            .rows()
            .get(self.cursor)
            .map(|change| self.change_index(change));
        self.search_mode = mode;
        self.cursor = current
            .and_then(|index| {
                self.rows()
                    .iter()
                    .position(|change| self.change_index(change) == index)
            })
            .unwrap_or(0);
        if mode == crate::search::Mode::Locate {
            self.cursor = self
                .matches()
                .nearest(Some(self.cursor))
                .unwrap_or(self.cursor);
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
        let rows = self.rows();
        if self.selected.is_empty() {
            rows.get(self.cursor)
                .map(|c| vec![(*c).clone()])
                .unwrap_or_default()
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
