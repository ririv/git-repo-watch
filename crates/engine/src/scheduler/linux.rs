//! systemd user service and monotonic timer integration.
use super::*;
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

fn service(store: &Store, binary: &Path, home: &Path) -> Result<String> {
    let (arguments, environment) = match store {
        Store::Custom { home } => (format!(" --home {}", quoted(home, true)?), String::new()),
        Store::Standard {
            configuration,
            state,
            ..
        } => {
            let config_base = configuration
                .parent()
                .context(i18n::tr!(InstallationPathHasNoParentDirectory))?;
            let state_base = state
                .parent()
                .context(i18n::tr!(InstallationPathHasNoParentDirectory))?;
            let environment = format!(
                "Environment={} {}\n",
                quoted(
                    Path::new(&format!(
                        "XDG_CONFIG_HOME={}",
                        config_base
                            .to_str()
                            .context(i18n::tr!(InstallationPathNotUtf8))?
                    )),
                    false
                )?,
                quoted(
                    Path::new(&format!(
                        "XDG_STATE_HOME={}",
                        state_base
                            .to_str()
                            .context(i18n::tr!(InstallationPathNotUtf8))?
                    )),
                    false
                )?
            );
            (String::new(), environment)
        }
    };
    let environment = format!(
        "Environment={}\n{}",
        quoted(
            Path::new(&format!(
                "HOME={}",
                home.to_str().context(i18n::tr!(InstallationPathNotUtf8))?
            )),
            false
        )?,
        environment
    );
    Ok(format!(
        "# {}\n[Unit]\nDescription=Git repository status check\n\n[Service]\nType=oneshot\nTimeoutStartSec=infinity\nSuccessExitStatus=1\nWorkingDirectory={}\n{}ExecStart={}{} scheduled-tick\n",
        task_marker(store, binary),
        quoted(&store.state_dir(), false)?,
        environment,
        quoted(binary, true)?,
        arguments
    ))
}
#[cfg(test)]
fn timer(store: &Store, binary: &Path, interval: u64) -> String {
    format!(
        "# {}\n[Unit]\nDescription=Periodic Git repository status check\n\n[Timer]\nOnActiveSec={interval}s\nOnUnitActiveSec={interval}s\nUnit={LABEL}.service\n\n[Install]\nWantedBy=timers.target\n",
        task_marker(store, binary)
    )
}
fn calendar_events(plan: &timing::Plan) -> String {
    let weekdays = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    calendar::slots(plan)
        .into_iter()
        .map(|slot| {
            let weekday = slot
                .weekday
                .map(|day| format!("{} ", weekdays[day as usize]))
                .unwrap_or_default();
            let hour = slot
                .hour
                .map(|hour| format!("{hour:02}"))
                .unwrap_or_else(|| "*".into());
            let minute = slot
                .minute
                .map(|minute| format!("{minute:02}"))
                .unwrap_or_else(|| "*".into());
            format!("OnCalendar={weekday}*-*-* {hour}:{minute}:00\n")
        })
        .collect::<String>()
}
fn anchored_timer(store: &Store, binary: &Path, plan: &timing::Plan) -> Result<String> {
    let events = calendar_events(plan);
    Ok(format!(
        "# {}\n# {}\n[Unit]\nDescription=Anchored Git repository status check\n\n[Timer]\n{events}AccuracySec=1s\nRandomizedDelaySec=0\nPersistent=false\nUnit={LABEL}.service\n\n[Install]\nWantedBy=timers.target\n",
        task_marker(store, binary),
        plan_comment(plan)?
    ))
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
        self.directories.config_base.join("systemd/user")
    }
    pub(super) fn service(&self) -> PathBuf {
        self.units().join(format!("{LABEL}.service"))
    }
    pub fn timer(&self) -> PathBuf {
        self.units().join(format!("{LABEL}.timer"))
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
            Ok(contents) => match read_plan(&contents, "# schedule-plan=", "")? {
                Some(plan) => {
                    let actual = contents
                        .lines()
                        .filter(|line| line.starts_with("OnCalendar="))
                        .map(|line| format!("{line}\n"))
                        .collect::<String>();
                    ensure!(
                        actual == calendar_events(&plan) && !contents.contains("OnActiveSec="),
                        i18n::tr!(InvalidScheduledInterval)
                    );
                    Ok(Schedule::Anchored { plan })
                }
                None => Ok(Schedule::Saved {
                    interval_seconds: interval(&contents)?,
                }),
            },
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
    pub fn install(
        &self,
        store: &Store,
        source: &Path,
        plan: &timing::Plan,
        cancel: &Cancel,
    ) -> Result<()> {
        self.verify_install_target(store)?;
        let data = fs::read(source)?;
        let payload = service(store, &self.binary(), &self.user_home)?;
        // 在任何写入之前确认用户服务管理器可连接。
        self.control(&["show-environment"], cancel)?;
        for path in [self.binary(), self.timer()] {
            fs::create_dir_all(
                path.parent()
                    .context(i18n::tr!(InstallationPathHasNoParentDirectory))?,
            )?;
        }
        state::atomic(&self.binary(), &data, 0o755)?;
        Self::prepare_storage(store)?;
        state::atomic(&self.service(), payload.as_bytes(), 0o600)?;
        state::atomic(
            &self.timer(),
            anchored_timer(store, &self.binary(), plan)?.as_bytes(),
            0o600,
        )?;
        self.control(&["daemon-reload"], cancel)?;
        let name = format!("{LABEL}.timer");
        self.control(&["enable", &name], cancel)?;
        self.control(&["restart", &name], cancel)
    }
    pub fn disable(&self, store: &Store, cancel: &Cancel) -> Result<()> {
        self.verify_owner(store)?;
        ensure!(
            self.timer().try_exists()? && self.service().try_exists()?,
            i18n::tr!(ScheduleDefinitionMissing)
        );
        self.control(&["disable", "--now", &format!("{LABEL}.timer")], cancel)
    }
    pub fn apply(&self, store: &Store, plan: &timing::Plan, cancel: &Cancel) -> Result<()> {
        self.verify_owner(store)?;
        ensure!(
            self.timer().try_exists()? && self.service().try_exists()?,
            i18n::tr!(ScheduleDefinitionMissing)
        );
        let previous = self.registration(cancel)?;
        let payload = service(store, &self.binary(), &self.user_home)?;
        self.control(&["show-environment"], cancel)?;
        state::atomic(&self.service(), payload.as_bytes(), 0o600)?;
        state::atomic(
            &self.timer(),
            anchored_timer(store, &self.binary(), plan)?.as_bytes(),
            0o600,
        )?;
        self.control(&["daemon-reload"], cancel)?;
        if previous == Registration::Loaded {
            self.control(&["restart", &format!("{LABEL}.timer")], cancel)?;
        }
        Ok(())
    }
    pub fn uninstall(&self, store: &Store, cancel: &Cancel) -> Result<()> {
        self.verify_owner(store)?;
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
        use crate::directories::xdg_directory;
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
            &Store::custom("/example/中文 $USER %h \"\nfolder".into()),
            Path::new("/example/a b"),
            Path::new("/example/user"),
        )
        .unwrap();
        assert!(payload.contains("WorkingDirectory=\"/example/中文 $USER %%h \\\"\\nfolder/var\""));
        assert!(payload.contains("ExecStart=\"/example/a b\" --home \"/example/中文 $$USER %%h"));
        assert!(
            payload
                .lines()
                .any(|line| line.starts_with("ExecStart=") && line.ends_with(" scheduled-tick"))
        );
        assert_eq!(
            payload
                .lines()
                .filter(|line| line.starts_with("ExecStart="))
                .count(),
            1
        );
    }
    #[test]
    fn standard_service_captures_xdg_and_home_without_binding_to_a_checkout() {
        let home = Path::new("/example/用户 空格");
        let store = crate::directories::UserDirectories::linux(
            home,
            Some("/example/配置 %h $USER".into()),
            Some("/example/状态 空格".into()),
        )
        .unwrap()
        .store();
        let payload = service(&store, Path::new("/example/bin"), home).unwrap();
        assert!(!payload.contains(" --home "));
        assert!(payload.contains("Environment=\"XDG_CONFIG_HOME=/example/配置 %%h $USER\" \"XDG_STATE_HOME=/example/状态 空格\""));
        assert!(payload.contains("Environment=\"HOME=/example/用户 空格\""));
        assert!(payload.contains("WorkingDirectory=\"/example/状态 空格/"));
        assert!(
            payload
                .lines()
                .any(|line| line == "ExecStart=\"/example/bin\" scheduled-tick")
        );
    }

    #[test]
    fn anchored_calendar_uses_local_clock_and_preserves_the_persisted_plan() {
        let plan = timing::Plan::new(
            21600,
            timing::Basis {
                source: timing::Source::Explicit,
                at: chrono::NaiveDate::from_ymd_opt(2026, 10, 23)
                    .unwrap()
                    .and_hms_opt(0, 0, 0)
                    .unwrap(),
            },
        )
        .unwrap();
        let payload = anchored_timer(
            &Store::custom("/example".into()),
            Path::new("/example/bin"),
            &plan,
        )
        .unwrap();
        assert_eq!(
            read_plan(&payload, "# schedule-plan=", "")
                .unwrap()
                .unwrap(),
            plan
        );
        for hour in ["00", "06", "12", "18"] {
            assert!(payload.contains(&format!("OnCalendar=*-*-* {hour}:00:00")));
        }
        assert!(payload.contains("AccuracySec=1s") && payload.contains("Persistent=false"));
        assert!(!payload.contains("OnActiveSec="));
    }
    #[test]
    fn timer_delays_first_run_and_preserves_interval() {
        let payload = timer(
            &Store::custom("/example".into()),
            Path::new("/example/bin"),
            21600,
        );
        assert_eq!(interval(&payload).unwrap(), 21600);
        assert!(payload.contains("OnActiveSec=21600s\nOnUnitActiveSec=21600s"));
        assert!(interval("OnActiveSec=1s").is_err());
        assert!(interval("OnActiveSec=60s\nOnActiveSec=120s").is_err());
        assert!(interval("OnActiveSec=invalid").is_err());
    }
}
