//! The filtered changes and tree projection are reused while the selection moves.
use super::{Cursor, FileSession, Group, Layout, tree};
use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
};

#[derive(Debug)]
pub(crate) enum Row {
    Header {
        group: Group,
        count: usize,
    },
    Entry {
        group: Group,
        entry: tree::Row,
        index: usize,
    },
}
impl Row {
    pub fn cursor(&self) -> Option<Cursor> {
        match self {
            Self::Header { .. } => None,
            Self::Entry { group, entry, .. } => Some(entry.cursor(*group)),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(super) struct Projection {
    filtered: Option<Filtered>,
    layout: Option<(Layout, Arc<List>)>,
}
#[derive(Debug, Clone)]
struct Filtered {
    query: String,
    scope: Option<Group>,
    mode: crate::search::Mode,
    indices: Arc<[usize]>,
}
#[derive(Debug, Default)]
pub(crate) struct List {
    pub rows: Vec<Row>,
    items: Vec<usize>,
    positions: HashMap<Cursor, usize>,
    label_widths: [OnceLock<usize>; 2],
}
impl List {
    pub fn position(&self, cursor: &Cursor) -> Option<usize> {
        self.positions.get(cursor).copied()
    }

    pub(super) fn item_index(&self, cursor: &Cursor) -> Option<usize> {
        match &self.rows[self.position(cursor)?] {
            Row::Entry { index, .. } => Some(*index),
            Row::Header { .. } => unreachable!("cursor identifies an entry"),
        }
    }

    pub(super) fn cursor(&self, index: usize) -> Option<Cursor> {
        self.rows.get(*self.items.get(index)?)?.cursor()
    }

    pub(super) fn len(&self) -> usize {
        self.items.len()
    }

    pub fn widest(
        &self,
        icons: crate::file_icons::Display,
        width: impl Fn(&Row) -> usize,
    ) -> usize {
        *self.label_widths[usize::from(icons == crate::file_icons::Display::Enabled)]
            .get_or_init(|| self.rows.iter().map(width).max().unwrap_or(0))
    }
}
impl FileSession {
    pub(super) fn filtered_indices(&self) -> Arc<[usize]> {
        let mut projection = self.projection.borrow_mut();
        if let Some(filtered) = &projection.filtered
            && filtered.query == self.query
            && filtered.scope == self.scope()
            && filtered.mode == self.search_mode
        {
            return filtered.indices.clone();
        }
        let needle = self.query.to_lowercase();
        let changes = &self.snapshot.changes;
        let mut indices = changes
            .iter()
            .enumerate()
            .filter(|(_, change)| {
                self.scope()
                    .is_none_or(|group| Group::of(change.area) == group)
                    && (self.search_mode == crate::search::Mode::Locate
                        || needle.is_empty()
                        || change
                            .location
                            .name()
                            .to_string_lossy()
                            .to_lowercase()
                            .contains(&needle))
            })
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        indices.sort_by(|&left, &right| {
            let (a, b) = (&changes[left], &changes[right]);
            Group::of(a.area)
                .index()
                .cmp(&Group::of(b.area).index())
                .then_with(|| a.location.name().cmp(b.location.name()))
        });
        let indices: Arc<[usize]> = indices.into();
        *projection = Projection {
            filtered: Some(Filtered {
                query: self.query.clone(),
                scope: self.scope(),
                mode: self.search_mode,
                indices: indices.clone(),
            }),
            layout: None,
        };
        indices
    }

    pub(super) fn invalidate_layout(&mut self) {
        self.projection.get_mut().layout = None;
    }

    pub fn entries(&self, layout: Layout) -> [Vec<tree::Row>; 2] {
        self.project_entries(layout, &self.filtered_indices())
    }

    fn project_entries(&self, layout: Layout, indices: &[usize]) -> [Vec<tree::Row>; 2] {
        [Group::Staged, Group::Unstaged].map(|group| {
            let changes = indices
                .iter()
                .enumerate()
                .map(|(index, &original)| (index, &self.snapshot.changes[original]))
                .filter(|(_, change)| Group::of(change.area) == group)
                .collect::<Vec<_>>();
            tree::project(&changes, layout, &|head, tip| {
                self.folding.expansion(group, head, tip, &self.query)
            })
        })
    }

    pub fn list(&self, layout: Layout) -> Arc<List> {
        let changes = self.filtered_indices();
        let mut projection = self.projection.borrow_mut();
        if let Some((cached, list)) = &projection.layout
            && *cached == layout
        {
            return list.clone();
        }
        let mut list = List::default();
        for (group, entries) in self
            .project_entries(layout, &changes)
            .into_iter()
            .enumerate()
        {
            if entries.is_empty() {
                continue;
            }
            let group = Group::from_index(group);
            if self.scope().is_none() {
                list.rows.push(Row::Header {
                    group,
                    count: changes
                        .iter()
                        .filter(|&&index| Group::of(self.snapshot.changes[index].area) == group)
                        .count(),
                });
            }
            for entry in entries {
                let pos = list.rows.len();
                let index = list.items.len();
                list.positions.insert(entry.cursor(group), pos);
                list.items.push(pos);
                list.rows.push(Row::Entry {
                    group,
                    entry,
                    index,
                });
            }
        }
        let list = Arc::new(list);
        projection.layout = Some((layout, list.clone()));
        list
    }

    pub fn seek(&mut self, offset: usize, layout: Layout) {
        let list = self.list(layout);
        self.offset = offset.min(list.rows.len().saturating_sub(1));
        if let Some(cursor) = list.rows.iter().skip(self.offset).find_map(Row::cursor) {
            self.cursor = cursor;
        } else if let Some(cursor) = list.rows.iter().rev().find_map(Row::cursor) {
            self.cursor = cursor;
        }
    }
}
