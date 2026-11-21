//! Repository execution, workspace changes and branch comparisons render independently.
use super::*;
use crate::repository_list::Status;
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

pub(super) fn branch_synchronization(
    result: &RepositoryResult,
    without: &[engine::domain::BranchWithoutUpstream],
    current: Option<&str>,
) -> Vec<Line<'static>> {
    use engine::domain::{UpstreamReason, synchronization::RemoteComparison};

    let row = |value, color| Line::styled(format!("  {value}"), Style::default().fg(color));
    let mut lines = vec![super::field(i18n::tr!(SyncColumn), "", Color::Reset)];
    let mut current_row = None;
    if let Some(branches) = result.synchronization().remote_branches() {
        for branch in branches {
            if current == Some(branch.name.as_str()) {
                current_row = Some(lines.len());
            }
            let (comparison, color) = match branch.comparison {
                RemoteComparison::Compared { ahead, behind } => (
                    format!("↑{ahead} ↓{behind}"),
                    if ahead == 0 && behind == 0 {
                        GOOD
                    } else {
                        WARN
                    },
                ),
                RemoteComparison::MissingUpstream => (
                    i18n::tr!(RemoteUpstreamBranchDoesNotExistAlt).to_owned(),
                    MUTED,
                ),
            };
            lines.push(row(format!("{} · {comparison}", safe(&branch.name)), color));
        }
        if branches.is_empty() && without.is_empty() {
            lines.push(row(
                i18n::tr!(NoLocalBranchCommitsToCheck).to_owned(),
                MUTED,
            ));
        }
    } else {
        if let RepositoryResult::Attention { issues, .. } = result {
            for issue in issues {
                let color = match issue.kind {
                    IssueKind::OutOfSync => WARN,
                    IssueKind::MissingUpstream => MUTED,
                    _ => continue,
                };
                lines.push(row(safe(&issue.detail), color));
            }
        }
        lines.push(row(i18n::tr!(SyncPartialReport).to_owned(), MUTED));
    }
    for branch in without {
        if current == Some(branch.branch.as_str()) {
            current_row = Some(lines.len());
        }
        let label = match branch.reason {
            UpstreamReason::NotConfigured => i18n::tr!(NoRemoteUpstream),
            UpstreamReason::LocalUpstream => i18n::tr!(TracksALocalBranch),
        };
        lines.push(row(format!("{} · {label}", safe(&branch.branch)), MUTED));
    }
    if let Some(index) = current_row {
        lines[1..=index].rotate_right(1);
    }
    lines
}
