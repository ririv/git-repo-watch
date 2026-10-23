//! Shared file selection with explicit staged and unstaged identities.
use crate::{Area, Change, Files};
use std::collections::BTreeSet;

#[derive(Debug, Clone)]
pub(super) struct FileSession {
    pub snapshot: Files,
    pub cursor: usize,
    pub offsets: [usize; 2],
    pub selected: BTreeSet<usize>,
    pub query: String,
}
impl FileSession {
    pub fn new(snapshot: Files) -> Self {
        Self {
            snapshot,
            cursor: 0,
            offsets: [0; 2],
            selected: BTreeSet::new(),
            query: String::new(),
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
            .filter(|change| {
                change
                    .location
                    .name()
                    .to_string_lossy()
                    .to_lowercase()
                    .contains(&self.query.to_lowercase())
            })
            .collect::<Vec<_>>();
        rows.sort_by_key(|change| change.area == Area::Staged);
        rows
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
