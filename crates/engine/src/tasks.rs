//! Persistent task lifecycles shared by interactive and scheduled execution.
use crate::{
    cancel::Cancel,
    state::{self, Lock, Store},
};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
    time::Duration,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Scope {
    All,
    Repository { path: PathBuf },
    Application,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    Interactive,
    Scheduled,
    Imported,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Retry {
    Unavailable,
    CheckAll,
    CheckRepository { path: PathBuf },
    Fetch { path: PathBuf },
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Spec {
    pub title: String,
    pub scope: Scope,
    pub source: Source,
    pub retry: Retry,
    pub retry_of: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Resolution {
    Pending,
    Handled { at: String },
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Outcome {
    Succeeded {
        detail: String,
    },
    Failed {
        detail: String,
        resolution: Resolution,
    },
    Cancelled {
        detail: String,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum State {
    Running { progress: String },
    Cancelling { progress: String },
    Finished { at: String, outcome: Outcome },
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Record {
    pub id: String,
    pub spec: Spec,
    pub started_at: String,
    pub state: State,
}
impl Record {
    pub fn priority(&self) -> u8 {
        match &self.state {
            State::Finished {
                outcome:
                    Outcome::Failed {
                        resolution: Resolution::Pending,
                        ..
                    },
                ..
            } => 0,
            State::Running { .. } | State::Cancelling { .. } => 1,
            State::Finished { .. } => 2,
        }
    }
    pub fn active(&self) -> bool {
        matches!(self.state, State::Running { .. } | State::Cancelling { .. })
    }
    pub fn pending(&self) -> bool {
        self.priority() == 0
    }
}
impl Outcome {
    pub fn capture<T>(
        result: &Result<T>,
        cancel: &Cancel,
        detail: impl FnOnce(&T) -> String,
    ) -> Self {
        match result {
            Ok(value) => Self::Succeeded {
                detail: detail(value),
            },
            Err(error) if cancel.is_cancelled() => Self::Cancelled {
                detail: format!("{error:#}"),
            },
            Err(error) => Self::Failed {
                detail: format!("{error:#}"),
                resolution: Resolution::Pending,
            },
        }
    }
}
/// Owns a live task lock; a released lock makes interrupted execution detectable.
pub struct Run {
    store: Store,
    pub record: Record,
    _owner: Lock,
    monitor: Monitor,
}
struct Monitor {
    stop: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<Result<()>>>,
}
impl Monitor {
    fn stop(&mut self) -> Result<()> {
        self.stop.store(true, Ordering::Release);
        if let Some(handle) = self.handle.take() {
            handle.thread().unpark();
            handle
                .join()
                .map_err(|_| anyhow::anyhow!("task cancellation monitor panicked"))??;
        }
        Ok(())
    }
}
impl Drop for Monitor {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(handle) = self.handle.take() {
            handle.thread().unpark();
            let _ = handle.join();
        }
    }
}
impl Run {
    pub fn begin(store: &Store, spec: Spec, cancel: &Cancel) -> Result<Self> {
        static SERIAL: AtomicU64 = AtomicU64::new(0);
        let seed = format!(
            "{}:{}:{}",
            chrono::Utc::now()
                .timestamp_nanos_opt()
                .expect("current timestamp"),
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        );
        let id = format!("{:x}", Sha256::digest(seed));
        store.prepare_state()?;
        let owner = Lock::acquire(&store.task_lock_path(&id))?;
        let record = Record {
            id: id.clone(),
            spec,
            started_at: state::now(),
            state: State::Running {
                progress: String::new(),
            },
        };
        store.insert_task(&record)?;
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = stop.clone();
        let store_copy = store.clone();
        let cancel = cancel.clone();
        let handle = thread::spawn(move || {
            while !worker_stop.load(Ordering::Acquire) {
                if let Some(record) = store_copy.task(&id)?
                    && matches!(record.state, State::Cancelling { .. })
                {
                    cancel.cancel();
                }
                thread::park_timeout(Duration::from_millis(250));
            }
            Ok(())
        });
        Ok(Self {
            store: store.clone(),
            record,
            _owner: owner,
            monitor: Monitor {
                stop,
                handle: Some(handle),
            },
        })
    }
    pub fn progress(&mut self, progress: String) -> Result<()> {
        self.store.task_progress(&self.record.id, &progress)
    }
    pub fn finish(mut self, outcome: Outcome) -> Result<()> {
        let stopped = self.monitor.stop().context("task cancellation monitor");
        let outcome = match &stopped {
            Ok(()) => outcome,
            Err(error) => Outcome::Failed {
                detail: format!("{error:#}"),
                resolution: Resolution::Pending,
            },
        };
        self.store.finish_task(&self.record.id, outcome)?;
        std::fs::remove_file(self.store.task_lock_path(&self.record.id))?;
        stopped
    }
}
