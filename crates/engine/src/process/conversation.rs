//! Cancellable duplex subprocesses with bounded stderr and explicit completion policies.
use super::*;
use std::io::BufReader;
use std::process::{ChildStdin, ChildStdout};

#[derive(Clone, Copy)]
pub(crate) enum Completion {
    Protocol,
    Process,
}

pub fn run<T: Send>(
    mut command: Command,
    cancel: &Cancel,
    timeout: Duration,
    completion: Completion,
    communicate: impl FnOnce(BufReader<ChildStdout>, ChildStdin) -> Result<T> + Send,
) -> Result<T> {
    cancel.check()?;
    let program = command.get_program().to_string_lossy().into_owned();
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut tree = Tree::spawn(&mut command).with_context(|| i18n::tf!(UnableToStart, program))?;
    let child = &mut tree.child;
    let stdout = child.stdout.take().context(i18n::tr!(CannotAttachStdout))?;
    let stdin = child.stdin.take().context(i18n::tr!(CannotAttachStdout))?;
    let mut stderr = child.stderr.take().context(i18n::tr!(CannotAttachStderr))?;
    let overflow = std::sync::atomic::AtomicBool::new(false);
    thread::scope(|scope| {
        let errors = scope.spawn(|| collect(&mut stderr, 64 * 1024, &overflow));
        let protocol = scope.spawn(move || communicate(BufReader::new(stdout), stdin));
        let started = Instant::now();
        let check = || -> Result<()> {
            cancel.check()?;
            anyhow::ensure!(
                !overflow.load(std::sync::atomic::Ordering::Acquire),
                i18n::tr!(AgentOutputTooLarge)
            );
            anyhow::ensure!(
                started.elapsed() < timeout,
                i18n::tf!(TimedOutSeconds, program, timeout.as_secs())
            );
            Ok(())
        };
        let result = (|| {
            while !protocol.is_finished() {
                check()?;
                thread::sleep(Duration::from_millis(20));
            }
            check()?;
            let value = protocol
                .join()
                .map_err(|_| anyhow::anyhow!(i18n::tr!(StdoutReaderPanicked)))??;
            if matches!(completion, Completion::Process) {
                loop {
                    check()?;
                    if let Some(status) = tree.child.try_wait()? {
                        anyhow::ensure!(
                            status.success(),
                            "{}: exit {}",
                            program,
                            status.code().unwrap_or(128)
                        );
                        break;
                    }
                    thread::sleep(Duration::from_millis(20));
                }
            }
            Ok(value)
        })();
        // 成功、失败和取消均收尾整个进程组，App Server 只在本次生成期间运行。
        tree.finish();
        let stderr = errors
            .join()
            .map_err(|_| anyhow::anyhow!(i18n::tr!(StderrReaderPanicked)))??;
        result.with_context(|| format!("{}: {}", program, String::from_utf8_lossy(&stderr)))
    })
}
