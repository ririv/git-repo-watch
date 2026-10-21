//! Read bounded blobs and regular worktree files without following symlinks.
use super::patch::{MAX_SOURCE_BYTES, Source, SourceOmission};
use anyhow::{Context, Result};
use git2::{FileMode, Oid, Repository};
use std::{fs::OpenOptions, io::Read, os::unix::fs::OpenOptionsExt, path::PathBuf};

pub(super) struct FileInfo {
    pub path: PathBuf,
    pub oid: Oid,
    pub mode: FileMode,
    pub binary: bool,
    pub exists: bool,
}
#[derive(Clone, Copy)]
pub(super) enum Storage {
    Object,
    Worktree,
}
impl FileInfo {
    pub fn read(&self, repo: &Repository, storage: Storage) -> Result<Source> {
        if !self.exists {
            return Ok(Source::Absent);
        }
        if self.binary {
            return Ok(Source::Unavailable(SourceOmission::Binary));
        }
        if !matches!(
            self.mode,
            FileMode::Blob | FileMode::BlobExecutable | FileMode::BlobGroupWritable
        ) {
            return Ok(Source::Unavailable(SourceOmission::NonRegular));
        }
        let bytes = if matches!(storage, Storage::Worktree) {
            let path = repo
                .workdir()
                .context(i18n::tr!(BareRepositoriesAreNotSupported))?
                .join(&self.path);
            // 拒绝符号链接替换和特殊文件；只读取上限加一个字节，防止读取期间文件增长。
            let file = OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
                .open(path)?;
            if !file.metadata()?.is_file() {
                return Ok(Source::Unavailable(SourceOmission::NonRegular));
            }
            let mut bytes = Vec::new();
            file.take(MAX_SOURCE_BYTES as u64 + 1)
                .read_to_end(&mut bytes)?;
            bytes
        } else {
            let blob = repo.find_blob(self.oid)?;
            if blob.size() > MAX_SOURCE_BYTES {
                return Ok(Source::Unavailable(SourceOmission::TooLarge));
            }
            blob.content().to_vec()
        };
        if bytes.len() > MAX_SOURCE_BYTES {
            return Ok(Source::Unavailable(SourceOmission::TooLarge));
        }
        if bytes.contains(&0) {
            return Ok(Source::Unavailable(SourceOmission::Binary));
        }
        let text = String::from_utf8(bytes).context(i18n::tr!(DiffIsNotUtf8ViewWith))?;
        Ok(Source::Text {
            path: self.path.clone(),
            text,
        })
    }
}
