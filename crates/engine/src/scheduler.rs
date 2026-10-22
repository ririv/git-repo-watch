//! Ownership-checked native installation and platform scheduling.
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::plist;
#[cfg(any(target_os = "linux", test))]
mod linux;
use crate::{
    cancel::Cancel,
    directories::UserDirectories,
    process,
    state::{self, Store},
};
use anyhow::{Context, Result, ensure};
pub use identity::SERVICE_LABEL as LABEL;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    Enable,
    Disable,
    Apply,
}
#[derive(Debug, Clone)]
pub struct Manager {
    user_home: PathBuf,
    #[cfg(target_os = "macos")]
    uid: u32,
    directories: UserDirectories,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Registration {
    Loaded,
    NotLoaded,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Schedule {
    Missing,
    Saved { interval_seconds: u64 },
}
pub const BINARY_MARKER: &str = concat!(
    "\0repository-watch-installation:",
    identity::service_label!(),
    "\0"
);

/// Identify the managed task independently of its current interval or executable version.
pub fn task_marker(store: &Store, binary: &Path) -> String {
    use std::os::unix::ffi::OsStrExt;
    let mut paths = Vec::new();
    for path in [
        store.config_path(),
        store.state_dir(),
        store.log_dir(),
        binary.to_owned(),
    ] {
        let bytes = path.as_os_str().as_bytes();
        paths.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
        paths.extend_from_slice(bytes);
    }
    format!("managed-by={LABEL};storage={}", digest(&paths))
}
impl Manager {
    pub fn new() -> Result<Self> {
        let user_home = std::env::var_os("HOME").context(i18n::tr!(CannotReadUserHomeDirectory))?;
        let user_home = PathBuf::from(user_home);
        ensure!(
            user_home.is_absolute(),
            i18n::tr!(UserHomeDirectoryIsNotAnAbsolute)
        );
        let mut manager = Self::for_user(user_home, unsafe { libc::getuid() })?;
        manager.directories = UserDirectories::from_environment(&manager.user_home)?;
        Ok(manager)
    }
    /// Construct an isolated installation using default directories for a user.
    pub fn for_user(user_home: PathBuf, _uid: u32) -> Result<Self> {
        ensure!(
            user_home.is_absolute(),
            i18n::tr!(UserHomeDirectoryIsNotAnAbsolute)
        );
        Ok(Self {
            directories: UserDirectories::defaults(&user_home),
            user_home,
            #[cfg(target_os = "macos")]
            uid: _uid,
        })
    }
    pub fn user_home(&self) -> &Path {
        &self.user_home
    }
    pub fn binary(&self) -> PathBuf {
        self.user_home.join(".local/bin").join(identity::COMMAND)
    }
    pub fn definition(&self) -> PathBuf {
        #[cfg(target_os = "macos")]
        {
            self.plist()
        }
        #[cfg(target_os = "linux")]
        {
            self.timer()
        }
    }
    pub fn default_store(&self) -> Store {
        self.directories.store()
    }
    pub fn verify_install_target(&self, store: &Store) -> Result<()> {
        // 发布优化会把常量比较拆成若干字节段；保留完整字节串，供安装器读取二进制自识别。
        let marker = std::hint::black_box(BINARY_MARKER.as_bytes());
        match fs::read(self.binary()) {
            Ok(data) => ensure!(
                data.windows(marker.len()).any(|bytes| bytes == marker),
                i18n::tr!(InstalledCommandOwnershipMismatch)
            ),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        let expected = task_marker(store, &self.binary());
        for path in self.definitions() {
            match fs::read_to_string(path) {
                Ok(contents) => ensure!(
                    contents.lines().any(|line| line == format!("# {expected}")
                        || line == format!("<!-- {expected} -->")),
                    i18n::tr!(ScheduleOwnedByAnotherInstallation)
                ),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
        Ok(())
    }
    pub fn verify_owner(&self, store: &Store) -> Result<()> {
        ensure!(
            self.binary().try_exists()?,
            i18n::tr!(InstalledCommandOwnershipMismatch)
        );
        self.verify_install_target(store)
    }
    fn definitions(&self) -> Vec<PathBuf> {
        #[cfg(target_os = "macos")]
        {
            vec![self.plist()]
        }
        #[cfg(target_os = "linux")]
        {
            vec![self.service(), self.timer()]
        }
    }
    fn prepare_storage(store: &Store) -> Result<()> {
        use std::os::unix::fs::PermissionsExt;
        for directory in [store.state_dir(), store.log_dir()] {
            fs::create_dir_all(&directory)?;
            fs::set_permissions(directory, fs::Permissions::from_mode(0o700))?;
        }
        Ok(())
    }
}

pub fn digest(data: &[u8]) -> String {
    format!("{:x}", Sha256::digest(data))
}
