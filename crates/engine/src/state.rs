//! Private, atomic persistence and cross-process coordination.
use crate::{
    cancel::Cancel,
    domain::{PushPolicy, Report},
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    os::{
        fd::AsRawFd,
        unix::fs::{OpenOptionsExt, PermissionsExt},
    },
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub interval_seconds: u64,
    pub repositories: Vec<PathBuf>,
    pub excluded_roots: Vec<PathBuf>,
    pub push_policy: PushPolicy,
    #[serde(
        default = "default_concurrency",
        skip_serializing_if = "is_default_concurrency"
    )]
    pub max_concurrency: usize,
    #[serde(default = "default_startup_check")]
    pub check_on_startup: bool,
}
fn default_startup_check() -> bool {
    true
}
fn default_concurrency() -> usize {
    4
}
fn is_default_concurrency(value: &usize) -> bool {
    *value == 4
}
impl Config {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.interval_seconds >= 60,
            i18n::tr!(CheckIntervalMustBeAtLeast60)
        );
        ensure!(
            (1..=32).contains(&self.max_concurrency),
            i18n::tr!(ConcurrencyMustBeBetween1And32)
        );
        for paths in [&self.repositories, &self.excluded_roots] {
            let mut seen = std::collections::HashSet::new();
            for p in paths {
                ensure!(
                    p.is_absolute()
                        && p.components().all(|c| !matches!(
                            c,
                            std::path::Component::ParentDir | std::path::Component::CurDir
                        )),
                    i18n::tf!(PathMustBeANormalizedAbsolutePath, p.display())
                );
                ensure!(seen.insert(p), i18n::tf!(DuplicatePath, p.display()));
            }
        }
        for p in &self.repositories {
            ensure!(
                !self.excluded_roots.iter().any(|x| p.starts_with(x)),
                i18n::tf!(RepositoryIsUnderAnExcludedDirectory, p.display())
            )
        }
        Ok(())
    }
}
#[derive(Debug, Clone)]
pub struct Store {
    pub home: PathBuf,
}
impl Store {
    pub fn state_dir(&self) -> PathBuf {
        self.home.join("var")
    }
    pub fn config(&self) -> Result<Config> {
        let c: Config = read_json(&self.home.join("config.json"))?;
        c.validate()?;
        Ok(c)
    }
    pub fn save_config(&self, c: &Config) -> Result<()> {
        c.validate()?;
        write_json(&self.home.join("config.json"), c)
    }
    pub fn report(&self) -> Result<Option<Report>> {
        let p = self.state_dir().join("latest.json");
        match fs::read(&p) {
            Ok(data) => {
                let r: Report =
                    serde_json::from_slice(&data).context(i18n::tr!(InvalidReportFormat))?;
                r.validate()?;
                Ok(Some(r))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
    /// Read independent single-repository reports without replacing a complete batch report.
    pub fn repository_reports(&self) -> Result<std::collections::BTreeMap<PathBuf, Report>> {
        let path = self.state_dir().join("repository-checks.json");
        let data = match fs::read(path) {
            Ok(data) => data,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(std::collections::BTreeMap::new());
            }
            Err(error) => return Err(error.into()),
        };
        let reports: std::collections::BTreeMap<PathBuf, Report> =
            serde_json::from_slice(&data).context(i18n::tr!(InvalidRepositoryCheckReport))?;
        for (path, report) in &reports {
            report.validate()?;
            ensure!(
                path.is_absolute()
                    && report.repositories.len() == 1
                    && report.repositories[0].path() == path,
                i18n::tr!(InvalidRepositoryCheckReport)
            );
            chrono::DateTime::parse_from_rfc3339(&report.finished_at)
                .context(i18n::tr!(InvalidRepositoryCheckReport))?;
        }
        Ok(reports)
    }
    pub fn save_report(&self, r: &Report) -> Result<()> {
        write_json(&self.state_dir().join("latest.json"), r)
    }
    pub fn lock(&self) -> Result<Lock> {
        self.prepare_state()?;
        Lock::acquire(&self.state_dir().join("check.lock"))
    }
    /// Acquire the short-lived configuration/report transaction lock.
    pub fn transaction(&self, cancel: &Cancel) -> Result<Lock> {
        self.prepare_state()?;
        Lock::wait(&self.state_dir().join("check.lock"), cancel)
    }
    fn prepare_state(&self) -> Result<()> {
        fs::create_dir_all(self.state_dir())?;
        fs::set_permissions(self.state_dir(), fs::Permissions::from_mode(0o700))?;
        Ok(())
    }
    /// Serialize complete batches without locking unrelated repository operations.
    pub fn batch_lock(&self) -> Result<Lock> {
        self.prepare_state()?;
        Lock::acquire(&self.state_dir().join("batch.lock"))
    }
    /// Worktrees sharing Git storage deliberately share the same operation lock.
    pub fn repository_lock(&self, common_dir: &Path, cancel: &Cancel) -> Result<Lock> {
        use std::os::unix::ffi::OsStrExt;
        self.prepare_state()?;
        let directory = self.state_dir().join("repository-locks");
        fs::create_dir_all(&directory)?;
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))?;
        let key = crate::scheduler::digest(common_dir.as_os_str().as_bytes());
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
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))?;
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
    /// Merge while holding only the persistence lock; never hold it during Git work.
    pub fn save_repository_report(&self, report: &mut Report, cancel: &Cancel) -> Result<()> {
        report.validate()?;
        ensure!(
            report.repositories.len() == 1,
            i18n::tr!(InvalidRepositoryCheckReport)
        );
        let _transaction = self.transaction(cancel)?;
        let mut reports = self.repository_reports()?;
        let path = report.repositories[0].path().to_owned();
        let previous = reports
            .get(&path)
            .and_then(|old| old.repository_revisions.get(&path))
            .copied()
            .unwrap_or(0);
        let revision = previous
            .checked_add(1)
            .context(i18n::tr!(InvalidRepositoryCheckReport))?;
        report.repository_revisions.insert(path.clone(), revision);
        reports.insert(path, report.clone());
        write_json(&self.state_dir().join("repository-checks.json"), &reports)
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
pub struct Lock(File);
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
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .mode(0o600)
            .open(path)?;
        // flock 由文件描述符持有，退出或展开错误时由 RAII 释放。
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            let e = std::io::Error::last_os_error();
            if e.raw_os_error() == Some(libc::EWOULDBLOCK) {
                return Err(Busy.into());
            }
            return Err(e.into());
        }
        Ok(Self(file))
    }
}
impl Drop for Lock {
    fn drop(&mut self) {
        unsafe {
            libc::flock(self.0.as_raw_fd(), libc::LOCK_UN);
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
    tmp.as_file()
        .set_permissions(fs::Permissions::from_mode(mode))?;
    tmp.write_all(data)?;
    tmp.as_file().sync_all()?;
    tmp.persist(path).map_err(|e| e.error)?;
    File::open(parent)?.sync_all()?;
    Ok(())
}
pub fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let mut data = serde_json::to_vec_pretty(value)?;
    data.push(b'\n');
    atomic(path, &data, 0o600)
}
pub fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    serde_json::from_slice(&fs::read(path).with_context(|| i18n::tf!(CannotRead, path.display()))?)
        .context(i18n::tr!(InvalidJson))
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
    let (n, mult) = if let Some(v) = value.strip_suffix('d') {
        (v, 86400.0)
    } else if let Some(v) = value.strip_suffix('h') {
        (v, 3600.0)
    } else if let Some(v) = value.strip_suffix('m') {
        (v, 60.0)
    } else if let Some(v) = value.strip_suffix('s') {
        (v, 1.0)
    } else {
        anyhow::bail!(i18n::tr!(IntervalRequiresAUnitSMH))
    };
    let seconds = n.parse::<f64>()? * mult;
    ensure!(
        seconds.is_finite()
            && seconds >= 60.0
            && seconds.fract() == 0.0
            && seconds <= i32::MAX as f64,
        i18n::tr!(CheckIntervalMustBeAnIntegerOf)
    );
    Ok(seconds as u64)
}
pub fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}
