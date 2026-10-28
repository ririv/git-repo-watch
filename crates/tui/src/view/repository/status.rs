//! Repository execution, workspace changes and branch comparisons render independently.
use super::*;
use crate::repository_list::Execution;
use engine::domain::synchronization::Synchronization;

pub(super) fn color(execution: Execution) -> Color {
    match execution {
        Execution::Unchecked => MUTED,
        Execution::Succeeded => GOOD,
        Execution::PartialFailure => WARN,
        Execution::Failed => BAD,
    }
}
pub(super) fn workspace(result: Option<&RepositoryResult>) -> (&'static str, Color) {
    match result {
        None => ("?", MUTED),
        Some(RepositoryResult::Failed { .. }) => ("?", color(Execution::of(result))),
        Some(RepositoryResult::Attention { issues, .. })
            if issues.iter().any(|issue| issue.kind == IssueKind::Dirty) =>
        {
            (i18n::tr!(Changes), ACCENT)
        }
        Some(_) => (i18n::tr!(CleanAlt), MUTED),
    }
}
pub(super) fn synchronization(result: Option<&RepositoryResult>) -> (String, Color) {
    if matches!(result, Some(RepositoryResult::Failed { .. })) {
        return ("?".into(), color(Execution::of(result)));
    }
    match result.map(RepositoryResult::synchronization) {
        None | Some(Synchronization::Unknown) => ("?".into(), MUTED),
        Some(sync @ Synchronization::Known { .. }) => match sync.counts() {
            Some((0, 0)) => ("↑0 ↓0".into(), GOOD),
            Some((ahead, behind)) => (format!("↑{ahead} ↓{behind}"), ACCENT),
            None => ("—".into(), MUTED),
        },
    }
}
