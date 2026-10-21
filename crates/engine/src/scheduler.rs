//! Ownership-checked native installation and launchd integration.
use crate::{cancel::Cancel, process, state};
use anyhow::{Context, Result, ensure};
pub use identity::SERVICE_LABEL as LABEL;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};
#[derive(Debug, Clone)]
pub struct Manager {
    pub user_home: PathBuf,
    pub uid: u32,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Registration {
    Loaded,
    NotLoaded,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Schedule {
    Missing,
    Saved { interval_seconds: u64 },
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Location {
    pub home: PathBuf,
    pub binary_sha256: String,
}
impl Manager {
    pub fn new() -> Result<Self> {
        let user_home = std::env::var_os("HOME").context(i18n::tr!(CannotReadUserHomeDirectory))?;
        let user_home = PathBuf::from(user_home);
        ensure!(
            user_home.is_absolute(),
            i18n::tr!(UserHomeDirectoryIsNotAnAbsolute)
        );
        Ok(Self {
            user_home,
            uid: unsafe { libc::getuid() },
        })
    }
    pub fn target(&self) -> String {
        format!("gui/{}/{}", self.uid, LABEL)
    }
    pub fn binary(&self) -> PathBuf {
        self.user_home.join(".local/bin").join(identity::COMMAND)
    }
    pub fn plist(&self) -> PathBuf {
        self.user_home
            .join("Library/LaunchAgents")
            .join(format!("{LABEL}.plist"))
    }
    pub fn location(&self) -> PathBuf {
        self.user_home
            .join("Library/Application Support")
            .join(identity::DATA_DIRECTORY)
            .join("location.json")
    }
    pub fn read_location(&self) -> Result<Location> {
        let l: Location = state::read_json(&self.location())?;
        ensure!(
            l.home.is_absolute(),
            i18n::tr!(InstallationLocationIsNotAnAbsolutePath)
        );
        Ok(l)
    }
    pub fn registration(&self, cancel: &Cancel) -> Result<Registration> {
        let out = process::system("/bin/launchctl", &["print", &self.target()], cancel)?;
        match out.code {
            0 => Ok(Registration::Loaded),
            113 => Ok(Registration::NotLoaded),
            _ => {
                out.success(i18n::tr!(QuerySystemSchedule))?;
                unreachable!()
            }
        }
    }
    pub fn schedule(&self, cancel: &Cancel) -> Result<Schedule> {
        let path = self.plist();
        if !path.try_exists()? {
            return Ok(Schedule::Missing);
        }
        let out = process::system(
            "/usr/bin/plutil",
            &[
                "-extract",
                "StartInterval",
                "raw",
                "-o",
                "-",
                path.to_str().context(i18n::tr!(SchedulePathIsNotUtf8))?,
            ],
            cancel,
        )?
        .success(i18n::tr!(ReadScheduledInterval))?;
        let interval = std::str::from_utf8(&out)?.trim().parse::<u64>()?;
        ensure!(interval >= 60, i18n::tr!(InvalidScheduledInterval));
        Ok(Schedule::Saved {
            interval_seconds: interval,
        })
    }
    pub fn install(
        &self,
        root: &Path,
        source: &Path,
        interval: u64,
        cancel: &Cancel,
    ) -> Result<()> {
        ensure!(
            root.is_absolute() && interval >= 60,
            i18n::tr!(InvalidInstallationArguments)
        );
        let data = fs::read(source)?;
        match fs::read(self.binary()) {
            Ok(old) => {
                let l = self.read_location()?;
                ensure!(
                    digest(&old) == l.binary_sha256,
                    i18n::tr!(InstalledCommandOwnershipMismatch)
                );
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
        let previous = self.registration(cancel)?;
        for path in [
            self.binary(),
            self.location(),
            self.plist(),
            root.join("var/placeholder"),
        ] {
            let parent = path
                .parent()
                .context(i18n::tr!(InstallationPathHasNoParentDirectory))?;
            fs::create_dir_all(parent)?;
        }
        let plist = plist(root, &self.binary(), interval)?;
        let staged = tempfile::Builder::new().prefix(".schedule-").tempfile_in(
            self.plist()
                .parent()
                .context(i18n::tr!(SchedulePathHasNoParentDirectory))?,
        )?;
        fs::write(staged.path(), &plist)?;
        process::system(
            "/usr/bin/plutil",
            &[
                "-lint",
                staged
                    .path()
                    .to_str()
                    .context(i18n::tr!(TemporaryPathIsNotUtf8))?,
            ],
            cancel,
        )?
        .success(i18n::tr!(ValidateSystemSchedule))?;
        state::atomic(&self.binary(), &data, 0o755)?;
        state::write_json(
            &self.location(),
            &Location {
                home: root.to_owned(),
                binary_sha256: digest(&data),
            },
        )?;
        state::atomic(&self.plist(), plist.as_bytes(), 0o600)?;
        if previous == Registration::Loaded {
            process::system("/bin/launchctl", &["bootout", &self.target()], cancel)?
                .success(i18n::tr!(UnloadOldSystemSchedule))?;
        }
        process::system(
            "/bin/launchctl",
            &[
                "bootstrap",
                &format!("gui/{}", self.uid),
                self.plist()
                    .to_str()
                    .context(i18n::tr!(SchedulePathIsNotUtf8))?,
            ],
            cancel,
        )?
        .success(i18n::tr!(LoadSystemSchedule))?;
        Ok(())
    }
    pub fn uninstall(&self, cancel: &Cancel) -> Result<()> {
        if self.registration(cancel)? == Registration::Loaded {
            process::system("/bin/launchctl", &["bootout", &self.target()], cancel)?
                .success(i18n::tr!(UnloadSystemSchedule))?;
        }
        fs::remove_file(self.plist())?;
        Ok(())
    }
}
pub fn digest(data: &[u8]) -> String {
    format!("{:x}", Sha256::digest(data))
}
fn xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
pub fn plist(root: &Path, binary: &Path, interval: u64) -> Result<String> {
    let root = root.to_str().context(i18n::tr!(InstallationPathNotUtf8))?;
    let binary = binary.to_str().context(i18n::tr!(CommandPathNotUtf8))?;
    Ok(format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>Label</key><string>{LABEL}</string>
<key>ProgramArguments</key><array><string>{binary}</string><string>--home</string><string>{root}</string><string>check</string></array>
<key>WorkingDirectory</key><string>{root}</string>
<key>StartInterval</key><integer>{interval}</integer>
<key>RunAtLoad</key><false/>
<key>ProcessType</key><string>Background</string>
<key>EnvironmentVariables</key><dict><key>PATH</key><string>/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin:/sbin</string></dict>
<key>StandardOutPath</key><string>{root}/var/launchd.stdout.log</string>
<key>StandardErrorPath</key><string>{root}/var/launchd.stderr.log</string>
</dict></plist>
"#,
        binary = xml(binary),
        root = xml(root)
    ))
}
