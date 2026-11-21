//! Desktop notifications with payloads passed as literal arguments.
#[cfg(target_os = "macos")]
#[path = "notify/macos.rs"]
mod platform;
#[cfg(target_os = "linux")]
#[path = "notify/linux.rs"]
mod platform;
#[cfg(windows)]
#[path = "notify/windows.rs"]
mod platform;
#[cfg(unix)]
use crate::process;
use crate::{
    cancel::Cancel,
    domain::{Notice, PushTarget},
};
use std::path::Path;
#[cfg(unix)]
use std::time::Duration;
struct Payload {
    title: String,
    subtitle: String,
    message: String,
}
impl Payload {
    fn new(path: &Path, target: &PushTarget, error: &str) -> Self {
        Self {
            title: i18n::tr!(AutoPushFailureTitle).to_owned(),
            subtitle: format!(
                "{} / {}",
                path.file_name()
                    .unwrap_or(path.as_os_str())
                    .to_string_lossy(),
                target.branch
            ),
            message: i18n::tf!(
                PushReportDetails,
                error.chars().take(240).collect::<String>()
            ),
        }
    }
}
pub fn failure(path: &Path, target: &PushTarget, error: &str, cancel: &Cancel) -> Notice {
    let payload = Payload::new(path, target, error);
    #[cfg(unix)]
    let args = platform::arguments(&payload);
    #[cfg(unix)]
    let attempt = process::run(
        platform::program(),
        &args,
        None,
        cancel,
        Duration::from_secs(15),
    )
    .and_then(|o| o.success(i18n::tr!(SystemNotification)));
    #[cfg(windows)]
    let attempt = platform::send(&payload, cancel);
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
    fn notification_payload_uses_selected_language_and_bounds_error_length() {
        let target = PushTarget {
            branch: "feature".into(),
            local_ref: "refs/heads/feature".into(),
            local_oid: "a".repeat(40),
            remote: "origin".into(),
            remote_ref: "refs/heads/feature".into(),
            remote_oid: crate::domain::RemoteTip::Existing("b".repeat(40)),
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
            let payload = Payload::new(
                Path::new("/example/中文 \" repo"),
                &target,
                &"错误".repeat(300),
            );
            assert_eq!(payload.title, title);
            assert_eq!(payload.subtitle, "中文 \" repo / feature");
            assert!(payload.message.contains(details));
            assert!(payload.message.contains(&"错误".repeat(120)));
            assert!(!payload.message.contains(&"错误".repeat(121)));
        }
    }
}
