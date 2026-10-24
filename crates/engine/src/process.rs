//! Cancellable subprocess execution with process-group cleanup.
use crate::cancel::Cancel;
use anyhow::{Context, Result, bail};
use std::{
    ffi::OsString,
    io::{Read, Seek, Write},
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
            bail!(i18n::tf!(
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
    execute(command, cancel, timeout, usize::MAX)
}
/// Run a bounded non-interactive request with private file-backed stdin.
pub fn run_with_input(
    program: &Path,
    args: &[OsString],
    cwd: &Path,
    input: &[u8],
    cancel: &Cancel,
    timeout: Duration,
    limit: usize,
) -> Result<Output> {
    cancel.check()?;
    let mut stdin = tempfile::tempfile()?;
    stdin.write_all(input)?;
    stdin.rewind()?;
    let mut command = Command::new(program);
    command
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::from(stdin))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0);
    execute(command, cancel, timeout, limit)
}
fn execute(
    mut command: Command,
    cancel: &Cancel,
    timeout: Duration,
    limit: usize,
) -> Result<Output> {
    cancel.check()?;
    let program = command.get_program().to_string_lossy().into_owned();
    let mut child = command
        .spawn()
        .with_context(|| i18n::tf!(UnableToStart, program))?;
    let mut stdout = child.stdout.take().context(i18n::tr!(CannotAttachStdout))?;
    let mut stderr = child.stderr.take().context(i18n::tr!(CannotAttachStderr))?;
    let overflow = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let output_overflow = overflow.clone();
    let error_overflow = overflow.clone();
    let out = thread::spawn(move || collect(&mut stdout, limit, &output_overflow));
    let err = thread::spawn(move || collect(&mut stderr, limit, &error_overflow));
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
        if overflow.load(std::sync::atomic::Ordering::Acquire) {
            break Err(anyhow::anyhow!(i18n::tr!(AgentOutputTooLarge)));
        }
        if started.elapsed() >= timeout {
            break Err(anyhow::anyhow!(i18n::tf!(
                TimedOutSeconds,
                program,
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
        .map_err(|_| anyhow::anyhow!(i18n::tr!(StdoutReaderPanicked)))??;
    let stderr = err
        .join()
        .map_err(|_| anyhow::anyhow!(i18n::tr!(StderrReaderPanicked)))??;
    let status = result?;
    if overflow.load(std::sync::atomic::Ordering::Acquire) {
        bail!(i18n::tr!(AgentOutputTooLarge));
    }
    Ok(Output {
        code: status.code().unwrap_or(128),
        stdout,
        stderr,
    })
}
fn collect(
    reader: &mut impl Read,
    limit: usize,
    overflow: &std::sync::atomic::AtomicBool,
) -> std::io::Result<Vec<u8>> {
    let mut data = Vec::new();
    let mut buffer = [0; 8192];
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            return Ok(data);
        }
        if count > limit.saturating_sub(data.len()) {
            overflow.store(true, std::sync::atomic::Ordering::Release);
        } else {
            data.extend_from_slice(&buffer[..count]);
        }
    }
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
