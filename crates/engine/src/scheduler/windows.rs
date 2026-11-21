//! Task Scheduler registration for the current interactive Windows user.
use super::*;
use crate::platform::windows::{powershell, user_sid};
use std::ffi::OsString;

const CONNECT: &str = r#"
$service = New-Object -ComObject Schedule.Service
$service.Connect()
$folder = $service.GetFolder('\')
$sid = $env:GW_TASK_SID
$name = "${env:GW_TASK_NAME}-$sid"
$task = $null
try { $task = $folder.GetTask($name) }
catch { if ($_.Exception.GetBaseException().HResult -ne -2147024894) { throw } }
if ($null -ne $task) {
    $definition = $task.Definition
    if ($definition.RegistrationInfo.Description -cne $env:GW_TASK_MARKER -or
        $definition.Actions.Count -ne 1 -or
        $definition.Actions.Item(1).Path -ine $env:GW_TASK_BINARY) {
        if ($env:GW_TASK_READ -eq '1') { $task = $null }
        else { throw $env:GW_TASK_OWNER_ERROR }
    }
}
"#;
const REGISTER: &str = r#"
$definition = $service.NewTask(0)
$definition.XmlText = $env:GW_TASK_XML
$definition.Principal.UserId = $sid
$definition.Principal.LogonType = 3
if ($env:GW_TASK_ACTION -eq 'apply') {
    if ($null -eq $task) { throw $env:GW_TASK_MISSING_ERROR }
    $definition.Settings.Enabled = $task.Enabled
}
$flags = if ($null -eq $task) { 2 } else { 4 }
$null = $folder.RegisterTaskDefinition($name, $definition, $flags, $sid, $null, 3, $null)
"#;
const DISABLE: &str = r#"
if ($null -eq $task) { throw $env:GW_TASK_MISSING_ERROR }
$task.Enabled = $false
if ($task.GetInstances(0).Count -gt 0) { $task.Stop(0) }
"#;
const REMOVE: &str = r#"
if ($null -eq $task) { throw $env:GW_TASK_MISSING_ERROR }
$task.Enabled = $false
if ($task.GetInstances(0).Count -gt 0) { $task.Stop(0) }
$folder.DeleteTask($name, 0)
"#;

fn xml(value: &str) -> Result<String> {
    ensure!(
        !value.chars().any(char::is_control),
        i18n::tr!(InvalidScheduledInterval)
    );
    Ok(value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;"))
}

fn path_text(path: &Path) -> Result<&str> {
    path.to_str().context(i18n::tr!(InstallationPathNotUtf8))
}

fn arguments(store: &Store) -> Result<String> {
    match store {
        Store::Standard { .. } => Ok("scheduled-tick".into()),
        Store::Custom { home } => {
            let home = path_text(home)?;
            ensure!(!home.contains('"'), i18n::tr!(FolderInvalidPath));
            let trailing = home.len() - home.trim_end_matches('\\').len();
            Ok(format!(
                "--home \"{home}{}\" scheduled-tick",
                "\\".repeat(trailing)
            ))
        }
    }
}

fn trigger(plan: &timing::Plan) -> String {
    let slots = calendar::slots(plan);
    let first = &slots[0];
    let clock = format!(
        "{:02}:{:02}:00",
        first.hour.unwrap_or(0),
        first.minute.unwrap_or(0)
    );
    let date = plan.basis().at.date();
    let (repetition, calendar) = match first.weekday {
        Some(day) => {
            let weekdays = [
                "Sunday",
                "Monday",
                "Tuesday",
                "Wednesday",
                "Thursday",
                "Friday",
                "Saturday",
            ];
            (
                String::new(),
                format!(
                    "<ScheduleByWeek><WeeksInterval>1</WeeksInterval><DaysOfWeek><{}/></DaysOfWeek></ScheduleByWeek>",
                    weekdays[day as usize]
                ),
            )
        }
        None => {
            let step = if first.minute.is_none() {
                1
            } else if first.hour.is_none() {
                60 / slots.len()
            } else {
                1440 / slots.len()
            };
            let repetition = if step < 1440 {
                format!(
                    "<Repetition><Interval>PT{step}M</Interval><Duration>PT{}M</Duration><StopAtDurationEnd>false</StopAtDurationEnd></Repetition>",
                    1440 - step
                )
            } else {
                String::new()
            };
            (
                repetition,
                "<ScheduleByDay><DaysInterval>1</DaysInterval></ScheduleByDay>".into(),
            )
        }
    };
    format!(
        "<CalendarTrigger><StartBoundary>{date}T{clock}</StartBoundary>{repetition}{calendar}</CalendarTrigger>"
    )
}

fn definition(store: &Store, binary: &Path, plan: &timing::Plan) -> Result<String> {
    let marker = task_marker(store, binary);
    Ok(format!(
        r#"<?xml version="1.0"?>
<Task version="1.3" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo><Description>{marker}</Description><Source>{}</Source></RegistrationInfo>
  <Triggers>{}</Triggers>
  <Principals><Principal id="User"><LogonType>InteractiveToken</LogonType><RunLevel>LeastPrivilege</RunLevel></Principal></Principals>
  <Settings><MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy><DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries><StopIfGoingOnBatteries>false</StopIfGoingOnBatteries><StartWhenAvailable>false</StartWhenAvailable><ExecutionTimeLimit>PT0S</ExecutionTimeLimit></Settings>
  <Actions Context="User"><Exec><Command>{}</Command><Arguments>{}</Arguments><WorkingDirectory>{}</WorkingDirectory></Exec></Actions>
</Task>
"#,
        xml(&serde_json::to_string(plan)?)?,
        trigger(plan),
        xml(path_text(binary)?)?,
        xml(&arguments(store)?)?,
        xml(path_text(&store.state_dir())?)?,
    ))
}

enum RegistrationUpdate {
    Enable,
    Apply,
}

impl Manager {
    fn task_command(
        &self,
        store: &Store,
        script: &str,
        extra: &[(OsString, OsString)],
        cancel: &Cancel,
    ) -> Result<Vec<u8>> {
        let mut environment = vec![
            ("GW_TASK_NAME".into(), LABEL.into()),
            ("GW_TASK_SID".into(), user_sid()?.into()),
            (
                "GW_TASK_OWNER_ERROR".into(),
                i18n::tr!(ScheduleOwnedByAnotherInstallation).into(),
            ),
            (
                "GW_TASK_MISSING_ERROR".into(),
                i18n::tr!(ScheduleDefinitionMissing).into(),
            ),
            (
                "GW_TASK_MARKER".into(),
                task_marker(store, &self.binary()).into(),
            ),
            ("GW_TASK_BINARY".into(), self.binary().into_os_string()),
            ("GW_TASK_READ".into(), "0".into()),
        ];
        environment.extend_from_slice(extra);
        powershell(&format!("{CONNECT}\n{script}"), &environment, cancel)?
            .success(i18n::tr!(LoadSystemSchedule))
    }

    pub(crate) fn schedule_state(
        &self,
        store: &Store,
        cancel: &Cancel,
    ) -> Result<(Registration, Schedule)> {
        let output = self.task_command(
            store,
            r#"
if ($null -ne $task) {
    if ($task.Enabled) { 'enabled' } else { 'disabled' }
    $task.Definition.RegistrationInfo.Source
}
"#,
            &[("GW_TASK_READ".into(), "1".into())],
            cancel,
        )?;
        let text = std::str::from_utf8(&output)?;
        let mut lines = text.lines();
        let registration = match lines.next() {
            None => return Ok((Registration::NotLoaded, Schedule::Missing)),
            Some("enabled") => Registration::Loaded,
            Some("disabled") => Registration::NotLoaded,
            _ => anyhow::bail!(i18n::tr!(QuerySystemSchedule)),
        };
        let plan =
            serde_json::from_str(lines.next().context(i18n::tr!(InvalidScheduledInterval))?)?;
        Ok((registration, Schedule::Anchored { plan }))
    }

    fn register(
        &self,
        store: &Store,
        plan: &timing::Plan,
        update: RegistrationUpdate,
        cancel: &Cancel,
    ) -> Result<()> {
        let action = match update {
            RegistrationUpdate::Enable => "enable",
            RegistrationUpdate::Apply => "apply",
        };
        self.task_command(
            store,
            REGISTER,
            &[
                (
                    "GW_TASK_XML".into(),
                    definition(store, &self.binary(), plan)?.into(),
                ),
                ("GW_TASK_ACTION".into(), action.into()),
            ],
            cancel,
        )?;
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
        self.task_command(store, "", &[], cancel)?;
        let binary = self.binary();
        let data = fs::read(source)?;
        fs::create_dir_all(
            binary
                .parent()
                .context(i18n::tr!(InstallationPathHasNoParentDirectory))?,
        )?;
        Self::prepare_storage(store)?;
        // Windows 不能覆盖正在执行的同一文件；重复启用无需重写命令。
        let unchanged = match fs::read(&binary) {
            Ok(installed) => installed == data,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
            Err(error) => return Err(error.into()),
        };
        if !unchanged {
            state::atomic(&binary, &data, 0o755)?;
        }
        self.register(store, plan, RegistrationUpdate::Enable, cancel)
    }

    pub fn disable(&self, store: &Store, cancel: &Cancel) -> Result<()> {
        self.verify_owner(store)?;
        self.task_command(store, DISABLE, &[], cancel)?;
        Ok(())
    }

    pub fn apply(&self, store: &Store, plan: &timing::Plan, cancel: &Cancel) -> Result<()> {
        self.verify_owner(store)?;
        self.register(store, plan, RegistrationUpdate::Apply, cancel)
    }

    pub fn uninstall(&self, store: &Store, cancel: &Cancel) -> Result<()> {
        self.verify_owner(store)?;
        self.task_command(store, REMOVE, &[], cancel)?;
        Ok(())
    }
}
