//! Composable application services for Git repository monitoring.
pub mod ai;
pub mod cancel;
pub mod check;
pub mod commit_message;
pub mod composition;
pub mod desktop;
pub mod directories;
pub mod discovery;
pub mod domain;
pub mod editor;
pub mod executor;
pub mod git_environment;
pub mod local;
pub mod notify;
pub mod observation;
pub mod operations;
pub mod paths;
mod platform;
pub mod process;
pub mod scheduler;
pub mod service;
pub mod state;
pub mod tasks;
pub mod terminal;
pub mod transport;
pub mod workflow;

#[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
compile_error!("This application currently supports macOS, Linux, and Windows.");
