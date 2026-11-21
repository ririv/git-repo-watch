//! Launch Windows Terminal in the selected directory without a shell.
use crate::{cancel::Cancel, desktop, platform::windows::shell_path};
use anyhow::{Context, Result};
use std::{
    ffi::OsString,
    path::Path,
    thread,
    time::{Duration, Instant},
};

pub(super) fn open(path: &Path, cancel: &Cancel) -> Result<()> {
    cancel.check()?;
    let program = std::env::var_os("LOCALAPPDATA")
        .map(std::path::PathBuf::from)
        .filter(|path| path.is_absolute())
        .context(i18n::tr!(CannotReadUserHomeDirectory))?
        .join("Microsoft/WindowsApps/wt.exe");
    let args = [OsString::from("-d"), shell_path(path)];
    let mut launch =
        desktop::launch_command(&program, &args, Some(path), path.display().to_string())?;
    let start = Instant::now();
    while start.elapsed() < Duration::from_millis(200) {
        if launch.poll()?.is_some() {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(10));
    }
    Ok(())
}
