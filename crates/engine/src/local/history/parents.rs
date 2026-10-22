//! Preserve original parent identity when libgit2 cuts traversal at shallow or graft boundaries.
use super::Oid;
use anyhow::{Result, ensure};
use git2::{Commit, Repository};

#[derive(Debug, Clone)]
pub struct Parents(Vec<Oid>);
impl Parents {
    pub fn as_slice(&self) -> &[Oid] {
        &self.0
    }
}

#[derive(Debug, Clone)]
pub enum Ancestry {
    Root,
    Linked(Parents),
    Boundary(Parents),
}
impl Ancestry {
    pub fn parents(&self) -> &[Oid] {
        match self {
            Self::Root => &[],
            Self::Linked(parents) | Self::Boundary(parents) => parents.as_slice(),
        }
    }
}

pub(super) fn original(repo: &Repository, commit: &Commit<'_>) -> Result<Vec<Oid>> {
    commit
        .raw_header_bytes()
        .split(|byte| *byte == b'\n')
        .filter_map(|line| line.strip_prefix(b"parent "))
        .map(|value| {
            let text = std::str::from_utf8(value)?;
            Ok(Oid::from_str_ext(text, repo.object_format())?)
        })
        .collect()
}

pub(super) fn ancestry(repo: &Repository, commit: &Commit<'_>) -> Result<Ancestry> {
    let parents = original(repo, commit)?;
    let effective: Vec<_> = commit.parent_ids().collect();
    // 浅克隆的截断点仍有真实父提交，不能当成根提交，也不能画向不存在的本地历史。
    if parents.is_empty() {
        return Ok(Ancestry::Root);
    }
    if effective.is_empty() {
        return Ok(Ancestry::Boundary(Parents(parents)));
    }
    ensure!(parents == effective, i18n::tr!(HistoryGraftedParents));
    Ok(Ancestry::Linked(Parents(parents)))
}
