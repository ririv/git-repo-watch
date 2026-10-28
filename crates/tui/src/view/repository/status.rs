//! Repository execution, workspace changes and branch comparisons render independently.
use super::*;
use crate::repository_list::Status;
use engine::domain::synchronization::{Checkout, Synchronization};

pub(super) fn branch(result: Option<&RepositoryResult>) -> (&str, Color) {
    match result {
        None => ("?", MUTED),
        Some(RepositoryResult::Failed { .. }) => ("?", BAD),
        Some(result) => match result.synchronization() {
            Synchronization::Current { checkout } => match checkout {
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
pub(super) fn synchronization(result: Option<&RepositoryResult>) -> (String, Color) {
    if matches!(result, Some(RepositoryResult::Failed { .. })) {
        return ("?".into(), color(Status::of(result)));
    }
    match result.map(RepositoryResult::synchronization) {
        None | Some(Synchronization::Unknown | Synchronization::LegacyBranches { .. }) => {
            ("?".into(), MUTED)
        }
        Some(sync @ Synchronization::Current { .. }) => match sync.current_commits() {
            Some((0, 0)) => ("↑0 ↓0".into(), GOOD),
            Some((ahead, behind)) => (format!("↑{ahead} ↓{behind}"), WARN),
            None => ("—".into(), MUTED),
        },
    }
}
