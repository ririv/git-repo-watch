//! macOS Notification Center delivery.
use super::Payload;
use std::{ffi::OsString, path::Path};
const SCRIPT: &str = "on run argv\n display notification (item 1 of argv) with title (item 3 of argv) subtitle (item 2 of argv) sound name \"Basso\"\nend run";
pub(super) fn program() -> &'static Path {
    Path::new("/usr/bin/osascript")
}
pub(super) fn arguments(payload: &Payload) -> Vec<OsString> {
    vec![
        "-e".into(),
        SCRIPT.into(),
        "--".into(),
        payload.message.clone().into(),
        payload.subtitle.clone().into(),
        payload.title.clone().into(),
    ]
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn notification_payload_is_not_script_source() {
        let payload = Payload {
            title: "title".into(),
            subtitle: "repo".into(),
            message: "network \" failure".into(),
        };
        let args = arguments(&payload);
        assert_eq!(args[3], payload.message.as_str());
        assert!(!SCRIPT.contains(&payload.message));
        let temp = tempfile::tempdir().unwrap();
        let out = std::process::Command::new("/usr/bin/osacompile")
            .args(["-e", SCRIPT, "-o"])
            .arg(temp.path().join("test.scpt"))
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}
