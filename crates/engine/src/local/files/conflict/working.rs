//! Read the actual result independently of whether Git produces a patch.
use super::*;
use std::fs;

pub fn read(files: &Files, change: &Change, cancel: &Cancel) -> Result<Source> {
    cancel.check()?;
    ensure!(
        files.changes.contains(change) && change.kind == 'U',
        i18n::tr!(ConflictFileRequired)
    );
    let repo = open(&files.path)?;
    super::super::validate(&repo, files)?;
    let path =
        super::super::partial::capture::checked_worktree_path(&files.path, change.location.name())?;
    let source = match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() => FileInfo {
            path: change.location.name().to_owned(),
            oid: Oid::ZERO_SHA1,
            mode: FileMode::Blob,
            binary: false,
            exists: true,
        }
        .read(&repo, Storage::Worktree)?,
        Ok(_) => Source::Unavailable(super::super::patch::SourceOmission::NonRegular),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Source::Absent,
        Err(error) => return Err(error.into()),
    };
    super::super::validate(&repo, files)?;
    cancel.check()?;
    Ok(source)
}
