//! Keep subprocess groups alive until all output is collected or the task is cancelled.
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::{
    io,
    process::{Child, Command},
};
#[cfg(windows)]
#[path = "tree/windows.rs"]
mod windows;

pub(super) struct Tree {
    pub(super) child: Child,
    #[cfg(windows)]
    job: windows::Job,
}

impl Tree {
    pub(super) fn spawn(command: &mut Command) -> io::Result<Self> {
        #[cfg(unix)]
        {
            command.process_group(0);
            Ok(Self {
                child: command.spawn()?,
            })
        }
        #[cfg(windows)]
        {
            let (child, job) = windows::Job::spawn(command)?;
            Ok(Self { child, job })
        }
    }

    pub(super) fn finish(&mut self) {
        #[cfg(unix)]
        unsafe {
            libc::kill(-(self.child.id() as i32), libc::SIGKILL);
        }
        #[cfg(windows)]
        self.job.terminate();
        let _ = self.child.wait();
    }
}
