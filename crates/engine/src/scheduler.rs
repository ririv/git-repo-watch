//! Ownership-checked native installation and optional platform scheduling.
#[cfg(feature = "developer-scheduler")]
use crate::platform::files;
#[cfg(all(feature = "developer-scheduler", target_os = "macos"))]
mod macos;
#[cfg(all(feature = "developer-scheduler", target_os = "macos"))]
pub use macos::plist;
#[cfg(feature = "developer-scheduler")]
mod calendar;
pub mod duration;
#[cfg(all(feature = "developer-scheduler", any(target_os = "linux", test)))]
mod linux;
pub mod timing;
#[cfg(all(test, feature = "developer-scheduler"))]
mod timing_tests;
#[cfg(all(windows, feature = "developer-scheduler"))]
mod windows;
#[cfg(all(unix, feature = "developer-scheduler"))]
use crate::process;
use crate::{
    cancel::Cancel,
    directories::UserDirectories,
    state::{self, Store},
};
use anyhow::{Context, Result, ensure};
#[cfg(feature = "developer-scheduler")]
pub use identity::SERVICE_LABEL as LABEL;
#[cfg(feature = "developer-scheduler")]
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};
#[cfg(feature = "developer-scheduler")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    Enable,
    Disable,
    Apply,
}
#[derive(Debug, Clone)]
pub struct Manager {
    user_home: PathBuf,
    #[cfg(all(feature = "developer-scheduler", target_os = "macos"))]
    uid: u32,
    directories: UserDirectories,
}
#[cfg(feature = "developer-scheduler")]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Registration {
    Loaded,
    NotLoaded,
}
#[cfg(feature = "developer-scheduler")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Schedule {
    Missing,
    Saved { interval_seconds: u64 },
    Anchored { plan: timing::Plan },
}
#[cfg(feature = "developer-scheduler")]
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum NextTrigger {
    Disabled,
    Missing,
    Unanchored,
    At { at: chrono::DateTime<chrono::Local> },
}
#[cfg(feature = "developer-scheduler")]
impl NextTrigger {
    pub fn display(&self) -> String {
        match self {
            Self::Disabled => i18n::tr!(ScheduleNextDisabled).into(),
            Self::Missing => i18n::tr!(ScheduleNextMissing).into(),
            Self::Unanchored => i18n::tr!(ScheduleNextUnanchored).into(),
            Self::At { at } => at.format("%Y-%m-%d %H:%M:%S").to_string(),
        }
    }
}
#[cfg(feature = "developer-scheduler")]
impl Schedule {
    pub fn next_trigger(
        &self,
        registration: Registration,
        now: chrono::DateTime<chrono::Local>,
    ) -> Result<NextTrigger> {
        if matches!(self, Self::Missing) {
            return Ok(NextTrigger::Missing);
        }
        if registration == Registration::NotLoaded {
            return Ok(NextTrigger::Disabled);
        }
        match self {
            Self::Missing => Ok(NextTrigger::Missing),
            Self::Saved { .. } => Ok(NextTrigger::Unanchored),
            Self::Anchored { plan } => Ok(NextTrigger::At {
                at: plan.next_after(now)?,
            }),
        }
    }
}

#[cfg(all(unix, feature = "developer-scheduler"))]
fn plan_comment(plan: &timing::Plan) -> Result<String> {
    Ok(format!("schedule-plan={}", serde_json::to_string(plan)?))
}
#[cfg(all(unix, feature = "developer-scheduler"))]
fn read_plan(contents: &str, prefix: &str, suffix: &str) -> Result<Option<timing::Plan>> {
    let entries = contents
        .lines()
        .filter_map(|line| {
            line.strip_prefix(prefix)
                .and_then(|line| line.strip_suffix(suffix))
        })
        .collect::<Vec<_>>();
    ensure!(entries.len() <= 1, i18n::tr!(InvalidScheduledInterval));
    entries
        .first()
        .map(|json| serde_json::from_str(json).map_err(Into::into))
        .transpose()
}
pub const BINARY_MARKER: &str = concat!(
    "\0repository-watch-installation:",
    identity::service_label!(),
    "\0"
);

