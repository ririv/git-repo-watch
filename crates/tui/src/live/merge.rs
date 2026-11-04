//! Snapshot projection keeps remote failures and captured command plans intact.
use crate::{
    Mode, history,
    mutations::readback::{self, Readback},
    stashes,
};
use engine::domain::{IssueKind, Issues, RepositoryResult};

pub(crate) fn result(
    local: RepositoryResult,
    previous: Option<&RepositoryResult>,
) -> RepositoryResult {
    let Some(previous) = previous else {
        return local;
    };
    if matches!(previous, RepositoryResult::Failed { .. }) {
        return previous.clone();
    }
    let retained = match previous {
        RepositoryResult::Attention { issues, .. } => issues
            .iter()
            .filter(|issue| {
                matches!(
                    issue.kind,
                    IssueKind::PushFailed | IssueKind::OperationFailed
                )
            })
            .cloned()
            .collect::<Vec<_>>(),
        _ => Vec::new(),
    };
    let (path, synchronization, branches_without_upstream, mut issues) = match local {
        RepositoryResult::Clean {
            path,
            synchronization,
            branches_without_upstream,
            ..
        } => (path, synchronization, branches_without_upstream, Vec::new()),
        RepositoryResult::Attention {
            path,
            synchronization,
            branches_without_upstream,
            issues,
            ..
        } => (
            path,
            synchronization,
            branches_without_upstream,
            issues.iter().cloned().collect(),
        ),
        RepositoryResult::Failed { path, error, .. } => {
            return RepositoryResult::Failed {
                path,
                error,
                pushes: previous.pushes().to_vec(),
            };
        }
    };
    issues.extend(retained);
    let pushes = previous.pushes().to_vec();
    let mut issues = issues.into_iter();
    match issues.next() {
        Some(first) => RepositoryResult::Attention {
            path,
            synchronization,
            branches_without_upstream,
            pushes,
            issues: Issues::new(first, issues.collect()),
        },
        None => RepositoryResult::Clean {
            path,
            synchronization,
            branches_without_upstream,
            pushes,
        },
    }
}
pub(crate) fn mode(mode: &mut Mode, readback: &Readback) {
    // 编辑器、确认页及其原始快照保持冻结，最终写入继续使用原有校验。
    if let (Readback::Stashes(snapshot), Mode::Stashes(view)) = (readback, &mut *mode)
        && view.session().snapshot.path == snapshot.path
    {
        let next = view.session().refreshed(snapshot.clone());
        match view.as_mut() {
            stashes::View::List(session)
            | stashes::View::Detail { session, .. }
            | stashes::View::Diff { session, .. } => *session = next,
        }
        return;
    }
    readback::merge_mode(mode, readback.clone());
}
pub(crate) fn history_mode(mode: &mut Mode, graph: &history::Session) {
    if let Mode::History(view) = mode
        && view.session().references.path == graph.references.path
    {
        match view.as_mut() {
            history::View::Graph(current) => *current = current.refreshed(graph.clone()),
            history::View::Commit(commit) => commit.graph = commit.graph.refreshed(graph.clone()),
            history::View::Diff { commit, .. } => {
                commit.graph = commit.graph.refreshed(graph.clone())
            }
        }
    }
}
