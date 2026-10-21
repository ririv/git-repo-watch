//! Linux terminal launch through the explicitly configured XDG terminal selector.
use crate::cancel::Cancel;
use anyhow::{Context, Result, ensure};
use std::{ffi::OsString, path::Path};
fn arguments(path: &Path) -> Vec<OsString> {
    let mut directory = OsString::from("--dir=");
    directory.push(path);
    vec![directory]
}
#[cfg(target_os = "linux")]
pub fn open(path: &Path, cancel: &Cancel) -> Result<()> {
    launch(Path::new("xdg-terminal-exec"), &arguments(path), cancel).map_err(|error| {
        if error
            .downcast_ref::<std::io::Error>()
            .is_some_and(|cause| cause.kind() == std::io::ErrorKind::NotFound)
        {
            error.context(i18n::tr!(LinuxTerminalLauncherRequired))
        } else {
            error
        }
    })
}
fn launch(program: &Path, args: &[OsString], cancel: &Cancel) -> Result<()> {
    use std::{
        os::unix::process::CommandExt,
        process::{Command, Stdio},
        thread,
        time::Duration,
    };
    cancel.check()?;
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    // 桌面终端有独立生命周期，不加入 Git 子进程的超时和取消进程组。
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut child = command.spawn().context(i18n::tr!(OpenRepositoryTerminal))?;
    thread::sleep(Duration::from_millis(150));
    if let Some(status) = child.try_wait()? {
        ensure!(
            status.success(),
            "{}: {status}",
            i18n::tr!(OpenRepositoryTerminal)
        );
    } else {
        thread::spawn(move || {
            let _ = child.wait();
        });
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn working_directory_is_one_literal_argument() {
        assert_eq!(
            arguments(Path::new("/example/中文 $HOME; & \" folder")),
            vec![OsString::from("--dir=/example/中文 $HOME; & \" folder")]
        );
    }
    #[test]
    fn reports_missing_launcher_and_immediate_failure() {
        assert!(launch(Path::new("/missing/terminal"), &[], &Cancel::default()).is_err());
        assert!(launch(Path::new("/usr/bin/false"), &[], &Cancel::default()).is_err());
    }
    #[test]
    fn detached_application_survives_launcher_return() {
        use std::{thread, time::Duration};
        let temp = tempfile::tempdir().unwrap();
        let marker = temp.path().join("finished");
        launch(
            Path::new("/bin/sh"),
            &[
                "-c".into(),
                "sleep 0.4; printf done > \"$1\"".into(),
                "test".into(),
                marker.as_os_str().to_owned(),
            ],
            &Cancel::default(),
        )
        .unwrap();
        for _ in 0..100 {
            if marker.exists() {
                break;
            }
            thread::sleep(Duration::from_millis(20));
        }
        assert_eq!(std::fs::read(marker).unwrap(), b"done");
    }
}
