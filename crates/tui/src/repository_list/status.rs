//! Repository status prioritizes failures over actionable current-branch differences.
use engine::domain::{IssueKind, Publication, RepositoryResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Status {
    Unchecked,
    Normal,
    NeedsAttention,
    Failed,
}
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Counts {
    pub normal: usize,
    pub needs_attention: usize,
    pub failed: usize,
}
impl Status {
    pub fn of(result: Option<&RepositoryResult>) -> Self {
        let Some(result) = result else {
            return Self::Unchecked;
        };
        if matches!(result, RepositoryResult::Failed { .. })
            || matches!(result, RepositoryResult::Attention { issues, .. } if issues.iter().any(|issue| matches!(issue.kind, IssueKind::OperationFailed | IssueKind::PushFailed)))
            || result
                .pushes()
                .iter()
                .any(|publication| matches!(publication, Publication::Failed { .. }))
        {
            return Self::Failed;
        }
        if matches!(result, RepositoryResult::Attention { issues, .. } if issues.iter().any(|issue| issue.kind == IssueKind::Dirty))
            || result
                .synchronization()
                .current_commits()
                .is_some_and(|(ahead, behind)| ahead > 0 || behind > 0)
        {
            return Self::NeedsAttention;
        }
        Self::Normal
    }
    pub fn counts(states: impl IntoIterator<Item = Self>) -> Counts {
        let mut counts = Counts::default();
        for state in states {
            match state {
                Self::Normal => counts.normal += 1,
                Self::NeedsAttention => counts.needs_attention += 1,
                Self::Failed => counts.failed += 1,
                Self::Unchecked => {}
            }
        }
        counts
    }
    pub fn priority(self) -> u8 {
        match self {
            Self::Failed => 0,
            Self::NeedsAttention => 1,
            Self::Unchecked => 2,
            Self::Normal => 3,
        }
    }
    pub fn symbol(self) -> &'static str {
        match self {
            Self::Unchecked => "·",
            Self::Normal => "✓",
            Self::NeedsAttention => "!",
            Self::Failed => "×",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Unchecked => i18n::tr!(Unchecked),
            Self::Normal => i18n::tr!(Clean),
            Self::NeedsAttention => i18n::tr!(NeedsAttention),
            Self::Failed => i18n::tr!(Failed),
        }
    }
}
