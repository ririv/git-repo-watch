//! Shared operating-system primitives used by storage and worktree operations.
pub(crate) mod files;
#[cfg(windows)]
pub(crate) mod windows;
