//! Render projected text with explicit creation and deletion metadata.
use super::*;
use anyhow::Context;
use std::path::Path;

fn tree<'repo>(
    repo: &'repo git2::Repository,
    path: &Path,
    version: &Version,
) -> Result<git2::Tree<'repo>> {
    let mut index = git2::Index::new()?;
    if let Version::Text { data, mode } = version {
        index.add(&git2::IndexEntry {
            ctime: git2::IndexTime::new(0, 0),
            mtime: git2::IndexTime::new(0, 0),
            dev: 0,
            ino: 0,
            mode: if mode & 0o111 == 0 {
                0o100644
            } else {
                0o100755
            },
            uid: 0,
            gid: 0,
            file_size: data.len() as u32,
            id: repo.blob(data.as_bytes())?,
            flags: 0,
            flags_extended: 0,
            path: crate::local::git_path_bytes(path)?.into_owned(),
        })?;
    }
    Ok(repo.find_tree(index.write_tree_to(repo)?)?)
}
impl Plan {
    pub fn preview(&self) -> Result<String> {
        let current = match (&self.snapshot.layer, self.action) {
            (Layer::Working { before, .. }, Action::Stage) => before,
            (Layer::Staged { after, .. }, Action::Unstage)
            | (Layer::Working { after, .. }, Action::Discard) => after,
            _ => unreachable!("prepared plan matches its layer"),
        };
        let repo = crate::local::open(&self.snapshot.files.path)?;
        let path = self.snapshot.change.location.name();
        let before = tree(&repo, path, current)?;
        let after = tree(&repo, path, &self.next)?;
        let mut options = git2::DiffOptions::new();
        options.context_lines(3);
        let diff = repo.diff_tree_to_tree(Some(&before), Some(&after), Some(&mut options))?;
        let mut patch = git2::Patch::from_diff(&diff, 0)?.context(i18n::tr!(PartialChanged))?;
        Ok(String::from_utf8(patch.to_buf()?.to_vec())?)
    }
}
