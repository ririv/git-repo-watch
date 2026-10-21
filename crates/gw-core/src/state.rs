//! Private, atomic persistence and cross-process coordination.
use crate::domain::{PushPolicy, Report};
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
}
fn default_concurrency() -> usize {
    4
}
fn is_default_concurrency(value: &usize) -> bool {
    *value == 4
}
impl Config {
    pub fn validate(&self) -> Result<()> {
        ensure!(self.interval_seconds >= 60, "检查间隔至少为 60 秒");
        ensure!(
            (1..=32).contains(&self.max_concurrency),
            "并发数必须在 1 至 32 之间"
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
                    "路径必须为规范绝对路径：{}",
                    p.display()
                );
                ensure!(seen.insert(p), "重复路径：{}", p.display());
            }
        }
        for p in &self.repositories {
            ensure!(
                !self.excluded_roots.iter().any(|x| p.starts_with(x)),
                "仓库处于已排除目录：{}",
                p.display()
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
                let r: Report = serde_json::from_slice(&data).context("报告格式无效")?;
                r.validate()?;
                Ok(Some(r))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
    pub fn save_report(&self, r: &Report) -> Result<()> {
        write_json(&self.state_dir().join("latest.json"), r)
    }
    pub fn lock(&self) -> Result<Lock> {
        fs::create_dir_all(self.state_dir())?;
        fs::set_permissions(self.state_dir(), fs::Permissions::from_mode(0o700))?;
        Lock::acquire(&self.state_dir().join("check.lock"))
    }
    pub fn running(&self) -> Result<bool> {
        let p = self.state_dir().join("check.lock");
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
        f.write_str("已有检查或仓库操作正在运行")
    }
}
impl std::error::Error for Busy {}
#[derive(Debug)]
pub struct Lock(File);
impl Lock {
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
    let parent = path.parent().context("保存路径缺少父目录")?;
    let mut tmp = tempfile::Builder::new()
        .prefix(".gw-")
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
    serde_json::from_slice(&fs::read(path).with_context(|| format!("无法读取 {}", path.display()))?)
        .context("JSON 格式无效")
}
pub fn canonical(path: &Path) -> Result<PathBuf> {
    if path.try_exists()? {
        Ok(fs::canonicalize(path)?)
    } else {
        ensure!(path.is_absolute(), "请输入绝对路径");
        ensure!(
            path.components().all(|c| !matches!(
                c,
                std::path::Component::ParentDir | std::path::Component::CurDir
            )),
            "请输入规范路径"
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
        anyhow::bail!("间隔必须带 s、m、h 或 d 单位")
    };
    let seconds = n.parse::<f64>()? * mult;
    ensure!(
        seconds.is_finite()
            && seconds >= 60.0
            && seconds.fract() == 0.0
            && seconds <= i32::MAX as f64,
        "检查间隔必须为至少 60 秒的整数"
    );
    Ok(seconds as u64)
}
pub fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}
