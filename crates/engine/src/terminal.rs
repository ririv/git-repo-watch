//! Open repository directories with the user's system-selected terminal application.
mod macos;
use crate::{cancel::Cancel, process};
use anyhow::{Result, ensure};
use std::{ffi::OsString, path::Path, time::Duration};

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
    let application = macos::default_application()?;
    process::run(
        Path::new("/usr/bin/open"),
        &arguments(&application, path),
        None,
        cancel,
        Duration::from_secs(30),
    )?
    .success(i18n::tr!(OpenRepositoryTerminal))?;
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
