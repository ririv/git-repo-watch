//! File-list projection keeps directory headings separate from selectable changes.
use super::Layout;
use crate::Change;
use std::{collections::BTreeMap, ffi::OsString};

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Row {
    Directory {
        following: usize,
        label: String,
    },
    File {
        index: usize,
        prefix: String,
        label: String,
    },
}
#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    File,
    Directory,
}
enum Node<'a> {
    Directory(BTreeMap<(OsString, Kind), Node<'a>>),
    File(usize, &'a Change),
}
impl Node<'_> {
    fn first(&self) -> usize {
        match self {
            Self::File(index, _) => *index,
            Self::Directory(children) => children
                .first_key_value()
                .expect("nonempty directory")
                .1
                .first(),
        }
    }
}
pub(crate) fn project(changes: &[(usize, &Change)], layout: Layout) -> Vec<Row> {
    if layout == Layout::Flat {
        return changes
            .iter()
            .map(|(index, change)| Row::File {
                index: *index,
                prefix: String::new(),
                label: change.location.display(),
            })
            .collect();
    }
    let mut roots = BTreeMap::new();
    for (index, change) in changes {
        let components = change
            .location
            .name()
            .components()
            .map(|part| part.as_os_str().to_owned())
            .collect::<Vec<_>>();
        let mut children = &mut roots;
        for component in &components[..components.len() - 1] {
            let node = children
                .entry((component.clone(), Kind::Directory))
                .or_insert_with(|| Node::Directory(BTreeMap::new()));
            let Node::Directory(next) = node else {
                unreachable!("Git paths cannot contain a file as a directory")
            };
            children = next;
        }
        children.insert(
            (components.last().expect("change path").clone(), Kind::File),
            Node::File(*index, change),
        );
    }
    let mut rows = Vec::new();
    emit(&roots, "", &mut rows);
    rows
}
fn emit(children: &BTreeMap<(OsString, Kind), Node<'_>>, prefix: &str, rows: &mut Vec<Row>) {
    for (position, ((name, _), node)) in children.iter().enumerate() {
        let last = position + 1 == children.len();
        let branch = if last { "└─ " } else { "├─ " };
        match node {
            Node::Directory(nested) => {
                let mut nested = nested;
                let mut label = name.to_string_lossy().into_owned();
                // 只压缩单一子目录链；遇到文件或分叉就保留层级。
                while nested.len() == 1 {
                    let ((child_name, _), child) = nested.first_key_value().expect("single child");
                    let Node::Directory(children) = child else {
                        break;
                    };
                    label.push('/');
                    label.push_str(&child_name.to_string_lossy());
                    nested = children;
                }
                rows.push(Row::Directory {
                    following: node.first(),
                    label: format!("{prefix}{branch}{label}/"),
                });
                emit(
                    nested,
                    &format!("{prefix}{}", if last { "   " } else { "│  " }),
                    rows,
                );
            }
            Node::File(index, change) => {
                // 重命名保留两端路径，避免同名文件隐藏来源信息。
                let label = match &change.location {
                    engine::local::files::Location::Path(_) => name.to_string_lossy().into_owned(),
                    _ => change.location.display(),
                };
                rows.push(Row::File {
                    index: *index,
                    prefix: format!("{prefix}{branch}"),
                    label,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine::local::files::{Area, Location};
    fn change(path: &str) -> Change {
        Change {
            area: Area::Untracked,
            kind: '?',
            location: Location::Path(path.into()),
        }
    }
    #[test]
    fn nested_paths_keep_file_identity_unicode_and_sibling_connectors() {
        let files = [
            change("src/中文 目录/a.ts"),
            change("src/中文 目录/b.ts"),
            change("z.txt"),
        ];
        let changes = files.iter().enumerate().collect::<Vec<_>>();
        let tree = project(&changes, Layout::Tree);
        assert_eq!(
            tree,
            vec![
                Row::Directory {
                    following: 0,
                    label: "├─ src/中文 目录/".into()
                },
                Row::File {
                    index: 0,
                    prefix: "│  ├─ ".into(),
                    label: "a.ts".into()
                },
                Row::File {
                    index: 1,
                    prefix: "│  └─ ".into(),
                    label: "b.ts".into()
                },
                Row::File {
                    index: 2,
                    prefix: "└─ ".into(),
                    label: "z.txt".into()
                },
            ]
        );
        assert_eq!(project(&changes, Layout::Flat).len(), 3);
        let filtered = project(&[(1, &files[1])], Layout::Tree);
        assert_eq!(filtered.len(), 2);
        assert!(
            matches!(&filtered[0], Row::Directory{following: 1, label} if label == "└─ src/中文 目录/")
        );
    }
    #[test]
    fn deleted_file_and_replacement_directory_can_coexist_and_renames_keep_source() {
        let mut deleted = change("src");
        deleted.kind = 'D';
        deleted.area = Area::Unstaged;
        let nested = change("src/new.rs");
        let renamed = Change {
            area: Area::Staged,
            kind: 'R',
            location: Location::Rename {
                before: "old/a.rs".into(),
                after: "src/a.rs".into(),
            },
        };
        let rows = project(&[(0, &deleted), (1, &nested), (2, &renamed)], Layout::Tree);
        assert_eq!(rows.len(), 4);
        assert!(matches!(&rows[0], Row::File { index: 0, .. }));
        assert!(
            matches!(&rows[2], Row::File {index:2,label,..} if label.contains("old/a.rs") && label.contains("src/a.rs"))
        );
        assert!(project(&[], Layout::Tree).is_empty());
    }
    #[test]
    fn compact_chains_stop_at_files_and_forks() {
        let files = [
            change("output/imagegen/cache/a.png"),
            change("output/imagegen/thumb/b.png"),
            change("src/lib.rs"),
            change("src/nested/deep/main.rs"),
        ];
        let changes = files.iter().enumerate().collect::<Vec<_>>();
        let rows = project(&changes, Layout::Tree);
        let labels = rows
            .iter()
            .filter_map(|row| match row {
                Row::Directory { label, .. } => Some(label.as_str()),
                Row::File { .. } => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            labels,
            [
                "├─ output/imagegen/",
                "│  ├─ cache/",
                "│  └─ thumb/",
                "└─ src/",
                "   └─ nested/deep/"
            ]
        );
        let indices = rows
            .iter()
            .filter_map(|row| match row {
                Row::File { index, .. } => Some(*index),
                Row::Directory { .. } => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(indices, [0, 1, 2, 3]);
        let flat = project(&changes, Layout::Flat);
        assert_eq!(flat.len(), files.len());
        assert!(
            matches!(&flat[0], Row::File {index: 0, label, ..} if label.contains("output/imagegen/cache/a.png"))
        );
    }
}
