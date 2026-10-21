//! Cancellable subprocess execution with process-group cleanup.
use crate::cancel::Cancel;
use anyhow::{Context, Result, bail};
use std::{
    ffi::OsString,
    io::Read,
    os::unix::process::CommandExt,
    path::Path,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};
#[derive(Debug)]
pub struct Output {
    pub code: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}
impl Output {
    pub fn success(self, label: &str) -> Result<Vec<u8>> {
        if self.code == 0 {
            Ok(self.stdout)
        } else {
            bail!(gw_i18n::tf!(
                LabelFailed,
                self.code,
                String::from_utf8_lossy(&self.stderr),
                String::from_utf8_lossy(&self.stdout),
                label = label
            ))
        }
    }
}
pub fn run(
    program: &Path,
    args: &[OsString],
    cwd: Option<&Path>,
    cancel: &Cancel,
    timeout: Duration,
) -> Result<Output> {
    cancel.check()?;
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    command
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env(
            "GIT_SSH_COMMAND",
            "/usr/bin/ssh -o BatchMode=yes -o ConnectTimeout=15",
        );
    let mut child = command
        .spawn()
        .with_context(|| gw_i18n::tf!(UnableToStart, program.display()))?;
    let mut stdout = child
        .stdout
        .take()
        .context(gw_i18n::tr!(CannotAttachStdout))?;
    let mut stderr = child
        .stderr
        .take()
        .context(gw_i18n::tr!(CannotAttachStderr))?;
    let out = thread::spawn(move || {
        let mut data = Vec::new();
        stdout.read_to_end(&mut data).map(|_| data)
    });
    let err = thread::spawn(move || {
        let mut data = Vec::new();
        stderr.read_to_end(&mut data).map(|_| data)
    });
    let started = Instant::now();
    let result = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) => {}
            Err(e) => break Err(e.into()),
        }
        if cancel.is_cancelled() {
            break Err(crate::cancel::Cancelled.into());
        }
        if started.elapsed() >= timeout {
            break Err(anyhow::anyhow!(gw_i18n::tf!(
                TimedOutSeconds,
                program.display(),
                timeout.as_secs()
            )));
        }
        thread::sleep(Duration::from_millis(20));
    };
    // 连同 SSH、钩子和凭据子进程一起收尾，避免取消后继续产生副作用。
    unsafe {
        libc::kill(-(child.id() as i32), libc::SIGKILL);
    }
    let _ = child.wait();
    let stdout = out
        .join()
        .map_err(|_| anyhow::anyhow!(gw_i18n::tr!(StdoutReaderPanicked)))??;
    let stderr = err
        .join()
        .map_err(|_| anyhow::anyhow!(gw_i18n::tr!(StderrReaderPanicked)))??;
    let status = result?;
    Ok(Output {
        code: status.code().unwrap_or(128),
        stdout,
        stderr,
    })
}
pub fn system(program: &str, args: &[&str], cancel: &Cancel) -> Result<Output> {
    run(
        Path::new(program),
        &args.iter().map(OsString::from).collect::<Vec<_>>(),
        None,
        cancel,
        Duration::from_secs(30),
    )
}
