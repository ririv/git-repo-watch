//! UI-independent orchestration, persistence, and bounded repository checks.
use crate::{
    cancel::Cancel,
    check, discovery,
    domain::*,
    executor,
    local::{
        self,
        files::{self, Change, Files, IndexAction},
        review::CommitPlan,
    },
    notify,
    operations::{self, commit::CommitResult, push::PushPlan},
    scheduler::{Manager, Registration, Schedule},
    state::{self, Lock, Store},
    transport::Git,
};
use anyhow::{Context, Result, ensure};
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};
#[derive(Debug, Clone)]
pub struct Service {
    pub store: Store,
    pub git: Git,
    pub scheduler: Manager,
}
#[derive(Debug, Clone)]
pub enum Event {
    Checking(PathBuf),
    Checked(RepositoryResult),
    NotificationProblem(Notice),
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Activity {
    Idle,
    Running,
}
#[derive(Debug, Clone)]
pub enum CachedReport {
    Missing,
    Available { report: Report, scope: Scope },
}
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    Current,
    Outdated,
}
#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    pub registration: Registration,
    pub activity: Activity,
    pub configured_interval_seconds: u64,
    pub push_policy: PushPolicy,
    pub repository_count: usize,
    pub excluded_roots: Vec<PathBuf>,
    pub schedule: Schedule,
    pub report: CachedReport,
    pub max_concurrency: usize,
}
mod checks;
mod snapshot;
pub use checks::{Groups, group};
mod config;
mod repository;
mod schedule;
impl Service {
    pub fn new(home: PathBuf, scheduler: Manager) -> Self {
        Self {
            store: Store { home },
            git: Git::default(),
            scheduler,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanMode {
    Preview,
    Import,
}
#[derive(Debug, Clone)]
pub enum ConfigEdit {
    Remove(PathBuf),
    Exclude(PathBuf),
    Unexclude(PathBuf),
    Interval(u64),
    Policy(PushPolicy),
    Concurrency(usize),
}
