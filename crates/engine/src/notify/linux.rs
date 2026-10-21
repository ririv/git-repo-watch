//! Linux freedesktop desktop notifications through libnotify.
use super::Payload;
use std::{ffi::OsString, path::Path};
pub(super) fn program() -> &'static Path {
    Path::new("notify-send")
}
pub(super) fn arguments(payload: &Payload) -> Vec<OsString> {
    vec![
        "--urgency=critical".into(),
        "--".into(),
        payload.title.clone().into(),
        format!("{}\n{}", payload.subtitle, payload.message).into(),
    ]
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn notification_payload_is_literal_and_urgent() {
        let payload = Payload {
            title: "title".into(),
            subtitle: "-repo".into(),
            message: "<error> & \"quoted\"".into(),
        };
        assert_eq!(
            arguments(&payload),
            vec![
                OsString::from("--urgency=critical"),
                "--".into(),
                "title".into(),
                "-repo\n<error> & \"quoted\"".into()
            ]
        );
    }
}
