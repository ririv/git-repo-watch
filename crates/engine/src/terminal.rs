//! Open repository directories with the user's system-selected terminal application.
#[cfg(any(target_os = "linux", test))]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(windows)]
mod windows;
use crate::cancel::Cancel;
#[cfg(target_os = "macos")]
use crate::process;
use anyhow::{Result, ensure};
#[cfg(any(target_os = "macos", test))]
use std::ffi::OsString;
use std::path::Path;
#[cfg(target_os = "macos")]
use std::time::Duration;

#[cfg(any(target_os = "macos", test))]
fn arguments(application: &str, directory: &Path) -> Vec<OsString> {
    vec![
        "-b".into(),
        application.into(),
        directory.as_os_str().to_owned(),
    ]
}

pub fn open(path: &Path, cancel: &Cancel) -> Result<()> {
    cancel.check()?;
    ensure!(path.is_absolute(), i18n::tr!(EnterAnAbsolutePath));
    ensure!(path.is_dir(), i18n::tr!(TerminalDirectoryMissing));
    #[cfg(target_os = "linux")]
    {
        linux::open(path, cancel)?;
    }
    #[cfg(target_os = "macos")]
    {
        let application = macos::default_application()?;
        process::run(
            Path::new("/usr/bin/open"),
            &arguments(&application, path),
            None,
            cancel,
            Duration::from_secs(30),
        )?
        .success(i18n::tr!(OpenRepositoryTerminal))?;
    }
    #[cfg(windows)]
    windows::open(path, cancel)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folder_paths_are_literal_arguments_without_shell_commands() {
        for path in [
            "/example/simple",
            "/example/space and 'quotes'",
            "/example/中文目录",
            "/example/$(touch escaped); & newline\nfolder",
        ] {
            let args = arguments("com.mitchellh.ghostty", Path::new(path));
            assert_eq!(
                args,
                vec![
                    OsString::from("-b"),
                    OsString::from("com.mitchellh.ghostty"),
                    OsString::from(path)
                ]
            );
        }
    }
}
