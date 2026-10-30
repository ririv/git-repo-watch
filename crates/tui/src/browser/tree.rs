//! Directory projection and selection preserve lazily loaded tree state.
use super::*;
impl Session {
    pub fn rows(&self) -> Vec<Row<'_>> {
        if !self.query.is_empty() {
            return match &self.search {
                Search::Ready { query, entries, .. } if *query == self.query => entries
                    .iter()
                    .map(|entry| Row {
                        entry,
                        head: &entry.path,
                        depth: 0,
                    })
                    .collect(),
                _ => vec![],
            };
        }
        fn emit<'a>(session: &'a Session, path: &Path, depth: usize, rows: &mut Vec<Row<'a>>) {
            let Some(directory) = session.directories.get(path).and_then(Contents::directory)
            else {
                return;
            };
            for head in &directory.entries {
                let mut entry = head;
                // 与 diff 树相同：只合并单一子目录链，文件、分叉和叶节点保留层级。
                while entry.directory() {
                    if session.navigation.path() == Some(entry.path.as_path()) {
                        break;
                    }
                    let Some(children) = session
                        .directories
                        .get(&entry.path)
                        .and_then(Contents::directory)
                    else {
                        break;
                    };
                    let [child] = children.entries.as_slice() else {
                        break;
                    };
                    if !child.directory() {
                        break;
                    }
                    entry = child;
                }
                rows.push(Row {
                    entry,
                    head: &head.path,
                    depth,
                });
                if entry.directory() && session.expanded.contains(&head.path) {
                    emit(session, &entry.path, depth + 1, rows);
                }
            }
        }
        let mut rows = Vec::new();
        emit(self, Path::new(""), 0, &mut rows);
        rows
    }
    pub fn selected_row(&self) -> Option<Row<'_>> {
        self.state
            .selected()
            .and_then(|i| self.rows().get(i).copied())
    }
    pub fn selected(&self) -> Option<Entry> {
        self.selected_row().map(|row| row.entry.clone())
    }
    pub fn select(&mut self, index: usize) {
        let row = self.rows();
        let selected = row
            .get(index.min(row.len().saturating_sub(1)))
            .map(|row| row.entry.path.clone());
        self.navigation = Default::default();
        let rows = self.rows();
        let index = selected
            .and_then(|path| {
                rows.iter().position(|row| {
                    row.entry.path == path
                        || (path.starts_with(row.head) && row.entry.path.starts_with(&path))
                })
            })
            .unwrap_or(index);
        self.state
            .select(rows.len().checked_sub(1).map(|last| index.min(last)));
        self.sync_preview();
    }
    pub fn advance(&mut self, delta: isize) {
        self.select(
            self.state
                .selected()
                .map_or(0, |selected| selected.saturating_add_signed(delta)),
        );
    }
    pub(super) fn load(&mut self, path: PathBuf) {
        if matches!(
            self.directories.get(&path),
            Some(Contents::Loading | Contents::Ready(_) | Contents::Refreshing(_))
        ) {
            return;
        }
        self.directories.insert(path.clone(), Contents::Loading);
        self.queued.push_back(path);
    }
    pub fn expanded(&self, path: &Path) -> bool {
        self.expanded.contains(path)
    }
    pub(super) fn probe_visible(&mut self) {
        if !self.query.is_empty() {
            return;
        }
        let directories = self
            .rows()
            .into_iter()
            .filter(|row| row.entry.directory())
            .map(|row| (row.head.to_owned(), row.entry.path.clone()))
            .collect::<Vec<_>>();
        for (head, tip) in directories {
            if self.expanded(&head) {
                for path in tip.ancestors().take_while(|path| path.starts_with(&head)) {
                    self.expanded.insert(path.to_owned());
                }
            }
            // 仅预读可见目录行的链尾；遇到分叉后，不读取尚未展开的子树。
            if matches!(self.directories.get(&tip), Some(Contents::Unloaded)) {
                self.load(tip);
            }
        }
    }
    pub fn toggle(&mut self) {
        let Some(row) = self.selected_row() else {
            return;
        };
        let head = row.head.to_owned();
        let entry = row.entry.clone();
        if !entry.directory() {
            self.focus = Focus::Preview;
            return;
        }
        let expanded = self.expanded(&head);
        for path in entry
            .path
            .ancestors()
            .take_while(|path| path.starts_with(&head))
        {
            if expanded {
                self.expanded.remove(path);
            } else {
                self.expanded.insert(path.to_owned());
            }
        }
        if !expanded {
            self.load(entry.path);
        }
        self.probe_visible();
    }
    pub fn parent(&mut self) -> bool {
        let Some(row) = self.selected_row() else {
            return false;
        };
        if row.entry.directory() && self.expanded(row.head) {
            self.toggle();
            return true;
        }
        let depth = row.depth;
        let selected = self.state.selected().expect("selected browser row");
        let parent = self.rows()[..selected]
            .iter()
            .rposition(|row| row.depth < depth);
        if let Some(index) = parent {
            self.select(index);
            return true;
        }
        false
    }
}
