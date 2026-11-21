//! File publication and metadata operations with explicit platform semantics.
#[cfg(unix)]
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
#[cfg(windows)]
use std::os::windows::fs::{FileTypeExt, MetadataExt, OpenOptionsExt};
use std::{
    fs::{self, File, Metadata, OpenOptions},
    io::{self, ErrorKind},
    path::Path,
};

pub(crate) fn regular_file(path: &Path) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    #[cfg(windows)]
    options.custom_flags(windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT);
    let file = options.open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() || is_link(&metadata) {
        return Err(io::Error::new(
            ErrorKind::InvalidInput,
            i18n::tr!(MutationNonRegularFile),
        ));
    }
    Ok(file)
}

pub(crate) fn is_link(metadata: &Metadata) -> bool {
    #[cfg(unix)]
    return metadata.is_symlink();
    #[cfg(windows)]
    return metadata.file_attributes()
        & windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT
        != 0;
}

pub(crate) fn private_path(path: &Path, mode: u32) -> io::Result<()> {
    #[cfg(unix)]
    return fs::set_permissions(path, fs::Permissions::from_mode(mode));
    #[cfg(windows)]
    return super::windows::restrict_to_user(path, mode == 0o700);
}

pub(crate) fn worktree_mode(file: &File, mode: u32) -> io::Result<()> {
    #[cfg(unix)]
    return file.set_permissions(fs::Permissions::from_mode(mode));
    #[cfg(windows)]
    {
        let mut permissions = file.metadata()?.permissions();
        permissions.set_readonly(mode & 0o222 == 0);
        file.set_permissions(permissions)
    }
}

pub(crate) fn original_mode(metadata: &Metadata) -> u32 {
    #[cfg(unix)]
    return metadata.mode();
    #[cfg(windows)]
    return 0o100000
        | if metadata.permissions().readonly() {
            0o444
        } else {
            0o644
        };
}

pub(crate) fn replace(source: &Path, destination: &Path) -> io::Result<()> {
    #[cfg(unix)]
    return fs::rename(source, destination);
    #[cfg(windows)]
    return super::windows::move_file(source, destination, true);
}

pub(crate) fn publish(
    temporary: tempfile::NamedTempFile,
    destination: &Path,
    overwrite: bool,
) -> io::Result<()> {
    #[cfg(unix)]
    {
        let result = if overwrite {
            temporary.persist(destination)
        } else {
            temporary.persist_noclobber(destination)
        };
        result.map(|_| ()).map_err(|error| error.error)
    }
    #[cfg(windows)]
    {
        let mut path = temporary.into_temp_path();
        super::windows::move_file(&path, destination, overwrite)?;
        path.disable_cleanup(true);
        Ok(())
    }
}

pub(crate) fn sync_directory(path: &Path) -> io::Result<()> {
    #[cfg(unix)]
    return File::open(path)?.sync_all();
    #[cfg(windows)]
    {
        // Windows 的发布步骤使用写透移动，目录句柄无需另行刷新。
        let _ = path;
        Ok(())
    }
}

pub(crate) fn symlink_file(target: &Path, link: &Path) -> io::Result<()> {
    #[cfg(unix)]
    return std::os::unix::fs::symlink(target, link);
    #[cfg(windows)]
    return std::os::windows::fs::symlink_file(target, link);
}

pub(crate) fn is_file_link(metadata: &Metadata) -> bool {
    #[cfg(unix)]
    return metadata.is_symlink();
    #[cfg(windows)]
    return metadata.file_type().is_symlink_file();
}

pub(crate) fn find_program(
    name: &str,
    path: &std::ffi::OsStr,
    base: &Path,
) -> io::Result<Option<std::path::PathBuf>> {
    #[cfg(unix)]
    let filename = name;
    #[cfg(windows)]
    let executable = format!("{name}.exe");
    #[cfg(windows)]
    let filename = if Path::new(name).extension().is_none() {
        executable.as_str()
    } else {
        name
    };
    for directory in std::env::split_paths(path) {
        let candidate = base.join(directory).join(filename);
        match fs::metadata(&candidate) {
            Ok(metadata) if executable_file(&metadata) => return Ok(Some(candidate)),
            Ok(_) => {}
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(None)
}

fn executable_file(metadata: &Metadata) -> bool {
    #[cfg(unix)]
    return metadata.is_file() && metadata.permissions().mode() & 0o111 != 0;
    #[cfg(windows)]
    return metadata.is_file();
}
