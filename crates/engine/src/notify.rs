//! macOS notifications with data passed as arguments, never script interpolation.
use crate::{
    cancel::Cancel,
    domain::{Notice, PushTarget},
    process,
};
use std::{ffi::OsString, path::Path, time::Duration};
const SCRIPT: &str = "on run argv\n display notification (item 1 of argv) with title (item 3 of argv) subtitle (item 2 of argv) sound name \"Basso\"\nend run";
fn arguments(path: &Path, target: &PushTarget, error: &str) -> Vec<OsString> {
    let message = i18n::tf!(
        PushReportDetails,
        error.chars().take(240).collect::<String>()
    );
    let subtitle = format!(
        "{} / {}",
        path.file_name()
            .unwrap_or(path.as_os_str())
            .to_string_lossy(),
        target.branch
    );
    vec![
        OsString::from("-e"),
        SCRIPT.into(),
        "--".into(),
        message.into(),
        subtitle.into(),
        i18n::tr!(AutoPushFailureTitle).into(),
    ]
}

pub fn failure(path: &Path, target: &PushTarget, error: &str, cancel: &Cancel) -> Notice {
    let args = arguments(path, target, error);
    let attempt = process::run(
        Path::new("/usr/bin/osascript"),
        &args,
        None,
        cancel,
        Duration::from_secs(15),
    )
    .and_then(|o| o.success(i18n::tr!(SystemNotification)));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notification_localizes_copy_and_keeps_payload_outside_script() {
        let target = PushTarget {
            branch: "feature".into(),
            local_ref: "refs/heads/feature".into(),
            local_oid: "a".repeat(40),
            remote: "origin".into(),
            remote_ref: "refs/heads/feature".into(),
            remote_oid: "b".repeat(40),
            ahead_commits: 1,
        };
        for (language, title, details) in [
            (
                i18n::Language::English,
                concat!(identity::display_name!(), " automatic push failed"),
                "Details:",
            ),
            (
                i18n::Language::Chinese,
                concat!(identity::display_name!(), " 自动推送失败"),
                "详情：",
            ),
        ] {
            let _language = i18n::using(language);
            let args = arguments(Path::new("/example/a \" repo"), &target, "network error");
            assert_eq!(args[5], title);
            assert!(args[3].to_str().unwrap().contains(details));
            assert_eq!(args[4], "a \" repo / feature");
            assert!(!SCRIPT.contains("network error"));
        }
        let temp = tempfile::tempdir().unwrap();
        let status = std::process::Command::new("/usr/bin/osacompile")
            .args(["-e", SCRIPT, "-o"])
            .arg(temp.path().join("notification.scpt"))
            .output()
            .unwrap();
        assert!(
            status.status.success(),
            "{}",
            String::from_utf8_lossy(&status.stderr)
        );
    }
}
