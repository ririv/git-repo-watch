//! macOS launchd integration.
use super::*;
impl Manager {
    pub fn target(&self) -> String {
        format!("gui/{}/{}", self.uid, LABEL)
    }
    pub fn plist(&self) -> PathBuf {
        self.user_home
            .join("Library/LaunchAgents")
            .join(format!("{LABEL}.plist"))
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
        let contents = fs::read_to_string(&path)?;
        let out = process::system(
            "/usr/bin/plutil",
            &[
                "-convert",
                "json",
                "-o",
                "-",
                path.to_str().context(i18n::tr!(SchedulePathIsNotUtf8))?,
            ],
            cancel,
        )?
        .success(i18n::tr!(ReadScheduledInterval))?;
        let value: serde_json::Value = serde_json::from_slice(&out)?;
        if let Some(plan) = read_plan(&contents, "<!-- schedule-plan=", " -->")? {
            ensure!(
                value["StartCalendarInterval"] == calendar_value(&plan),
                i18n::tr!(InvalidScheduledInterval)
            );
            ensure!(
                value["StartInterval"].is_null(),
                i18n::tr!(InvalidScheduledInterval)
            );
            ensure!(
                value["ProgramArguments"]
                    .as_array()
                    .is_some_and(|args| args.last().is_some_and(|arg| arg == "scheduled-tick")),
                i18n::tr!(InvalidScheduledInterval)
            );
            return Ok(Schedule::Anchored { plan });
        }
        let interval_seconds = value["StartInterval"]
            .as_u64()
            .context(i18n::tr!(InvalidScheduledInterval))?;
        duration::validate(interval_seconds)?;
        Ok(Schedule::Saved { interval_seconds })
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
        let previous = self.registration(cancel)?;
        for path in [self.binary(), self.plist()] {
            let parent = path
                .parent()
                .context(i18n::tr!(InstallationPathHasNoParentDirectory))?;
            fs::create_dir_all(parent)?;
        }
        Self::prepare_storage(store)?;
        let plist = anchored_plist(store, &self.binary(), plan, &self.user_home)?;
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
        state::atomic(&self.plist(), plist.as_bytes(), 0o600)?;
        if previous == Registration::Loaded {
            process::system("/bin/launchctl", &["bootout", &self.target()], cancel)?
                .success(i18n::tr!(UnloadOldSystemSchedule))?;
        }
        process::system("/bin/launchctl", &["enable", &self.target()], cancel)?
            .success(i18n::tr!(EnableScheduledTask))?;
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
    pub fn disable(&self, store: &Store, cancel: &Cancel) -> Result<()> {
        self.verify_owner(store)?;
        ensure!(
            self.plist().try_exists()?,
            i18n::tr!(ScheduleDefinitionMissing)
        );
        if self.registration(cancel)? == Registration::Loaded {
            process::system("/bin/launchctl", &["bootout", &self.target()], cancel)?
                .success(i18n::tr!(UnloadSystemSchedule))?;
        }
        // 仅 bootout 会在下次登录时重新加载；同时保存 launchd 的禁用覆盖项。
        process::system("/bin/launchctl", &["disable", &self.target()], cancel)?
            .success(i18n::tr!(DisableScheduledTask))?;
        Ok(())
    }
    pub fn apply(&self, store: &Store, plan: &timing::Plan, cancel: &Cancel) -> Result<()> {
        self.verify_owner(store)?;
        ensure!(
            self.plist().try_exists()?,
            i18n::tr!(ScheduleDefinitionMissing)
        );
        let previous = self.registration(cancel)?;
        let payload = anchored_plist(store, &self.binary(), plan, &self.user_home)?;
        let staged = tempfile::NamedTempFile::new_in(
            self.plist()
                .parent()
                .context(i18n::tr!(SchedulePathHasNoParentDirectory))?,
        )?;
        fs::write(staged.path(), &payload)?;
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
        state::atomic(&self.plist(), payload.as_bytes(), 0o600)?;
        if previous == Registration::Loaded {
            process::system("/bin/launchctl", &["bootout", &self.target()], cancel)?
                .success(i18n::tr!(UnloadOldSystemSchedule))?;
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
        }
        Ok(())
    }
    pub fn uninstall(&self, store: &Store, cancel: &Cancel) -> Result<()> {
        self.verify_owner(store)?;
        if self.registration(cancel)? == Registration::Loaded {
            process::system("/bin/launchctl", &["bootout", &self.target()], cancel)?
                .success(i18n::tr!(UnloadSystemSchedule))?;
        }
        fs::remove_file(self.plist())?;
        Ok(())
    }
}
fn xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
pub fn plist(store: &Store, binary: &Path, interval: u64, user_home: &Path) -> Result<String> {
    let executable = binary.to_str().context(i18n::tr!(CommandPathNotUtf8))?;
    let arguments = match store {
        Store::Custom { home } => format!(
            "<string>--home</string><string>{}</string>",
            xml(home.to_str().context(i18n::tr!(InstallationPathNotUtf8))?)
        ),
        Store::Standard { .. } => String::new(),
    };
    let working = store.state_dir();
    let logs = store.log_dir();
    Ok(format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<!-- {marker} -->
<plist version="1.0"><dict>
<key>Label</key><string>{LABEL}</string>
<key>ProgramArguments</key><array><string>{binary}</string>{arguments}<string>scheduled-check</string></array>
<key>WorkingDirectory</key><string>{working}</string>
<key>StartInterval</key><integer>{interval}</integer>
<key>RunAtLoad</key><false/>
<key>ProcessType</key><string>Background</string>
<key>EnvironmentVariables</key><dict><key>HOME</key><string>{user_home}</string><key>PATH</key><string>/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin:/sbin</string></dict>
<key>StandardOutPath</key><string>{stdout}</string>
<key>StandardErrorPath</key><string>{stderr}</string>
</dict></plist>
"#,
        marker = task_marker(store, binary),
        user_home = xml(user_home
            .to_str()
            .context(i18n::tr!(InstallationPathNotUtf8))?),
        binary = xml(executable),
        working = xml(working
            .to_str()
            .context(i18n::tr!(InstallationPathNotUtf8))?),
        stdout = xml(logs
            .join("launchd.stdout.log")
            .to_str()
            .context(i18n::tr!(InstallationPathNotUtf8))?),
        stderr = xml(logs
            .join("launchd.stderr.log")
            .to_str()
            .context(i18n::tr!(InstallationPathNotUtf8))?),
    ))
}

