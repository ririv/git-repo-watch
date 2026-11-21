//! Read bounded blobs and regular worktree files without following symlinks.
use super::{
    media::{Format, Image, MAX_IMAGE_BYTES},
    patch::{MAX_SOURCE_BYTES, Source, SourceOmission},
};
use crate::platform::files as platform_files;
use anyhow::{Context, Result};
use git2::{FileMode, Oid, Repository};
use std::{io::Read, path::PathBuf};

pub(super) struct FileInfo {
    pub path: PathBuf,
    pub oid: Oid,
    pub mode: FileMode,
    pub binary: bool,
    pub exists: bool,
}
#[derive(Clone, Copy)]
pub(crate) enum Storage {
    Object,
    Worktree,
}
impl FileInfo {
    pub fn read(&self, repo: &Repository, storage: Storage) -> Result<Source> {
        if !self.exists {
            return Ok(Source::Absent);
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
            let mut file = platform_files::regular_file(&path)?;
            if !file.metadata()?.is_file() {
                return Ok(Source::Unavailable(SourceOmission::NonRegular));
            }
            let mut bytes = Vec::new();
            file.by_ref().take(12).read_to_end(&mut bytes)?;
            let format = Format::detect(&bytes);
            if format.is_none() && self.binary {
                return Ok(Source::Unavailable(SourceOmission::Binary));
            }
            let limit = if format.is_some() {
                MAX_IMAGE_BYTES
            } else {
                MAX_SOURCE_BYTES
            };
            file.take((limit + 1 - bytes.len()) as u64)
                .read_to_end(&mut bytes)?;
            bytes
        } else {
            let blob = repo.find_blob(self.oid)?;
            let format = Format::detect(blob.content());
            if format.is_none() && self.binary {
                return Ok(Source::Unavailable(SourceOmission::Binary));
            }
            if format.is_some() && blob.size() > MAX_IMAGE_BYTES {
                anyhow::bail!(i18n::tr!(ImageEncodedLimit));
            }
            if format.is_none() && blob.size() > MAX_SOURCE_BYTES {
                return Ok(Source::Unavailable(SourceOmission::TooLarge));
            }
            blob.content().to_vec()
        };
        if let Some(format) = Format::detect(&bytes) {
            anyhow::ensure!(bytes.len() <= MAX_IMAGE_BYTES, i18n::tr!(ImageEncodedLimit));
            return Ok(Source::Image(Image {
                path: self.path.clone(),
                format,
                bytes,
            }));
        }
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
