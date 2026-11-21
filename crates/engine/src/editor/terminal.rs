//! Run an interactive editor in the current terminal.
#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

#[cfg(unix)]
pub use unix::run;
#[cfg(windows)]
pub use windows::run;