#[cfg(feature = "developer-scheduler")]
/// Identify the managed task independently of its current interval or executable version.
pub fn task_marker(store: &Store, binary: &Path) -> String {
    let mut paths = Vec::new();
    for path in [
        store.config_dir(),
        store.state_dir(),
        store.log_dir(),
        binary.to_owned(),
    ] {
        let bytes = path.as_os_str().as_encoded_bytes();
        paths.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
        paths.extend_from_slice(bytes);
    }
    format!("managed-by={LABEL};storage={}", digest(&paths))
}
impl Manager {
    pub fn new() -> Result<Self> {
        let user_home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
            .context(i18n::tr!(CannotReadUserHomeDirectory))?;
        let user_home = PathBuf::from(user_home);
        ensure!(
            user_home.is_absolute(),
            i18n::tr!(UserHomeDirectoryIsNotAnAbsolute)
        );
        #[cfg(all(feature = "developer-scheduler", target_os = "macos"))]
        let uid = unsafe { libc::getuid() };
        #[cfg(not(all(feature = "developer-scheduler", target_os = "macos")))]
        let uid = 0;
        let mut manager = Self::for_user(user_home, uid)?;
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
            #[cfg(all(feature = "developer-scheduler", target_os = "macos"))]
            uid: _uid,
        })
    }
    pub fn user_home(&self) -> &Path {
        &self.user_home
    }
    pub fn binary(&self) -> PathBuf {
        #[cfg(unix)]
        return self.user_home.join(".local/bin").join(identity::COMMAND);
        #[cfg(windows)]
        return self
            .directories
            .state_base
            .join(identity::DATA_DIRECTORY)
            .join("bin")
            .join(format!("{}.exe", identity::COMMAND));
    }
    #[cfg(all(unix, feature = "developer-scheduler"))]
    pub fn definition(&self) -> PathBuf {
        #[cfg(all(feature = "developer-scheduler", target_os = "macos"))]
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
    fn verify_install_binary(&self) -> Result<()> {
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
        Ok(())
    }
    #[cfg(not(feature = "developer-scheduler"))]
    pub fn install_command(&self, source: &Path, cancel: &Cancel) -> Result<()> {
        cancel.check()?;
        self.verify_install_binary()?;
        let data = fs::read(source)?;
        let path = self.binary();
        let parent = path
            .parent()
            .context(i18n::tr!(InstallationPathHasNoParentDirectory))?;
        fs::create_dir_all(parent)?;
        state::atomic(&path, &data, 0o755)
    }
    #[cfg(feature = "developer-scheduler")]
    pub fn verify_install_target(&self, _store: &Store) -> Result<()> {
        self.verify_install_binary()?;
        #[cfg(unix)]
        self.verify_task_owner(_store)?;
        Ok(())
    }

    #[cfg(all(unix, feature = "developer-scheduler"))]
    pub(crate) fn schedule_state(
        &self,
        _store: &Store,
        cancel: &Cancel,
    ) -> Result<(Registration, Schedule)> {
        Ok((self.registration(cancel)?, self.schedule(cancel)?))
    }

    #[cfg(all(unix, feature = "developer-scheduler"))]
    pub fn verify_task_owner(&self, store: &Store) -> Result<()> {
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
    #[cfg(feature = "developer-scheduler")]
    pub fn verify_owner(&self, store: &Store) -> Result<()> {
        ensure!(
            self.binary().try_exists()?,
            i18n::tr!(InstalledCommandOwnershipMismatch)
        );
        self.verify_install_target(store)
    }
    #[cfg(all(unix, feature = "developer-scheduler"))]
    fn definitions(&self) -> Vec<PathBuf> {
        #[cfg(all(feature = "developer-scheduler", target_os = "macos"))]
        {
            vec![self.plist()]
        }
        #[cfg(target_os = "linux")]
        {
            vec![self.service(), self.timer()]
        }
    }
    #[cfg(feature = "developer-scheduler")]
    fn prepare_storage(store: &Store) -> Result<()> {
        for directory in [store.state_dir(), store.log_dir()] {
            fs::create_dir_all(&directory)?;
            files::private_path(&directory, 0o700)?;
        }
        Ok(())
    }
}

pub fn digest(data: &[u8]) -> String {
    format!("{:x}", Sha256::digest(data))
}
