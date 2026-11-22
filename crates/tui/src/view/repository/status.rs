//! Repository execution, workspace changes and branch comparisons render independently.
use super::*;
use crate::repository_list::Status;
use crate::view::synchronization::commit_counts;
use engine::domain::synchronization::{Checkout, Synchronization};

pub(super) fn branch(result: Option<&RepositoryResult>) -> (&str, Color) {
    match result {
        None => ("?", MUTED),
        Some(RepositoryResult::Failed { .. }) => ("?", BAD),
        Some(result) => match result.synchronization() {
            Synchronization::Current { checkout, .. } => match checkout {
                Checkout::Compared { branch, .. }
                | Checkout::NoUpstream { branch }
                | Checkout::MissingUpstream { branch }
                | Checkout::Unborn { branch } => (branch, Color::Reset),
                Checkout::Detached => (i18n::tr!(OverviewDetached), MUTED),
            },
            Synchronization::Unknown | Synchronization::LegacyBranches { .. } => ("?", MUTED),
        },
    }
}

pub(super) fn color(execution: Status) -> Color {
    match execution {
        Status::Unchecked => MUTED,
        Status::Normal => GOOD,
        Status::NeedsAttention => WARN,
        Status::Failed => BAD,
    }
}
pub(super) fn workspace(result: Option<&RepositoryResult>) -> (&'static str, Color) {
    match result {
        None => ("?", MUTED),
        Some(RepositoryResult::Failed { .. }) => ("?", color(Status::of(result))),
        Some(RepositoryResult::Attention { issues, .. })
            if issues.iter().any(|issue| issue.kind == IssueKind::Dirty) =>
        {
            (i18n::tr!(Changes), WARN)
        }
        Some(_) => (i18n::tr!(CleanAlt), MUTED),
    }
}
pub(super) fn synchronization(result: Option<&RepositoryResult>) -> Line<'static> {
    let (value, color) = match result {
        Some(RepositoryResult::Failed { .. }) => ("?", BAD),
        None => ("?", MUTED),
        Some(result) => match result.synchronization() {
            Synchronization::Unknown | Synchronization::LegacyBranches { .. } => ("?", MUTED),
            sync @ Synchronization::Current { .. } => match sync.current_commits() {
                Some((ahead, behind)) => {
                    return Line::from(Vec::from(commit_counts(ahead, behind)));
                }
                None => ("—", MUTED),
            },
        },
    };
    Line::from(Span::styled(value, Style::default().fg(color)))
}
