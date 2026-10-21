//! Bounded, literal-path diffs with untracked and binary handling.
use super::{Area, Change, Files, validate};
use crate::local::{Head, open, utf8_path};
use anyhow::{Context, Result, ensure};
use git2::{DiffFormat, DiffOptions};
use std::fs;
pub fn diff(snapshot: &Files, change: &Change) -> Result<String> {
    let repo = open(&snapshot.path)?;
    validate(&repo, snapshot)?;
    ensure!(snapshot.changes.contains(change), "文件不属于该快照");
    if change.area == Area::Untracked {
        let path = snapshot.path.join(change.location.name());
        let meta = fs::symlink_metadata(&path)?;
        if meta.file_type().is_symlink() {
            return Ok(format!(
                "新符号链接 {:?} → {:?}",
                change.location.name(),
                fs::read_link(path)?
            ));
        }
        if !meta.is_file() {
            return Ok("目录或特殊文件，无法显示文本 diff".into());
        }
        if meta.len() > 1024 * 1024 {
            return Ok(format!(
                "未跟踪大文件（{} 字节），请使用外部工具查看",
                meta.len()
            ));
        }
    }
    let mut options = DiffOptions::new();
    options
        .disable_pathspec_match(true)
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .show_untracked_content(true)
        .include_typechange(true);
    for path in change.location.paths() {
        options.pathspec(utf8_path(path)?);
    }
    let tree = match snapshot.head {
        Head::Initial { .. } => None,
        Head::Existing { oid, .. } => Some(repo.find_commit(oid)?.tree()?),
    };
    let diff = if change.area == Area::Staged {
        repo.diff_tree_to_index(tree.as_ref(), None, Some(&mut options))?
    } else {
        repo.diff_index_to_workdir(None, Some(&mut options))?
    };
    let mut out = Vec::new();
    let mut truncated = false;
    diff.print(DiffFormat::Patch, |_, _, line| {
        if truncated {
            return true;
        }
        if out.len() + line.content().len() + 1 > 128 * 1024 {
            truncated = true;
            return true;
        }
        if matches!(line.origin(), '+' | '-' | ' ') {
            out.push(line.origin() as u8)
        }
        out.extend_from_slice(line.content());
        true
    })?;
    let mut text = String::from_utf8(out).context("差异内容不是 UTF-8，请使用外部工具查看")?;
    if truncated {
        text.push_str("\n… diff 超过 128 KiB，仅显示前段。")
    }
    if text.is_empty() {
        text = "当前文件没有可显示的文本差异（可能为二进制或子模块状态变化）".into()
    }
    Ok(text)
}
