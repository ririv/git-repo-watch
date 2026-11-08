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
use anyhow::{Result, ensure};
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
    session_repository: Option<PathBuf>,
}
#[derive(Debug, Clone)]
pub enum Event {
    Checking(PathBuf),
    Checked {
        result: RepositoryResult,
        finished_at: String,
        revision: u64,
    },
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
    Available { report: Box<Report>, scope: Scope },
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
    pub configured_anchor: crate::scheduler::timing::Anchor,
    pub next_trigger: crate::scheduler::NextTrigger,
    pub push_policy: PushPolicy,
    pub scheduled_actions: crate::workflow::Plan,
    pub repository_count: usize,
    pub schedule: Schedule,
    pub report: CachedReport,
    pub max_concurrency: usize,
    pub fetch_on_startup: bool,
}
mod checks;
mod composition;
mod snapshot;
pub use checks::{Groups, group};
mod config;
mod mutations;
mod repository;
mod schedule;
impl Service {
    pub fn new(store: Store, scheduler: Manager) -> Self {
        Self {
            store,
            git: Git::default(),
            scheduler,
            session_repository: None,
        }
    }
    /// Authorize one worktree for this session without registering it for batch workflows.
    pub fn with_session_repository(mut self, path: PathBuf, cancel: &Cancel) -> Result<Self> {
        cancel.check()?;
        let path = std::fs::canonicalize(path)?;
        local::open(&path)?;
        self.session_repository = Some(path);
        Ok(self)
    }
    pub fn session_repository(&self) -> Option<&Path> {
        self.session_repository.as_deref()
    }
}
#[derive(Debug, Clone)]
pub enum ConfigEdit {
    Remove(PathBuf),
    Interval(u64),
    Anchor(crate::scheduler::timing::Anchor),
    Policy(PushPolicy),
    Concurrency(usize),
    FetchOnStartup(bool),
    AutoFetchEnabled(bool),
    AutoFetchInterval(u64),
    ScheduledWorkflow(crate::workflow::Plan),
    TextWrap(bool),
    FileIcons(bool),
    ExperimentalFeatures(bool),
    ChineseCommandTrigger(bool),
    Mouse(state::MouseSetting, bool),
    Interaction(state::InteractionMode),
    View(state::view::Edit),
    Editor(crate::editor::Preference),
    AiEnabled(bool),
    AiAgent(crate::ai::Agent),
    AiModel {
        agent: crate::ai::Agent,
        model: String,
    },
    AiPrompt(crate::ai::Prompt),
}
