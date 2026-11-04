//! Transfer terminal foreground ownership to an interactive editor and restore it on return.
use super::Plan;
use anyhow::{Context, Result, ensure};
use std::{
    os::unix::process::CommandExt,
    os::unix::process::ExitStatusExt,
    process::{Command, Stdio},
};
struct Foreground {
    fd: i32,
    previous: libc::pid_t,
}
fn foreground(fd: i32, group: libc::pid_t) -> Result<()> {
    // 父进程处于后台时恢复前台进程组，需要暂时屏蔽 SIGTTOU。
    unsafe {
        let mut set = std::mem::zeroed();
        let mut previous = std::mem::zeroed();
        libc::sigemptyset(&mut set);
        libc::sigaddset(&mut set, libc::SIGTTOU);
        let changed = libc::pthread_sigmask(libc::SIG_BLOCK, &set, &mut previous);
        ensure!(
            changed == 0,
            "{}",
            std::io::Error::from_raw_os_error(changed)
        );
        let result = libc::tcsetpgrp(fd, group);
        let error = std::io::Error::last_os_error();
        let restored = libc::pthread_sigmask(libc::SIG_SETMASK, &previous, std::ptr::null_mut());
        ensure!(
            restored == 0,
            "{}",
            std::io::Error::from_raw_os_error(restored)
        );
        ensure!(result == 0, "{error}");
    }
    Ok(())
}
impl Drop for Foreground {
    fn drop(&mut self) {
        let _ = foreground(self.fd, self.previous);
    }
}
pub fn run(plan: &Plan) -> Result<()> {
    let fd = libc::STDIN_FILENO;
    let previous = unsafe { libc::tcgetpgrp(fd) };
    ensure!(previous >= 0, "{}", std::io::Error::last_os_error());
    let mut child = Command::new(&plan.program)
        .args(&plan.arguments)
        .current_dir(&plan.directory)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .process_group(0)
        .spawn()
        .with_context(|| i18n::tf!(UnableToStart, plan.program.display()))?;
    let guard = Foreground { fd, previous };
    if let Err(error) = foreground(fd, child.id() as libc::pid_t) {
        let _ = child.kill();
        let _ = child.wait();
        return Err(error);
    }
    // 编辑器可能在父进程交出前台之前读入终端并收到 SIGTTIN；恢复它继续运行。
    unsafe {
        libc::kill(-(child.id() as libc::pid_t), libc::SIGCONT);
    }
    let status = loop {
        let mut status = 0;
        let waited =
            unsafe { libc::waitpid(child.id() as libc::pid_t, &mut status, libc::WUNTRACED) };
        if waited < 0 {
            let error = std::io::Error::last_os_error();
            if error.kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            return Err(error.into());
        }
        if libc::WIFSTOPPED(status) {
            foreground(fd, previous)?;
            // 将终端编辑器的暂停传回原 shell 管理的 gw 作业；fg 后继续编辑器。
            unsafe {
                libc::kill(-previous, libc::SIGTSTP);
            }
            foreground(fd, child.id() as libc::pid_t)?;
            unsafe {
                libc::kill(-(child.id() as libc::pid_t), libc::SIGCONT);
            }
            continue;
        }
        break std::process::ExitStatus::from_raw(status);
    };
    foreground(fd, previous)?;
    std::mem::forget(guard);
    ensure!(
        status.success(),
        "{}: {status}",
        i18n::tr!(ExternalEditorFailed)
    );
    Ok(())
}
