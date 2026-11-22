//! Private, atomic persistence and cross-process coordination.
mod command_history;
mod configuration;
mod database;
mod interaction;
pub mod view;
pub use configuration::{Config, ConfigurationMigration, MouseConfig, MouseSetting};
pub use interaction::InteractionMode;
mod operation;
use crate::{cancel::Cancel, platform::files};
use anyhow::{Context, Result, ensure};
pub use database::Transfer;
pub use operation::{Completion, Operation, OperationOrigin, OperationRecord, StoredOperation};
#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;
use std::{
    fs::{self, File, OpenOptions, TryLockError},
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub enum Store {
    Standard {
        configuration: PathBuf,
        state: PathBuf,
        logs: PathBuf,
    },
    Custom {
        home: PathBuf,
    },
}
impl Store {
    pub fn custom(home: PathBuf) -> Self {
        Self::Custom { home }
    }
    pub fn config_dir(&self) -> PathBuf {
        match self {
            Self::Standard { configuration, .. } => configuration.clone(),
            Self::Custom { home } => home.clone(),
        }
    }
    pub fn config_path(&self) -> PathBuf {
        self.config_dir().join("config.toml")
    }
    pub fn state_dir(&self) -> PathBuf {
        match self {
            Self::Standard { state, .. } => state.clone(),
            Self::Custom { home } => home.join("var"),
        }
    }
    pub fn log_dir(&self) -> PathBuf {
        match self {
            Self::Standard { logs, .. } => logs.clone(),
            Self::Custom { .. } => self.state_dir(),
        }
    }
    pub fn initialize(&self) -> Result<()> {
        let _lock = self.lock()?;
        ensure!(
            !self.legacy_config_path().try_exists()?,
            i18n::tr!(ConfigurationMigrationRequired)
        );
        let path = self.config_path();
        let directory = self.config_dir();
        fs::create_dir_all(&directory)?;
        if matches!(self, Self::Standard { .. }) {
            files::private_path(&directory, 0o700)?;
        }
        let mut temporary = tempfile::NamedTempFile::new_in(&directory)?;
        files::private_path(temporary.path(), 0o600)?;
        let data = configuration::encode(&Config::default(), None)?;
        temporary.write_all(data.as_bytes())?;
        temporary.as_file().sync_all()?;
        files::publish(temporary, &path, false)?;
        files::sync_directory(&directory)?;
        Ok(())
    }
    pub fn lock(&self) -> Result<Lock> {
        self.prepare_state()?;
        Lock::acquire(&self.state_dir().join("check.lock"))
    }
    /// Acquire the short-lived configuration transaction lock.
    pub fn transaction(&self, cancel: &Cancel) -> Result<Lock> {
        self.prepare_state()?;
        Lock::wait(&self.state_dir().join("check.lock"), cancel)
    }
    pub(crate) fn prepare_state(&self) -> Result<()> {
        fs::create_dir_all(self.state_dir())?;
        files::private_path(&self.state_dir(), 0o700)?;
        Ok(())
    }
    /// Serialize complete batches without locking unrelated repository operations.
    pub fn batch_lock(&self) -> Result<Lock> {
        self.prepare_state()?;
        Lock::acquire(&self.state_dir().join("batch.lock"))
    }
    /// Worktrees sharing Git storage deliberately share the same operation lock.
    pub fn repository_lock(&self, common_dir: &Path, cancel: &Cancel) -> Result<Lock> {
        self.prepare_state()?;
        let directory = self.state_dir().join("repository-locks");
        fs::create_dir_all(&directory)?;
        files::private_path(&directory, 0o700)?;
        let key = crate::scheduler::digest(common_dir.as_os_str().as_encoded_bytes());
        Lock::wait(&directory.join(format!("{key}.lock")), cancel)
    }
    /// Bound active Git operations across batches, manual tasks, and processes.
    pub fn operation_slot(&self, limit: usize, cancel: &Cancel) -> Result<Lock> {
        ensure!(
            (1..=32).contains(&limit),
            i18n::tr!(ConcurrencyMustBeBetween1And32)
        );
        self.prepare_state()?;
        let directory = self.state_dir().join("operation-slots");
        fs::create_dir_all(&directory)?;
        files::private_path(&directory, 0o700)?;
        loop {
            cancel.check()?;
            for slot in 0..limit {
                match Lock::acquire(&directory.join(format!("{slot}.lock"))) {
                    Ok(lock) => return Ok(lock),
                    Err(error) if error.downcast_ref::<Busy>().is_some() => {}
                    Err(error) => return Err(error),
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
    pub fn running(&self) -> Result<bool> {
        let p = self.state_dir().join("batch.lock");
        if !p.try_exists()? {
            return Ok(false);
        }
        match Lock::acquire(&p) {
            Ok(_) => Ok(false),
            Err(e) if e.downcast_ref::<Busy>().is_some() => Ok(true),
            Err(e) => Err(e),
        }
    }
}
#[derive(Debug)]
pub struct Busy;
impl std::fmt::Display for Busy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(i18n::tr!(ACheckOrRepositoryOperationIsAlready))
    }
}
impl std::error::Error for Busy {}
#[derive(Debug)]
pub struct Lock {
    _file: File,
}
impl Lock {
    pub fn wait(path: &Path, cancel: &Cancel) -> Result<Self> {
        loop {
            cancel.check()?;
            match Self::acquire(path) {
                Err(error) if error.downcast_ref::<Busy>().is_some() => {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                result => return result,
            }
        }
    }
    pub fn acquire(path: &Path) -> Result<Self> {
        let mut options = OpenOptions::new();
        options.create(true).truncate(false).read(true).write(true);
        #[cfg(unix)]
        options.mode(0o600);
        let file = options.open(path)?;
        #[cfg(windows)]
        files::private_path(path, 0o600)?;
        match file.try_lock() {
            Ok(()) => Ok(Self { _file: file }),
            Err(TryLockError::WouldBlock) => Err(Busy.into()),
            Err(TryLockError::Error(error)) => Err(error.into()),
        }
    }
}

pub fn atomic(path: &Path, data: &[u8], mode: u32) -> Result<()> {
    let parent = path
        .parent()
        .context(i18n::tr!(SavePathHasNoParentDirectory))?;
    let mut tmp = tempfile::Builder::new()
        .prefix(".atomic-")
        .tempfile_in(parent)?;
    files::private_path(tmp.path(), mode)?;
    tmp.write_all(data)?;
    tmp.as_file().sync_all()?;
    files::publish(tmp, path, true)?;
    files::sync_directory(parent)?;
    Ok(())
}
pub fn canonical(path: &Path) -> Result<PathBuf> {
    if path.try_exists()? {
        Ok(fs::canonicalize(path)?)
    } else {
        ensure!(path.is_absolute(), i18n::tr!(EnterAnAbsolutePath));
        ensure!(
            path.components().all(|c| !matches!(
                c,
                std::path::Component::ParentDir | std::path::Component::CurDir
            )),
            i18n::tr!(EnterANormalizedPath)
        );
        Ok(path.to_owned())
    }
}
pub fn interval(value: &str) -> Result<u64> {
    crate::scheduler::duration::parse(value)
}
pub fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}
