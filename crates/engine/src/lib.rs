//! Composable application services for Git repository monitoring.
pub mod cancel;
pub mod check;
pub mod desktop;
pub mod directories;
pub mod discovery;
pub mod domain;
pub mod executor;
pub mod local;
pub mod notify;
pub mod operations;
pub mod paths;
pub mod process;
pub mod scheduler;
pub mod service;
pub mod state;
pub mod terminal;
pub mod transport;

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
compile_error!(
    "This application currently supports macOS and Linux; a Windows platform backend is not implemented yet."
);
