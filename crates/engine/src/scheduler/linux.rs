//! systemd user service and monotonic timer integration.
use super::*;
#[cfg(target_os = "linux")]
#[derive(Debug, Clone)]
pub(super) struct Directories {
    config: PathBuf,
    data: PathBuf,
}
#[cfg(target_os = "linux")]
impl Directories {
    pub(super) fn defaults(home: &Path) -> Self {
        Self {
            config: home.join(".config"),
            data: home.join(".local/share"),
        }
    }
    pub(super) fn from_environment(home: &Path) -> Result<Self> {
        let default = Self::defaults(home);
        Ok(Self {
            config: xdg_directory(
                "XDG_CONFIG_HOME",
                std::env::var_os("XDG_CONFIG_HOME"),
                default.config,
            )?,
            data: xdg_directory(
                "XDG_DATA_HOME",
                std::env::var_os("XDG_DATA_HOME"),
                default.data,
            )?,
        })
    }
}

fn xdg_directory(
    name: &str,
    value: Option<std::ffi::OsString>,
    default: PathBuf,
) -> Result<PathBuf> {
    match value.filter(|v| !v.is_empty()) {
        Some(value) => {
            let path = PathBuf::from(value);
            ensure!(
                path.is_absolute(),
                "{name}: {}",
                i18n::tr!(XdgDirectoryMustBeAbsolute)
            );
            Ok(path)
        }
        None => Ok(default),
    }
}

fn quoted(value: &Path, exec: bool) -> Result<String> {
    let value = value.to_str().context(i18n::tr!(InstallationPathNotUtf8))?;
    let mut out = String::from("\"");
    for c in value.chars() {
        match c {
            '%' => out.push_str("%%"),
            '$' if exec => out.push_str("$$"),
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => anyhow::bail!(i18n::tr!(SystemdPathControlUnsupported)),
            c => out.push(c),
        }
    }
    out.push('"');
    Ok(out)
}

fn service(root: &Path, binary: &Path) -> Result<String> {
    Ok(format!(
        "[Unit]\nDescription=Git repository status check\n\n[Service]\nType=oneshot\nTimeoutStartSec=infinity\nSuccessExitStatus=1\nWorkingDirectory={}\nExecStart={} --home {} check\n",
        quoted(root, false)?,
        quoted(binary, true)?,
        quoted(root, true)?
    ))
}
fn timer(interval: u64) -> String {
    format!(
        "[Unit]\nDescription=Periodic Git repository status check\n\n[Timer]\nOnActiveSec={interval}s\nOnUnitActiveSec={interval}s\nUnit={LABEL}.service\n\n[Install]\nWantedBy=timers.target\n"
    )
}
fn interval(contents: &str) -> Result<u64> {
    let entries = contents
        .lines()
        .filter_map(|line| line.strip_prefix("OnActiveSec="))
        .collect::<Vec<_>>();
    ensure!(entries.len() == 1, i18n::tr!(InvalidScheduledInterval));
    let value = entries[0]
        .strip_suffix('s')
        .context(i18n::tr!(InvalidScheduledInterval))?
        .parse::<u64>()?;
    ensure!(value >= 60, i18n::tr!(InvalidScheduledInterval));
    Ok(value)
}

