//! File and directory focus share navigation, while checked identities remain individual changes.
use super::{FileSession, Layout, tree::Row};
use crate::{Area, Change};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Group {
    Unstaged,
    Staged,
}
impl Group {
    pub fn of(area: Area) -> Self {
        if area == Area::Staged {
            Self::Staged
        } else {
            Self::Unstaged
        }
    }
    pub fn other(self) -> Self {
        match self {
            Self::Staged => Self::Unstaged,
            Self::Unstaged => Self::Staged,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Staged => i18n::tr!(ChangesStaged),
            Self::Unstaged => i18n::tr!(ChangesUnstaged),
        }
    }
    pub fn index(self) -> usize {
        match self {
            Self::Staged => 0,
            Self::Unstaged => 1,
        }
    }
    pub fn from_index(index: usize) -> Self {
        match index {
            0 => Self::Staged,
            1 => Self::Unstaged,
            _ => unreachable!("two file groups"),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Cursor {
    Empty,
    File(usize),
    Directory { group: Group, path: PathBuf },
}
impl Cursor {
    pub fn index(&self) -> Option<usize> {
        if let Self::File(index) = self {
            Some(*index)
        } else {
            None
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CheckState {
    Unchecked,
    Mixed,
    Checked,
}

impl Row {
    pub fn cursor(&self, group: Group) -> Cursor {
        match self {
            Self::File { index, .. } => Cursor::File(*index),
            Self::Directory { path, .. } => Cursor::Directory {
                group,
                path: path.clone(),
            },
        }
    }
    pub fn indices(&self) -> &[usize] {
        match self {
            Self::File { index, .. } => std::slice::from_ref(index),
            Self::Directory { indices, .. } => indices,
        }
    }
}
impl FileSession {
    pub fn current(&self) -> Option<&Change> {
        self.rows().get(self.cursor.index()?).copied()
    }
    pub fn group(&self) -> Option<Group> {
        match &self.cursor {
            Cursor::Empty => None,
            Cursor::File(_) => self.current().map(|change| Group::of(change.area)),
            Cursor::Directory { group, .. } => Some(*group),
        }
    }
    pub fn entries(&self, layout: Layout) -> [Vec<Row>; 2] {
        let rows = self.rows();
        [Group::Staged, Group::Unstaged].map(|group| {
            let changes = rows
                .iter()
                .copied()
                .enumerate()
                .filter(|(_, change)| Group::of(change.area) == group)
                .collect::<Vec<_>>();
            super::tree::project(&changes, layout, &|head, tip| {
                self.folding.expansion(group, head, tip, &self.query)
            })
        })
    }
    fn cursors(&self, layout: Layout) -> Vec<Cursor> {
        self.entries(layout)
            .into_iter()
            .enumerate()
            .flat_map(|(group, rows)| {
                rows.into_iter()
                    .map(move |row| row.cursor(Group::from_index(group)))
            })
            .collect()
    }
    pub fn select_row(&mut self, index: usize, layout: Layout) -> bool {
        let Some(cursor) = self.cursors(layout).into_iter().nth(index) else {
            return false;
        };
        self.cursor = cursor;
        true
    }
    pub fn move_cursor(&mut self, delta: isize, layout: Layout) {
        let cursors = self.cursors(layout);
        if cursors.is_empty() {
            self.cursor = Cursor::Empty;
            return;
        }
        let index = cursors
            .iter()
            .position(|cursor| *cursor == self.cursor)
            .map_or(0, |index| {
                index.saturating_add_signed(delta).min(cursors.len() - 1)
            });
        self.cursor = cursors[index].clone();
    }
    pub fn edge(&mut self, last: bool, layout: Layout) {
        if !last {
            self.offset = 0;
        }
        let mut cursors = self.cursors(layout).into_iter();
        self.cursor = if last {
            cursors.next_back()
        } else {
            cursors.next()
        }
        .unwrap_or(Cursor::Empty);
    }
    #[cfg(test)]
    pub fn switch_group(&mut self, layout: Layout) {
        let group = self.group();
        for (index, rows) in self.entries(layout).iter().enumerate() {
            let candidate = Group::from_index(index);
            if Some(candidate) != group
                && let Some(row) = rows.first()
            {
                self.cursor = row.cursor(candidate);
                return;
            }
        }
    }
    pub fn focused_changes(&self) -> Vec<&Change> {
        match &self.cursor {
            Cursor::Empty => Vec::new(),
            Cursor::File(_) => self.current().into_iter().collect(),
            Cursor::Directory { group, path } => self
                .rows()
                .into_iter()
                .filter(|change| {
                    Group::of(change.area) == *group
                        && change
                            .location
                            .name()
                            .parent()
                            .is_some_and(|parent| parent.starts_with(path))
                })
                .collect(),
        }
    }
    pub fn toggle_current(&mut self) {
        if let Some(group) = self.group() {
            let indices = self
                .focused_changes()
                .into_iter()
                .map(|change| self.change_index(change))
                .collect();
            self.toggle_marks(group, indices);
        }
    }

    pub fn toggle_visible_group(&mut self) {
        if let Some(group) = self.group() {
            let indices = self
                .rows()
                .into_iter()
                .filter(|change| Group::of(change.area) == group)
                .map(|change| self.change_index(change))
                .collect();
            self.toggle_marks(group, indices);
        }
    }

    fn toggle_marks(&mut self, group: Group, indices: std::collections::BTreeSet<usize>) {
        if indices.is_empty() {
            return;
        }
        let mut marked = self
            .selected
            .0
            .take()
            .filter(|(active, _)| *active == group)
            .map(|(_, indices)| indices)
            .unwrap_or_default();
        if indices.is_subset(&marked) {
            marked.retain(|index| !indices.contains(index));
        } else {
            marked.extend(indices);
        }
        self.selected.replace(group, marked);
    }

    #[cfg(test)]
    pub(crate) fn mark_file(&mut self, index: usize) {
        let group = Group::of(self.snapshot.changes[index].area);
        if !self
            .selected
            .indices(group)
            .is_some_and(|marks| marks.contains(&index))
        {
            self.toggle_marks(group, [index].into());
        }
    }

    pub fn check_state(&self, row: &Row, rows: &[&Change]) -> CheckState {
        let checked = row
            .indices()
            .iter()
            .filter(|index| {
                let change = rows[**index];
                self.selected
                    .indices(Group::of(change.area))
                    .is_some_and(|marked| marked.contains(&self.change_index(change)))
            })
            .count();
        if checked == 0 {
            CheckState::Unchecked
        } else if checked == row.indices().len() {
            CheckState::Checked
        } else {
            CheckState::Mixed
        }
    }
    pub fn step_match(&self, backwards: bool) -> Option<usize> {
        if matches!(self.cursor, Cursor::Directory { .. }) {
            let rows = self.rows();
            let mut matches = self
                .focused_changes()
                .into_iter()
                .filter(|change| self.matches_change(change))
                .map(|change| {
                    rows.iter()
                        .position(|candidate| *candidate == change)
                        .expect("focused change is visible")
                });
            let index = if backwards {
                matches.next_back()
            } else {
                matches.next()
            };
            if index.is_some() {
                return index;
            }
        }
        self.matches().step(self.cursor.index(), backwards)
    }
    pub fn set_layout(&mut self, layout: Layout) {
        if layout == Layout::Tree {
            self.ensure_visible();
        }
        if layout == Layout::Flat && matches!(self.cursor, Cursor::Directory { .. }) {
            self.cursor = self
                .focused_changes()
                .first()
                .map(|change| {
                    Cursor::File(
                        self.rows()
                            .iter()
                            .position(|candidate| *candidate == *change)
                            .expect("focused file is visible"),
                    )
                })
                .unwrap_or(Cursor::Empty);
        }
    }
}
