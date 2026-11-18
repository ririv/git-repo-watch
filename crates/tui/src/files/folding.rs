//! Group-scoped folds affect projection, never file identities or batch selection.
use super::{Cursor, FileSession, Group, Layout, tree::Row};
use crate::Change;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Expansion {
    Expanded,
    Collapsed,
}

#[derive(Debug, Clone, Default)]
pub(super) struct Folding {
    normal: [BTreeSet<PathBuf>; 2],
    search: [BTreeSet<PathBuf>; 2],
}
impl Folding {
    fn collapsed(&self, group: Group, query: &str) -> &BTreeSet<PathBuf> {
        if query.is_empty() {
            &self.normal[group.index()]
        } else {
            &self.search[group.index()]
        }
    }
    pub fn expansion(&self, group: Group, head: &Path, tip: &Path, query: &str) -> Expansion {
        if self
            .collapsed(group, query)
            .iter()
            .any(|path| path.starts_with(head) && tip.starts_with(path))
        {
            Expansion::Collapsed
        } else {
            Expansion::Expanded
        }
    }
    fn toggle(
        &mut self,
        group: Group,
        head: PathBuf,
        tip: &Path,
        expansion: Expansion,
        query: &str,
    ) {
        let collapsed = if query.is_empty() {
            &mut self.normal[group.index()]
        } else {
            &mut self.search[group.index()]
        };
        match expansion {
            Expansion::Expanded => {
                collapsed.insert(head);
            }
            Expansion::Collapsed => {
                collapsed.retain(|path| !(path.starts_with(&head) && tip.starts_with(path)))
            }
        }
    }
    pub fn reset_search(&mut self) {
        self.search.iter_mut().for_each(BTreeSet::clear);
    }
    pub fn retain(&mut self, changes: &[Change]) {
        for groups in [&mut self.normal, &mut self.search] {
            for (index, collapsed) in groups.iter_mut().enumerate() {
                collapsed.retain(|head| {
                    changes.iter().any(|change| {
                        Group::of(change.area) == Group::from_index(index)
                            && change
                                .location
                                .name()
                                .parent()
                                .is_some_and(|parent| parent.starts_with(head))
                    })
                });
            }
        }
    }
}
impl FileSession {
    pub fn toggle_directory(&mut self, layout: Layout) -> bool {
        let Cursor::Directory { group, .. } = self.cursor else {
            return false;
        };
        let entries = self.entries(layout);
        let Some(Row::Directory {
            head,
            path,
            expansion,
            ..
        }) = entries[group.index()]
            .iter()
            .find(|row| row.cursor(group) == self.cursor)
        else {
            return false;
        };
        self.folding
            .toggle(group, head.clone(), path, *expansion, &self.query);
        self.invalidate_layout();
        true
    }
    pub fn locate(&mut self, index: usize) {
        let rows = self.rows();
        let change = rows[index];
        let group = Group::of(change.area);
        let path = change.location.name().to_owned();
        let collapsed = if self.query.is_empty() {
            &mut self.folding.normal[group.index()]
        } else {
            &mut self.folding.search[group.index()]
        };
        let before = collapsed.len();
        collapsed.retain(|head| !path.starts_with(head));
        if collapsed.len() != before {
            self.invalidate_layout();
        }
        self.cursor = Cursor::File(index);
    }
    pub(super) fn ensure_visible(&mut self) {
        if self.list(Layout::Tree).position(&self.cursor).is_some() {
            return;
        }
        let entries = self.entries(Layout::Tree);
        let Some(group) = self.group() else {
            return;
        };
        let rows = &entries[group.index()];
        let path = match &self.cursor {
            Cursor::File(_) => self.current().expect("focused file").location.name(),
            Cursor::Directory { path, .. } => path,
            Cursor::Empty => return,
        };
        // 搜索结束或压缩链变化时，选中仍可见的祖先，不展开用户收起的目录。
        if let Some(row) = rows.iter().rev().find(|row| match row {
            Row::Directory {
                head, path: tip, ..
            } => path.starts_with(tip) || (path.starts_with(head) && tip.starts_with(path)),
            Row::File { .. } => false,
        }) {
            self.cursor = row.cursor(group);
        } else {
            self.cursor = rows
                .first()
                .map(|row| row.cursor(group))
                .or_else(|| {
                    entries.iter().enumerate().find_map(|(index, rows)| {
                        rows.first().map(|row| row.cursor(Group::from_index(index)))
                    })
                })
                .unwrap_or(Cursor::Empty);
        }
    }
}
