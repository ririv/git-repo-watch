//! Git-compatible index locking and atomic installation.
use anyhow::{Context, Result};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

enum LockState {
    Held(PathBuf),
    Installed,
}
pub(crate) struct IndexLock {
    state: LockState,
    destination: PathBuf,
    file: File,
}
impl IndexLock {
    pub(crate) fn new(index: &Path) -> Result<Self> {
        let mut path = index.as_os_str().to_os_string();
        path.push(".lock");
        let path = PathBuf::from(path);
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .with_context(|| format!("{}: {}", i18n::tr!(ComposeIndexLocked), path.display()))?;
        let lock = Self {
            state: LockState::Held(path),
            destination: index.to_owned(),
            file,
        };
        match fs::metadata(index) {
            Ok(metadata) => lock.file.set_permissions(metadata.permissions())?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        Ok(lock)
    }
    pub(crate) fn prepare(&mut self, bytes: &[u8]) -> Result<()> {
        use std::io::Seek;
        self.file.set_len(0)?;
        self.file.rewind()?;
        self.file.write_all(bytes)?;
        self.file.sync_all()?;
        Ok(())
    }
    pub(crate) fn install(mut self) -> Result<()> {
        let LockState::Held(path) = &self.state else {
            unreachable!("index lock is installed only once");
        };
        fs::rename(path, &self.destination)?;
        // 替换后立即解除清理责任，避免误删随后由其它进程创建的新锁。
        self.state = LockState::Installed;
        File::open(
            self.destination
                .parent()
                .context(i18n::tr!(ComposeInvalid))?,
        )?
        .sync_all()?;
        Ok(())
    }
}
impl Drop for IndexLock {
    fn drop(&mut self) {
        if let LockState::Held(path) = &self.state {
            let _ = fs::remove_file(path);
        }
    }
}
