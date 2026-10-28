//! Execution status is independent of ordinary workspace and upstream differences.
use engine::domain::{IssueKind, Publication, RepositoryResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Execution {
    Unchecked,
    Succeeded,
    PartialFailure,
    Failed,
}
impl Execution {
    pub fn of(result: Option<&RepositoryResult>) -> Self {
        let Some(result) = result else {
            return Self::Unchecked;
        };
        if matches!(result, RepositoryResult::Failed { .. }) {
            return if result
                .pushes()
                .iter()
                .any(|item| matches!(item, Publication::Pushed { .. }))
                && result
                    .pushes()
                    .iter()
                    .any(|item| matches!(item, Publication::Failed { .. }))
            {
                Self::PartialFailure
            } else {
                Self::Failed
            };
        }
        if matches!(result, RepositoryResult::Attention { issues, .. } if issues.iter().any(|issue| issue.kind == IssueKind::OperationFailed))
        {
            return Self::Failed;
        }
        let failed = result
            .pushes()
            .iter()
            .any(|publication| matches!(publication, Publication::Failed { .. }))
            || matches!(result, RepositoryResult::Attention { issues, .. } if issues.iter().any(|issue| issue.kind == IssueKind::PushFailed));
        if !failed {
            Self::Succeeded
        } else if result
            .pushes()
            .iter()
            .any(|publication| matches!(publication, Publication::Pushed { .. }))
        {
            Self::PartialFailure
        } else {
            Self::Failed
        }
    }
    pub fn counts(states: impl IntoIterator<Item = Self>) -> engine::domain::Stats {
        let mut counts = engine::domain::Stats::default();
        for state in states {
            match state {
                Self::Succeeded => counts.clean += 1,
                Self::PartialFailure => counts.attention += 1,
                Self::Failed => counts.failed += 1,
                Self::Unchecked => {}
            }
        }
        counts
    }
    pub fn priority(self) -> u8 {
        match self {
            Self::Failed => 0,
            Self::PartialFailure => 1,
            Self::Unchecked => 2,
            Self::Succeeded => 3,
        }
    }
    pub fn symbol(self) -> &'static str {
        match self {
            Self::Unchecked => "·",
            Self::Succeeded => "✓",
            Self::PartialFailure => "!",
            Self::Failed => "×",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Unchecked => i18n::tr!(Unchecked),
            Self::Succeeded => i18n::tr!(Clean),
            Self::PartialFailure => i18n::tr!(RepositoryPartialFailure),
            Self::Failed => i18n::tr!(Failed),
        }
    }
}
