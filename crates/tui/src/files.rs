//! Shared file selection with explicit staged and unstaged identities.
mod cursor;
mod folding;
pub(crate) mod list;
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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct Marks(Option<(Group, BTreeSet<usize>)>);

impl Marks {
    fn indices(&self, group: Group) -> Option<&BTreeSet<usize>> {
        self.0
            .as_ref()
            .filter(|(active, _)| *active == group)
            .map(|(_, indices)| indices)
    }

    fn replace(&mut self, group: Group, indices: BTreeSet<usize>) {
        self.0 = (!indices.is_empty()).then_some((group, indices));
    }

    #[cfg(test)]
    pub fn clear(&mut self) {
        self.0 = None;
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.0.as_ref().map_or(0, |(_, indices)| indices.len())
    }

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.0.is_none()
    }
}

#[derive(Debug, Clone)]
pub(super) struct FileSession {
    pub snapshot: Files,
    scope: Option<Group>,
    pub cursor: Cursor,
    pub offset: usize,
    pub horizontal: crate::horizontal::Viewport,
    pub selected: Marks,
    pub query: String,
    pub recovery: crate::replay::controls::Focus,
    pub input_preference: crate::input_pages::Preference,
    pub search_mode: crate::search::Mode,
    folding: Box<folding::Folding>,
}
impl FileSession {
    pub fn working_target(&self) -> Option<crate::working_target::Target> {
        match &self.cursor {
            Cursor::Directory { path, .. } => {
                Some(crate::working_target::Target::Directory(path.clone()))
            }
            _ => self.current().map(|change| {
                crate::working_target::Target::File(change.location.name().to_owned())
            }),
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
            scope: None,
            cursor,
            offset: 0,
            horizontal: Default::default(),
            selected: Marks::default(),
            query: String::new(),
            recovery: Default::default(),
            input_preference: Default::default(),
            search_mode: Default::default(),
            folding: Default::default(),
        }
    }
    pub fn refreshed(&self, snapshot: Files) -> Self {
        let current = self
            .current()
            .map(|change| (change.area, change.location.clone()));
        let mut next = Self::new(snapshot);
        next.scope = self.scope;
        next.input_preference = self.input_preference.clone();
        next.recovery = self.recovery.refreshed(
            &self.snapshot.operation.state,
            &next.snapshot.operation.state,
        );
        next.offset = self.offset;
        next.horizontal = self.horizontal.clone();
        next.query = self.query.clone();
        next.search_mode = self.search_mode;
        next.folding = self.folding.clone();
        next.folding.retain(&next.snapshot.changes);
        if next.rows().is_empty() {
            next.cursor = Cursor::Empty;
        }
        if let Some((group, indices)) = &self.selected.0 {
            let selected = indices
                .iter()
                .map(|index| &self.snapshot.changes[*index])
                .collect::<Vec<_>>();
            next.selected.replace(
                *group,
                next.snapshot
                    .changes
                    .iter()
                    .enumerate()
                    .filter(|(_, change)| {
                        selected
                            .iter()
                            .any(|old| old.area == change.area && old.location == change.location)
                    })
                    .map(|(index, _)| index)
                    .collect(),
            );
        }
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
    pub fn scope(&self) -> Option<Group> {
        self.scope
    }

    pub fn with_scope(mut self, scope: Option<Group>) -> Self {
        if self.scope == scope {
            return self;
        }
        let current = self.current().cloned();
        let previous = self.cursor.clone();
        self.scope = scope;
        if let Some(group) = scope
            && self
                .selected
                .0
                .as_ref()
                .is_some_and(|(marked, _)| *marked != group)
        {
            self.selected = Marks::default();
        }
        let rows = self.rows();
        self.cursor = if rows.is_empty() {
            Cursor::Empty
        } else if matches!(&previous, Cursor::Directory { group, .. } if scope.is_none_or(|scope| scope == *group))
        {
            previous
        } else {
            Cursor::File(
                current
                    .and_then(|change| rows.iter().position(|row| **row == change))
                    .unwrap_or(0),
            )
        };
        self
    }

    pub fn has_changes(&self) -> bool {
        self.snapshot.changes.iter().any(|change| {
            self.scope
                .is_none_or(|group| Group::of(change.area) == group)
        })
    }

    pub fn rows(&self) -> Vec<&Change> {
        let mut rows = self
            .snapshot
            .changes
            .iter()
            .filter(|change| {
                self.scope
                    .is_none_or(|group| Group::of(change.area) == group)
                    && self.search_mode.includes(self.matches_change(change))
            })
            .collect::<Vec<_>>();
        rows.sort_by(|a, b| {
            (a.area != Area::Staged)
                .cmp(&(b.area != Area::Staged))
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
    pub fn all_marked_changes(&self) -> impl Iterator<Item = &Change> {
        self.selected
            .0
            .as_ref()
            .into_iter()
            .flat_map(|(_, indices)| indices)
            .map(|index| &self.snapshot.changes[*index])
    }

    pub fn marked_changes(&self) -> impl Iterator<Item = &Change> {
        let group = self.group();
        self.all_marked_changes()
            .filter(move |change| Some(Group::of(change.area)) == group)
    }

    pub fn discard_selection(&self) -> Vec<Change> {
        let mut marked = self.marked_changes().peekable();
        if marked.peek().is_some() {
            marked.cloned().collect()
        } else {
            self.current().cloned().into_iter().collect()
        }
    }

    pub fn selection(&self) -> Vec<Change> {
        let mut marked = self.marked_changes().peekable();
        if marked.peek().is_some() {
            marked.cloned().collect()
        } else {
            self.focused_changes().into_iter().cloned().collect()
        }
    }
}
