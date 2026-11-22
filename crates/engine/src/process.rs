//! Cancellable subprocess execution with process-group cleanup.
pub(crate) mod conversation;
mod tree;
use crate::cancel::Cancel;
use anyhow::{Context, Result, bail};
use std::{
    ffi::OsString,
    fmt,
    io::Read,
    path::Path,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};
use tree::Tree;
#[derive(Debug)]
pub struct Output {
    pub code: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}
#[derive(Debug)]
pub struct Failure {
    label: String,
    code: i32,
    stderr: String,
    stdout: String,
}

fn diagnostic<'a, I: Iterator<Item = &'a str>>(lines: impl Fn() -> I) -> Option<&'a str> {
    let candidates = || lines().map(str::trim).filter(|line| !line.is_empty());
    for marker in ["❌", "fatal: ", "error: "] {
        if let Some(reason) = candidates().find_map(|line| {
            line.strip_prefix(marker)
                .map(str::trim)
                .filter(|reason| !reason.is_empty())
        }) {
            return Some(reason);
        }
    }
    candidates().next()
}

impl Failure {
    pub(crate) fn summary(&self) -> Option<&str> {
        diagnostic(|| self.stderr.lines().chain(self.stdout.lines()))
    }

    /// Recognize the persisted Git process error, including an outer operation or phase label.
    pub(crate) fn git_summary(detail: &str) -> Option<&str> {
        for (opening, closing) in [("git failed (", "):\n"), ("git 失败（", "）：\n")] {
            if let Some((code, output)) = detail
                .split_once(opening)
                .and_then(|(_, rest)| rest.split_once(closing))
                && code.parse::<i32>().is_ok()
            {
                return diagnostic(|| output.lines());
            }
        }
        None
    }
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&i18n::tf!(
            LabelFailed,
            self.code,
            self.stderr,
            self.stdout,
            label = self.label
        ))
    }
}

impl std::error::Error for Failure {}

impl Output {
    pub fn success(self, label: &str) -> Result<Vec<u8>> {
        if self.code == 0 {
            Ok(self.stdout)
        } else {
            Err(Failure {
                label: label.to_owned(),
                code: self.code,
                stderr: String::from_utf8_lossy(&self.stderr).into_owned(),
                stdout: String::from_utf8_lossy(&self.stdout).into_owned(),
            }
            .into())
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
    run_with_environment(program, args, cwd, cancel, timeout, &[])
}
pub(crate) fn run_with_environment(
    program: &Path,
    args: &[OsString],
    cwd: Option<&Path>,
    cancel: &Cancel,
    timeout: Duration,
    environment: &[(OsString, OsString)],
) -> Result<Output> {
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    command.envs(environment.iter().map(|(key, value)| (key, value)));
    command
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env(
            "GIT_SSH_COMMAND",
            if cfg!(windows) {
                "ssh -o BatchMode=yes -o ConnectTimeout=15"
            } else {
                "/usr/bin/ssh -o BatchMode=yes -o ConnectTimeout=15"
            },
        );
    execute(command, cancel, timeout, usize::MAX)
}

fn execute(
    mut command: Command,
    cancel: &Cancel,
    timeout: Duration,
    limit: usize,
) -> Result<Output> {
    cancel.check()?;
    let program = command.get_program().to_string_lossy().into_owned();
    let mut tree = Tree::spawn(&mut command).with_context(|| i18n::tf!(UnableToStart, program))?;
    let child = &mut tree.child;
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
    tree.finish();
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
