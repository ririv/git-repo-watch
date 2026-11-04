//! Discover common macOS editor bundles by their verified application identity.
use anyhow::Result;
use std::path::PathBuf;

/// Locate common app bundles and verify their identity without querying Launch Services.
pub(super) fn standard_application(
    known: super::Known,
    home: &std::path::Path,
    cancel: &crate::cancel::Cancel,
) -> Result<Option<PathBuf>> {
    use std::{ffi::OsString, time::Duration};
    let spec = known.spec();
    let Some(bundle) = spec.bundle else {
        return Ok(None);
    };
    let name = match known {
        super::Known::CodeInsiders => "Visual Studio Code - Insiders",
        _ => spec.label,
    };
    for directory in [
        home.join("Applications"),
        PathBuf::from("/Applications"),
        PathBuf::from("/System/Applications"),
    ] {
        cancel.check()?;
        let application = directory.join(format!("{name}.app"));
        match std::fs::metadata(&application) {
            Ok(metadata) if metadata.is_dir() => {}
            Ok(_) => continue,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        }
        let arguments = [
            OsString::from("-extract"),
            "CFBundleIdentifier".into(),
            "raw".into(),
            "-o".into(),
            "-".into(),
            application.join("Contents/Info.plist").into_os_string(),
        ];
        let output = crate::process::run(
            std::path::Path::new("/usr/bin/plutil"),
            &arguments,
            None,
            cancel,
            Duration::from_secs(5),
        )?
        .success(i18n::tr!(ExternalEditorDetectionFailed))?;
        if std::str::from_utf8(&output)?.trim_end() == bundle {
            return Ok(Some(application));
        }
    }
    Ok(None)
}
