//! Open a macOS Terminal window without interpolating paths into AppleScript.
use crate::{cancel::Cancel, process};
use anyhow::{Context, Result, ensure};
use std::{ffi::OsString, path::Path, time::Duration};

const SCRIPT: &str = "on run argv\n tell application \"Terminal\"\n do script (item 1 of argv)\n activate\n end tell\nend run";

fn directory_command(path: &Path) -> Result<String> {
    ensure!(path.is_absolute(), i18n::tr!(EnterAnAbsolutePath));
    let path = path.to_str().context(i18n::tr!(TerminalPathNotUtf8))?;
    ensure!(!path.contains('\0'), i18n::tr!(TerminalPathContainsNul));
    // 路径只作为单引号包围的 Shell 参数；单引号拆开后转义再接回。
    Ok(format!("cd -- '{}'", path.replace('\'', "'\\''")))
}

pub fn open(path: &Path, cancel: &Cancel) -> Result<()> {
    ensure!(path.is_dir(), i18n::tr!(TerminalDirectoryMissing));
    let command = directory_command(path)?;
    let args = [
        OsString::from("-e"),
        SCRIPT.into(),
        "--".into(),
        command.into(),
    ];
    process::run(
        Path::new("/usr/bin/osascript"),
        &args,
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
    fn directory_command_preserves_spaces_quotes_unicode_and_shell_characters() {
        let temp = tempfile::tempdir().unwrap();
        for name in [
            "simple",
            "space and 'quote'",
            "中文目录",
            "$(touch escaped); & newline\nfolder",
        ] {
            let directory = temp.path().join(name);
            std::fs::create_dir(&directory).unwrap();
            let command = format!("{}; /bin/pwd -P", directory_command(&directory).unwrap());
            let output = std::process::Command::new("/bin/zsh")
                .current_dir(temp.path())
                .args(["-c", &command])
                .output()
                .unwrap();
            assert!(output.status.success());
            assert_eq!(
                String::from_utf8(output.stdout).unwrap(),
                format!("{}\n", std::fs::canonicalize(&directory).unwrap().display())
            );
            assert!(!temp.path().join("escaped").exists());
        }
    }
}
