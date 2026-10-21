//! macOS notifications with data passed as arguments, never script interpolation.
use crate::{
    cancel::Cancel,
    domain::{Notice, PushTarget},
    process,
};
use std::{ffi::OsString, path::Path, time::Duration};
const SCRIPT: &str = "on run argv\n display notification (item 1 of argv) with title \"GW 自动推送失败\" subtitle (item 2 of argv) sound name \"Basso\"\nend run";
pub fn failure(path: &Path, target: &PushTarget, error: &str, cancel: &Cancel) -> Notice {
    let message = format!(
        "{}\n详情：gw report --pushes",
        error.chars().take(240).collect::<String>()
    );
    let subtitle = format!(
        "{} / {}",
        path.file_name()
            .unwrap_or(path.as_os_str())
            .to_string_lossy(),
        target.branch
    );
    let args = vec![
        OsString::from("-e"),
        SCRIPT.into(),
        "--".into(),
        message.into(),
        subtitle.into(),
    ];
    let attempt = process::run(
        Path::new("/usr/bin/osascript"),
        &args,
        None,
        cancel,
        Duration::from_secs(15),
    )
    .and_then(|o| o.success("系统通知"));
    match attempt {
        Ok(_) => Notice::Accepted {
            path: path.to_owned(),
            branch: target.branch.clone(),
        },
        Err(e) => Notice::Failed {
            path: path.to_owned(),
            branch: target.branch.clone(),
            error: format!("{e:#}"),
        },
    }
}
