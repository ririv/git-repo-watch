//! A single scrolling projection includes group headers and selectable tree entries.
use super::{Cursor, FileSession, Group, Layout, tree};

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
impl FileSession {
    pub fn list(&self, layout: Layout) -> Vec<Row> {
        let changes = self.rows();
        let mut rows = Vec::new();
        let mut index = 0;
        for (group, entries) in self.entries(layout).into_iter().enumerate() {
            if entries.is_empty() {
                continue;
            }
            let group = Group::from_index(group);
            rows.push(Row::Header {
                group,
                count: changes
                    .iter()
                    .filter(|change| Group::of(change.area) == group)
                    .count(),
            });
            for entry in entries {
                rows.push(Row::Entry {
                    group,
                    entry,
                    index,
                });
                index += 1;
            }
        }
        rows
    }
    pub fn seek(&mut self, offset: usize, layout: Layout) {
        let rows = self.list(layout);
        self.offset = offset.min(rows.len().saturating_sub(1));
        if let Some(cursor) = rows.iter().skip(self.offset).find_map(Row::cursor) {
            self.cursor = cursor;
        } else if let Some(cursor) = rows.iter().rev().find_map(Row::cursor) {
            self.cursor = cursor;
        }
    }
}
