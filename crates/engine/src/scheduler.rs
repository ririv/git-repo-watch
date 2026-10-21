//! Ownership-checked native installation and platform scheduling.
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::plist;
#[cfg(any(target_os = "linux", test))]
mod linux;
use crate::{cancel::Cancel, process, state};
use anyhow::{Context, Result, ensure};
pub use identity::SERVICE_LABEL as LABEL;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};
#[derive(Debug, Clone)]
pub struct Manager {
    user_home: PathBuf,
    #[cfg(target_os = "macos")]
    uid: u32,
    #[cfg(target_os = "linux")]
    directories: linux::Directories,
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
#[derive(Debug, Serialize, Deserialize)]
pub struct Location {
    pub home: PathBuf,
    pub binary_sha256: String,
}
impl Manager {
    pub fn new() -> Result<Self> {
        let user_home = std::env::var_os("HOME").context(i18n::tr!(CannotReadUserHomeDirectory))?;
        let user_home = PathBuf::from(user_home);
        ensure!(
            user_home.is_absolute(),
            i18n::tr!(UserHomeDirectoryIsNotAnAbsolute)
        );
        let manager = Self::for_user(user_home, unsafe { libc::getuid() })?;
        #[cfg(target_os = "linux")]
        let manager = Self {
            directories: linux::Directories::from_environment(&manager.user_home)?,
            ..manager
        };
        Ok(manager)
    }
    /// Construct an isolated installation using default directories for a user.
    pub fn for_user(user_home: PathBuf, _uid: u32) -> Result<Self> {
        ensure!(
            user_home.is_absolute(),
            i18n::tr!(UserHomeDirectoryIsNotAnAbsolute)
        );
        Ok(Self {
            #[cfg(target_os = "linux")]
            directories: linux::Directories::defaults(&user_home),
            user_home,
            #[cfg(target_os = "macos")]
            uid: _uid,
        })
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
    pub fn read_location(&self) -> Result<Location> {
        let l: Location = state::read_json(&self.location())?;
        ensure!(
            l.home.is_absolute(),
            i18n::tr!(InstallationLocationIsNotAnAbsolutePath)
        );
        Ok(l)
    }
}

pub fn digest(data: &[u8]) -> String {
    format!("{:x}", Sha256::digest(data))
}
