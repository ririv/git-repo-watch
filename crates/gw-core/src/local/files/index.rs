//! Literal-path index edits without changing worktree contents.
use super::{Area, Change, Files, IndexAction, read, validate};
use crate::local::{Head, open};
use anyhow::{Context, Result, ensure};
use git2::{IndexEntry, Repository};
use std::{collections::HashSet, fs, path::Path};
pub fn edit(snapshot: &Files, action: IndexAction, selection: &[Change]) -> Result<Files> {
    ensure!(!selection.is_empty(), "请先选择文件");
    let repo = open(&snapshot.path)?;
    validate(&repo, snapshot)?;
    let mut paths = Vec::new();
    let mut seen = HashSet::new();
    for change in selection {
        ensure!(
            snapshot.changes.contains(change),
            "文件来自不同的状态快照，请刷新后重新选择"
        );
        ensure!(
            match action {
                IndexAction::Stage => change.area != Area::Staged,
                IndexAction::Unstage => change.area == Area::Staged,
            },
            "文件区域与操作不匹配"
        );
        for path in change.location.paths() {
            if seen.insert(path) {
                paths.push(path.to_owned());
            }
        }
    }
    let mut index = repo.index()?;
    match action {
        IndexAction::Stage => {
            for path in &paths {
                let full = snapshot.path.join(path);
                match fs::symlink_metadata(&full) {
                    Ok(info) => {
                        if info.is_dir() {
                            let sub = open(&full).context("目录只支持暂存子模块提交")?;
                            let oid = sub.head()?.target().context("子模块没有提交")?;
                            let entry = IndexEntry {
                                ctime: git2::IndexTime::new(0, 0),
                                mtime: git2::IndexTime::new(0, 0),
                                dev: 0,
                                ino: 0,
                                mode: 0o160000,
                                uid: 0,
                                gid: 0,
                                file_size: 0,
                                id: oid,
                                flags: 0,
                                flags_extended: 0,
                                path: path.as_os_str().as_encoded_bytes().to_vec(),
                            };
                            index.add(&entry)?;
                        } else {
                            require_supported_filters(&repo, path)?;
                            index.add_path(path)?;
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                        if index.get_path(path, 0).is_some() {
                            index.remove_path(path)?;
                        }
                    }
                    Err(e) => return Err(e.into()),
                }
            }
        }
        IndexAction::Unstage => {
            let tree = match snapshot.head {
                Head::Initial { .. } => None,
                Head::Existing { oid, .. } => Some(repo.find_commit(oid)?.tree()?),
            };
            let source = match &tree {
                Some(tree) => {
                    let mut source = git2::Index::new()?;
                    source.read_tree(tree)?;
                    Some(source)
                }
                None => None,
            };
            for path in &paths {
                if let Some(entry) = source.as_ref().and_then(|s| s.get_path(path, 0)) {
                    index.add(&entry)?;
                } else if index.get_path(path, 0).is_some() {
                    index.remove_path(path)?;
                }
            }
        }
    }
    // libgit2 在 write 时获取 index.lock；写入前再次检查预览快照。
    validate(&open(&snapshot.path)?, snapshot)?;
    index.write()?;
    read(&snapshot.path).context("暂存操作已执行，但刷新失败")
}
fn require_supported_filters(repo: &Repository, path: &Path) -> Result<()> {
    let flags = git2::AttrCheckFlags::FILE_THEN_INDEX;
    if let git2::AttrValue::String(name) =
        git2::AttrValue::from_bytes(repo.get_attr_bytes(path, "filter", flags)?)
    {
        let config = repo.config()?;
        let mut configured = false;
        for key in [
            format!("filter.{name}.clean"),
            format!("filter.{name}.process"),
        ] {
            match config.get_string(&key) {
                Ok(_) => configured = true,
                Err(e) if e.code() == git2::ErrorCode::NotFound => {}
                Err(e) => return Err(e.into()),
            }
        }
        let required = match config.get_bool(&format!("filter.{name}.required")) {
            Ok(value) => value,
            Err(e) if e.code() == git2::ErrorCode::NotFound => false,
            Err(e) => return Err(e.into()),
        };
        ensure!(
            !configured && !required,
            "文件 {path:?} 使用外部 filter {name:?}（例如 Git LFS），当前 libgit2 索引后端未注册该过滤器，已拒绝暂存；请使用 Git 显式处理"
        );
    }
    ensure!(
        matches!(
            git2::AttrValue::from_bytes(repo.get_attr_bytes(
                path,
                "working-tree-encoding",
                flags
            )?),
            git2::AttrValue::Unspecified | git2::AttrValue::False
        ),
        "文件 {path:?} 使用 working-tree-encoding，当前后端不支持该转换，已拒绝暂存"
    );
    Ok(())
}
