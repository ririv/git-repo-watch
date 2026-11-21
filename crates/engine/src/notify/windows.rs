//! Windows desktop notifications via the notification area.
use super::Payload;
use crate::{cancel::Cancel, platform::windows::powershell};
use anyhow::Result;

pub(super) fn send(payload: &Payload, cancel: &Cancel) -> Result<()> {
    let environment = [
        ("GW_NOTICE_TITLE".into(), payload.title.clone().into()),
        (
            "GW_NOTICE_BODY".into(),
            format!("{}\n{}", payload.subtitle, payload.message).into(),
        ),
    ];
    let script = r#"
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
$icon = [Windows.Forms.NotifyIcon]::new()
try {
    $icon.Icon = [Drawing.SystemIcons]::Warning
    $icon.Visible = $true
    $icon.ShowBalloonTip(7000, $env:GW_NOTICE_TITLE, $env:GW_NOTICE_BODY, 'Warning')
    Start-Sleep -Seconds 8
} finally {
    $icon.Dispose()
}
"#;
    powershell(script, &environment, cancel)?.success(i18n::tr!(SystemNotification))?;
    Ok(())
}
