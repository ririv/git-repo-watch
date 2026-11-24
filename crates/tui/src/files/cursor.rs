//! File and directory focus share navigation, while checked identities remain individual changes.
use super::{FileSession, Layout, tree::Row};
use crate::{Area, Change};
use std::{collections::BTreeSet, path::PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

pub(crate) struct MarkTargets {
    group: Group,
    indices: BTreeSet<usize>,
}

pub(crate) enum MarkEdit {
    Toggle(MarkTargets),
    Add(MarkTargets),
    Clear,
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
        self.change(self.cursor.index()?)
    }
    pub fn group(&self) -> Option<Group> {
        match &self.cursor {
            Cursor::Empty => None,
            Cursor::File(_) => self.current().map(|change| Group::of(change.area)),
            Cursor::Directory { group, .. } => Some(*group),
        }
    }
    pub fn select_row(&mut self, index: usize, layout: Layout) -> bool {
        let Some(cursor) = self.list(layout).cursor(index) else {
            return false;
        };
        self.cursor = cursor;
        true
    }
    pub fn move_cursor(&mut self, delta: isize, layout: Layout) {
        let list = self.list(layout);
        let index = list.item_index(&self.cursor).map_or(0, |index| {
            index
                .saturating_add_signed(delta)
                .min(list.len().saturating_sub(1))
        });
        self.cursor = list.cursor(index).unwrap_or(Cursor::Empty);
    }
    pub fn edge(&mut self, last: bool, layout: Layout) {
        if !last {
            self.offset = 0;
        }
        let list = self.list(layout);
        let index = if last {
            list.len().saturating_sub(1)
        } else {
            0
        };
        self.cursor = list.cursor(index).unwrap_or(Cursor::Empty);
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
        if let Some(targets) = self.current_mark_targets() {
            self.edit_marks(MarkEdit::Toggle(targets));
        }
    }

    pub fn toggle_visible_group(&mut self) {
        if let Some(targets) = self.visible_mark_targets() {
            self.edit_marks(MarkEdit::Toggle(targets));
        }
    }

    pub fn current_mark_targets(&self) -> Option<MarkTargets> {
        let group = self.group()?;
        let visible = self.filtered_indices();
        let indices = match &self.cursor {
            Cursor::Empty => return None,
            Cursor::File(index) => [*visible.get(*index)?].into(),
            Cursor::Directory { path, .. } => visible
                .iter()
                .copied()
                .filter(|&index| {
                    let change = &self.snapshot.changes[index];
                    Group::of(change.area) == group
                        && change
                            .location
                            .name()
                            .parent()
                            .is_some_and(|parent| parent.starts_with(path))
                })
                .collect(),
        };
        Some(MarkTargets { group, indices })
    }

    pub fn visible_mark_targets(&self) -> Option<MarkTargets> {
        let group = self.group()?;
        let indices = self
            .filtered_indices()
            .iter()
            .copied()
            .filter(|&index| Group::of(self.snapshot.changes[index].area) == group)
            .collect();
        Some(MarkTargets { group, indices })
    }

    pub fn edit_marks(&mut self, edit: MarkEdit) {
        let (MarkTargets { group, indices }, toggle) = match edit {
            MarkEdit::Toggle(targets) => (targets, true),
            MarkEdit::Add(targets) => (targets, false),
            MarkEdit::Clear => {
                self.selected.clear();
                return;
            }
        };
        if indices.is_empty() {
            return;
        }
        let mut marked = self.selected.take(group);
        if toggle && indices.is_subset(&marked) {
            marked.retain(|index| !indices.contains(index));
        } else {
            marked.extend(indices);
        }
        self.selected.replace(group, marked);
    }

    #[cfg(test)]
    pub(crate) fn mark_file(&mut self, index: usize) {
        self.edit_marks(MarkEdit::Add(MarkTargets {
            group: Group::of(self.snapshot.changes[index].area),
            indices: [index].into(),
        }));
    }

    pub fn check_state(&self, row: &Row, group: Group) -> CheckState {
        let Some(marked) = self.selected.indices(group) else {
            return CheckState::Unchecked;
        };
        let indices = self.filtered_indices();
        let checked = row
            .indices()
            .iter()
            .filter(|&&index| marked.contains(&indices[index]))
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
