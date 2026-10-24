//! Bounded file previews never traverse symbolic links or read special files.
use super::checked_path;
use crate::{
    cancel::Cancel,
    local::files::{
        media::{Format, Image, MAX_IMAGE_BYTES},
        patch::MAX_SOURCE_BYTES,
    },
};
use anyhow::{Context, Result, ensure};
use std::{
    fs::{self, OpenOptions},
    io::Read,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub enum Content {
    Text { path: PathBuf, text: String },
    Image(Image),
    Symlink(PathBuf),
    Missing,
    Binary,
    TooLarge,
    Special,
    Directory,
}
pub fn read(root: &Path, path: &Path, cancel: &Cancel) -> Result<Content> {
    cancel.check()?;
    super::super::open(root)?;
    let full = checked_path(root, path)?;
    let metadata = match fs::symlink_metadata(&full) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Content::Missing),
        Err(error) => return Err(error.into()),
    };
    if metadata.is_symlink() {
        return Ok(Content::Symlink(fs::read_link(full)?));
    }
    if metadata.is_dir() {
        return Ok(Content::Directory);
    }
    if !metadata.is_file() {
        return Ok(Content::Special);
    }
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(full)?;
    ensure!(file.metadata()?.is_file(), i18n::tr!(BrowserSpecialFile));
    let mut bytes = Vec::new();
    file.by_ref().take(12).read_to_end(&mut bytes)?;
    let format = Format::detect(&bytes);
    let limit = if format.is_some() {
        MAX_IMAGE_BYTES
    } else {
        MAX_SOURCE_BYTES
    };
    // 逐块检查取消，并仅多读一个字节来识别超限内容。
    let mut reader = file.take((limit + 1 - bytes.len()) as u64);
    let mut buffer = [0; 16384];
    loop {
        cancel.check()?;
        let size = reader.read(&mut buffer)?;
        if size == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..size]);
    }
    if bytes.len() > limit {
        return Ok(Content::TooLarge);
    }
    if let Some(format) = format {
        return Ok(Content::Image(Image {
            path: path.to_owned(),
            format,
            bytes,
        }));
    }
    if bytes.contains(&0) {
        return Ok(Content::Binary);
    }
    let text = String::from_utf8(bytes).context(i18n::tr!(BrowserNotUtf8))?;
    Ok(Content::Text {
        path: path.to_owned(),
        text,
    })
}
