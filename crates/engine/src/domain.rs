//! Domain outcomes and backward-compatible JSON records.
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PushPolicy {
    Disabled,
    FastForward,
}
impl std::str::FromStr for PushPolicy {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> anyhow::Result<Self> {
        match s {
            "disabled" => Ok(Self::Disabled),
            "fast_forward" => Ok(Self::FastForward),
            _ => anyhow::bail!(i18n::tf!(UnknownAutomaticPushPolicyS, s = s)),
        }
    }
}
impl std::fmt::Display for PushPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Disabled => "disabled",
            Self::FastForward => "fast_forward",
        })
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueKind {
    Dirty,
    DetachedHead,
    NoCommits,
    MissingUpstream,
    OutOfSync,
    PushFailed,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub kind: IssueKind,
    pub detail: String,
}
/// A non-empty issue sequence serialized as the existing JSON array.
#[derive(Debug, Clone)]
pub struct Issues {
    first: Issue,
    rest: Vec<Issue>,
}
impl Issues {
    pub fn new(first: Issue, rest: Vec<Issue>) -> Self {
        Self { first, rest }
    }
    pub fn iter(&self) -> impl Iterator<Item = &Issue> {
        std::iter::once(&self.first).chain(self.rest.iter())
    }
}
impl<'a> IntoIterator for &'a Issues {
    type Item = &'a Issue;
    type IntoIter = std::iter::Chain<std::iter::Once<&'a Issue>, std::slice::Iter<'a, Issue>>;
    fn into_iter(self) -> Self::IntoIter {
        std::iter::once(&self.first).chain(self.rest.iter())
    }
}
impl Serialize for Issues {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(self.iter())
    }
}
impl<'de> Deserialize<'de> for Issues {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut values = Vec::<Issue>::deserialize(deserializer)?.into_iter();
        let first = values.next().ok_or_else(|| {
            serde::de::Error::custom(i18n::tr!(AnAttentionResultRequiresAtLeastOne))
        })?;
        Ok(Self::new(first, values.collect()))
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UpstreamReason {
    NotConfigured,
    LocalUpstream,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchWithoutUpstream {
    pub branch: String,
    pub reason: UpstreamReason,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushTarget {
    pub branch: String,
    pub local_ref: String,
    pub local_oid: String,
    pub remote: String,
    pub remote_ref: String,
    pub remote_oid: String,
    pub ahead_commits: usize,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Validate,
    Transfer,
    Verify,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Publication {
    Pushed {
        target: PushTarget,
    },
    Failed {
        target: PushTarget,
        phase: Phase,
        error: String,
    },
}
impl Publication {
    pub fn target(&self) -> &PushTarget {
        match self {
            Self::Pushed { target } | Self::Failed { target, .. } => target,
        }
    }
    pub fn error(&self) -> Option<&str> {
        match self {
            Self::Failed { error, .. } => Some(error),
            Self::Pushed { .. } => None,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum RepositoryResult {
    Clean {
        path: PathBuf,
        branches_without_upstream: Vec<BranchWithoutUpstream>,
        pushes: Vec<Publication>,
    },
    Attention {
        path: PathBuf,
        issues: Issues,
        branches_without_upstream: Vec<BranchWithoutUpstream>,
        pushes: Vec<Publication>,
    },
    Failed {
        path: PathBuf,
        error: String,
        pushes: Vec<Publication>,
    },
}
impl RepositoryResult {
    pub fn path(&self) -> &std::path::Path {
        match self {
            Self::Clean { path, .. } | Self::Attention { path, .. } | Self::Failed { path, .. } => {
                path
            }
        }
    }
    pub fn pushes(&self) -> &[Publication] {
        match self {
            Self::Clean { pushes, .. }
            | Self::Attention { pushes, .. }
            | Self::Failed { pushes, .. } => pushes,
        }
    }
    pub fn branches(&self) -> Option<&[BranchWithoutUpstream]> {
        match self {
            Self::Clean {
                branches_without_upstream,
                ..
            }
            | Self::Attention {
                branches_without_upstream,
                ..
            } => Some(branches_without_upstream),
            Self::Failed { .. } => None,
        }
    }
    pub fn priority(&self) -> u8 {
        match self {
            Self::Failed { .. } => 0,
            Self::Attention { .. } => 1,
            Self::Clean { .. } => 3,
        }
    }
    pub fn label(&self) -> &str {
        match self {
            Self::Failed { .. } => i18n::tr!(CheckFailed),
            Self::Attention { .. } => i18n::tr!(NeedsAttention),
            Self::Clean { .. } => i18n::tr!(Clean),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Notice {
    Accepted {
        path: PathBuf,
        branch: String,
    },
    Failed {
        path: PathBuf,
        branch: String,
        error: String,
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportStatus {
    Clean,
    Attention,
    Unconfigured,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub schema_version: u32,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub repository_finished_at: std::collections::BTreeMap<PathBuf, String>,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub repository_revisions: std::collections::BTreeMap<PathBuf, u64>,
    pub push_policy: PushPolicy,
    pub notifications: Vec<Notice>,
    pub started_at: String,
    pub finished_at: String,
    pub status: ReportStatus,
    pub repositories: Vec<RepositoryResult>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub classification_updated_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution: Option<Execution>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Execution {
    pub max_concurrency: usize,
    pub repositories: Vec<Timing>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timing {
    pub path: PathBuf,
    pub elapsed_ms: u128,
    pub fetch_ms: u128,
    pub local_ms: u128,
    pub verify_ms: u128,
    pub push_ms: u128,
}
impl Report {
    pub fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.schema_version == 3,
            i18n::tf!(UnsupportedReportVersion, self.schema_version)
        );
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    All,
    Problems,
    WithoutUpstream,
    Pushes,
}
impl Filter {
    pub fn matches(self, r: &RepositoryResult) -> bool {
        match self {
            Self::All => true,
            Self::Problems => !matches!(r, RepositoryResult::Clean { .. }),
            Self::WithoutUpstream => r.branches().is_some_and(|b| !b.is_empty()),
            Self::Pushes => !r.pushes().is_empty(),
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::All => i18n::tr!(All),
            Self::Problems => i18n::tr!(Problems),
            Self::WithoutUpstream => i18n::tr!(NoUpstream),
            Self::Pushes => i18n::tr!(Pushes),
        }
    }
}
#[derive(Debug, Default)]
pub struct Stats {
    pub clean: usize,
    pub attention: usize,
    pub failed: usize,
    pub no_upstream: usize,
    pub no_upstream_repos: usize,
    pub pushed: usize,
    pub push_failed: usize,
    pub notice_failed: usize,
}
impl Stats {
    pub fn of(r: &Report) -> Self {
        let mut s = Self::repositories(&r.repositories);
        s.notice_failed = r
            .notifications
            .iter()
            .filter(|n| matches!(n, Notice::Failed { .. }))
            .count();
        s
    }
    pub fn repositories<'a>(repositories: impl IntoIterator<Item = &'a RepositoryResult>) -> Self {
        let mut s = Self::default();
        for repo in repositories {
            match repo {
                RepositoryResult::Clean { .. } => s.clean += 1,
                RepositoryResult::Attention { .. } => s.attention += 1,
                RepositoryResult::Failed { .. } => s.failed += 1,
            }
            if let Some(b) = repo.branches() {
                s.no_upstream += b.len();
                if !b.is_empty() {
                    s.no_upstream_repos += 1
                }
            }
            for p in repo.pushes() {
                match p {
                    Publication::Pushed { .. } => s.pushed += 1,
                    Publication::Failed { .. } => s.push_failed += 1,
                }
            }
        }
        s
    }
}
