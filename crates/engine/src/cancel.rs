//! Cooperative cancellation shared by workers and subprocesses.
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
#[derive(Debug, Clone, Default)]
pub struct Cancel(Arc<AtomicBool>);
impl Cancel {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release)
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
    pub fn check(&self) -> anyhow::Result<()> {
        if self.is_cancelled() {
            Err(Cancelled.into())
        } else {
            Ok(())
        }
    }
}
#[derive(Debug)]
pub struct Cancelled;
impl std::fmt::Display for Cancelled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(i18n::tr!(OperationCancelled))
    }
}
impl std::error::Error for Cancelled {}