fn calendar_value(plan: &timing::Plan) -> serde_json::Value {
    serde_json::Value::Array(
        calendar::slots(plan)
            .into_iter()
            .map(|slot| {
                let mut values = serde_json::Map::new();
                for (key, value) in [
                    ("Weekday", slot.weekday),
                    ("Hour", slot.hour),
                    ("Minute", slot.minute),
                ] {
                    if let Some(value) = value {
                        values.insert(key.into(), value.into());
                    }
                }
                values.into()
            })
            .collect(),
    )
}
fn anchored_plist(
    store: &Store,
    binary: &Path,
    plan: &timing::Plan,
    user_home: &Path,
) -> Result<String> {
    let entries = calendar::slots(plan)
        .into_iter()
        .map(|slot| {
            let mut entry = String::from("<dict>");
            for (key, value) in [
                ("Weekday", slot.weekday),
                ("Hour", slot.hour),
                ("Minute", slot.minute),
            ] {
                if let Some(value) = value {
                    entry.push_str(&format!("<key>{key}</key><integer>{value}</integer>"));
                }
            }
            entry.push_str("</dict>");
            entry
        })
        .collect::<String>();
    let payload = plist(store, binary, plan.interval_seconds(), user_home)?
        .replace(
            &format!(
                "<key>StartInterval</key><integer>{}</integer>",
                plan.interval_seconds()
            ),
            &format!("<key>StartCalendarInterval</key><array>{entries}</array>"),
        )
        .replace(
            "<string>scheduled-check</string>",
            "<string>scheduled-tick</string>",
        )
        .replace(
            "<plist version=",
            &format!("<!-- {} -->\n<plist version=", plan_comment(plan)?),
        );
    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn anchored_plists_round_trip_through_native_plutil_without_installing_a_task() {
        let temporary = tempfile::tempdir().unwrap();
        let manager = Manager::for_user(temporary.path().join("user"), 99999).unwrap();
        let store = Store::custom(temporary.path().join("profile"));
        fs::create_dir_all(manager.plist().parent().unwrap()).unwrap();
        let at = chrono::NaiveDate::from_ymd_opt(2026, 10, 23)
            .unwrap()
            .and_hms_opt(0, 0, 17)
            .unwrap();
        for seconds in [90, 420, 21600, 172800, 604800] {
            let plan = timing::Plan::new(
                seconds,
                timing::Basis {
                    source: timing::Source::Explicit,
                    at,
                },
            )
            .unwrap();
            let payload =
                anchored_plist(&store, &manager.binary(), &plan, manager.user_home()).unwrap();
            assert!(
                payload.contains("<key>RunAtLoad</key><false/>")
                    && !payload.contains("<key>StartInterval</key>")
            );
            fs::write(manager.plist(), payload).unwrap();
            assert!(
                matches!(manager.schedule(&Cancel::default()).unwrap(), Schedule::Anchored { plan: actual } if actual == plan)
            );
        }
    }
}