#[cfg(target_os = "linux")]
impl Manager {
    fn units(&self) -> PathBuf {
        self.directories.config.join("systemd/user")
    }
    fn service(&self) -> PathBuf {
        self.units().join(format!("{LABEL}.service"))
    }
    pub fn timer(&self) -> PathBuf {
        self.units().join(format!("{LABEL}.timer"))
    }
    pub fn location(&self) -> PathBuf {
        self.directories
            .data
            .join(identity::DATA_DIRECTORY)
            .join("location.json")
    }
    pub fn registration(&self, cancel: &Cancel) -> Result<Registration> {
        if !self.timer().try_exists()? {
            return Ok(Registration::NotLoaded);
        }
        let out = process::system(
            "systemctl",
            &["--user", "is-active", &format!("{LABEL}.timer")],
            cancel,
        )?;
        match (out.code, std::str::from_utf8(&out.stdout)?.trim()) {
            (0, "active") => Ok(Registration::Loaded),
            (3, "inactive" | "failed") | (4, "unknown") => Ok(Registration::NotLoaded),
            _ => {
                out.success(i18n::tr!(QuerySystemSchedule))?;
                anyhow::bail!(i18n::tr!(UnexpectedSystemdTimerState))
            }
        }
    }
    pub fn schedule(&self, cancel: &Cancel) -> Result<Schedule> {
        cancel.check()?;
        match fs::read_to_string(self.timer()) {
            Ok(contents) => Ok(Schedule::Saved {
                interval_seconds: interval(&contents)?,
            }),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Schedule::Missing),
            Err(e) => Err(e.into()),
        }
    }
    fn control(&self, args: &[&str], cancel: &Cancel) -> Result<()> {
        let mut command = vec!["--user"];
        command.extend_from_slice(args);
        process::system("systemctl", &command, cancel)?.success(i18n::tr!(LoadSystemSchedule))?;
        Ok(())
    }
    pub fn install(&self, root: &Path, source: &Path, seconds: u64, cancel: &Cancel) -> Result<()> {
        ensure!(
            root.is_absolute() && seconds >= 60,
            i18n::tr!(InvalidInstallationArguments)
        );
        let data = fs::read(source)?;
        match fs::read(self.binary()) {
            Ok(old) => ensure!(
                digest(&old) == self.read_location()?.binary_sha256,
                i18n::tr!(InstalledCommandOwnershipMismatch)
            ),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
        let payload = service(root, &self.binary())?;
        // 在任何写入之前确认用户服务管理器可连接。
        self.control(&["show-environment"], cancel)?;
        for path in [
            self.binary(),
            self.location(),
            self.timer(),
            root.join("var/placeholder"),
        ] {
            fs::create_dir_all(
                path.parent()
                    .context(i18n::tr!(InstallationPathHasNoParentDirectory))?,
            )?;
        }
        state::atomic(&self.binary(), &data, 0o755)?;
        state::write_json(
            &self.location(),
            &Location {
                home: root.to_owned(),
                binary_sha256: digest(&data),
            },
        )?;
        state::atomic(&self.service(), payload.as_bytes(), 0o600)?;
        state::atomic(&self.timer(), timer(seconds).as_bytes(), 0o600)?;
        self.control(&["daemon-reload"], cancel)?;
        let name = format!("{LABEL}.timer");
        self.control(&["enable", &name], cancel)?;
        self.control(&["restart", &name], cancel)
    }
    pub fn uninstall(&self, cancel: &Cancel) -> Result<()> {
        self.control(&["disable", "--now", &format!("{LABEL}.timer")], cancel)?;
        self.control(&["stop", &format!("{LABEL}.service")], cancel)?;
        fs::remove_file(self.timer())?;
        fs::remove_file(self.service())?;
        self.control(&["daemon-reload"], cancel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn xdg_directories_use_explicit_absolute_values_and_default_only_when_unset() {
        let default = PathBuf::from("/example/.config");
        assert_eq!(
            xdg_directory("XDG_CONFIG_HOME", None, default.clone()).unwrap(),
            default
        );
        assert_eq!(
            xdg_directory("XDG_CONFIG_HOME", Some("".into()), default.clone()).unwrap(),
            default
        );
        assert_eq!(
            xdg_directory(
                "XDG_CONFIG_HOME",
                Some("/example/custom".into()),
                default.clone()
            )
            .unwrap(),
            PathBuf::from("/example/custom")
        );
        assert!(xdg_directory("XDG_CONFIG_HOME", Some("relative".into()), default).is_err());
    }
    #[test]
    fn service_paths_are_quoted_and_specifiers_cannot_expand() {
        let payload = service(
            Path::new("/example/中文 $USER %h \"\nfolder"),
            Path::new("/example/a b"),
        )
        .unwrap();
        assert!(payload.contains("WorkingDirectory=\"/example/中文 $USER %%h \\\"\\nfolder\""));
        assert!(payload.contains("ExecStart=\"/example/a b\" --home \"/example/中文 $$USER %%h"));
        assert_eq!(
            payload
                .lines()
                .filter(|line| line.starts_with("ExecStart="))
                .count(),
            1
        );
    }
    #[test]
    fn timer_delays_first_run_and_preserves_interval() {
        let payload = timer(21600);
        assert_eq!(interval(&payload).unwrap(), 21600);
        assert!(payload.contains("OnActiveSec=21600s\nOnUnitActiveSec=21600s"));
        assert!(interval("OnActiveSec=1s").is_err());
        assert!(interval("OnActiveSec=60s\nOnActiveSec=120s").is_err());
        assert!(interval("OnActiveSec=invalid").is_err());
    }
}
