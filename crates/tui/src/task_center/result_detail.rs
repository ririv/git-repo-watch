//! Concise, localized results for completed repository operations.
use super::push_detail;
use crate::view::safe;
use engine::{
    local::files::conflict::resolve::Intent,
    operations::{
        commit::{CommitResult, Review},
        mutation::Change,
    },
};

fn item(label: &str, value: &str) -> String {
    format!("{label} · {}", safe(value))
}

fn short(oid: &str) -> &str {
    oid.get(..12).unwrap_or(oid)
}

pub(super) fn commit(result: &CommitResult) -> String {
    let mut detail = item(i18n::tr!(TaskCommitCreated), short(&result.oid));
    if result.review == Review::Changed {
        detail.push('\n');
        detail.push_str(
            i18n::tr!(ContentsChangedVerifyTheCommit).trim_start_matches(['；', ';', ' ']),
        );
    }
    detail
}

fn branch(label: &str, name: &str, oid: &str) -> String {
    item(label, &format!("{name} · {}", short(oid)))
}

fn stash(label: &str, oid: &str) -> String {
    item(label, short(oid))
}

fn conflict(label: &str, file: &str, intent: Option<Intent>) -> String {
    match intent {
        Some(Intent::Accept { choice }) => item(label, &format!("{} · {file}", choice.label())),
        _ => item(label, file),
    }
}

pub(super) fn mutation(change: &Change) -> String {
    match change {
        Change::CreatedBranch { name, oid } => branch(i18n::tr!(TaskBranchCreated), name, oid),
        Change::SwitchedBranch { name, oid } => branch(i18n::tr!(TaskBranchSwitched), name, oid),
        Change::Pulled { branch: name, oid } => branch(i18n::tr!(TaskBranchPulled), name, oid),
        Change::AddedWorktree { directory } => {
            item(i18n::tr!(TaskWorktreeCreated), &directory.to_string_lossy())
        }
        Change::CheckedOutSubmodules { mode, directories } => {
            let label = match mode {
                engine::operations::submodule::checkout::Mode::Initialize => {
                    i18n::tr!(TaskSubmodulesInitialized)
                }
                engine::operations::submodule::checkout::Mode::Update => {
                    i18n::tr!(TaskSubmodulesUpdated)
                }
            };
            let paths = directories
                .iter()
                .map(|path| safe(&path.to_string_lossy()))
                .collect::<Vec<_>>();
            format!("{label}\n{}", paths.join("\n"))
        }
        Change::AddedSubmodule { directory } => {
            item(i18n::tr!(TaskSubmoduleAdded), &directory.to_string_lossy())
        }
        Change::RemovedWorktree { directory } => {
            item(i18n::tr!(TaskWorktreeRemoved), &directory.to_string_lossy())
        }
        Change::Fetched { remotes } => item(i18n::tr!(TaskFetchedFrom), &remotes.join(", ")),
        Change::FetchSkipped { reason } => reason.label().into(),
        Change::SavedStash { oid } => stash(i18n::tr!(TaskStashSaved), oid),
        Change::AppliedStash { oid } => stash(i18n::tr!(TaskStashApplied), oid),
        Change::PoppedStash { oid } => stash(i18n::tr!(TaskStashPopped), oid),
        Change::DroppedStash { oid } => stash(i18n::tr!(TaskStashDropped), oid),
        Change::ResolvedConflict { file, intent } => {
            let label = match intent {
                Intent::Accept { .. } => i18n::tr!(TaskConflictAccepted),
                Intent::Undo => i18n::tr!(TaskConflictUndone),
                Intent::Reset => i18n::tr!(TaskConflictReset),
            };
            conflict(label, &file.to_string_lossy(), Some(*intent))
        }
        Change::SynchronizationPrepared => i18n::tr!(SyncCommand).into(),
        Change::Synchronized { result } => result.detail(),
        Change::Replayed { kind, commit } => {
            format!("{}\n{}: {}", kind.label(), i18n::tr!(ReplaySource), commit)
        }
        Change::ReplayedSequence { commits } => format!(
            "{}\n{}: {}",
            i18n::tr!(ReplayBatchTitle),
            i18n::tr!(ReplayOrder),
            commits.join("\n")
        ),
        Change::ReplayControlled { kind, control } => {
            format!("{} · {}", kind.label(), control.label())
        }
        Change::ResetBranch {
            mode,
            branch,
            from,
            to,
        } => format!("{} · {branch}\n{from} → {to}", mode.label()),
    }
}

pub(crate) fn from_legacy(detail: &str) -> Option<String> {
    if let Some(push) = push_detail::from_legacy(detail) {
        return Some(push);
    }
    let mut lines = detail.lines();
    let header = lines.next()?;
    if !matches!(lines.clone().last(), Some("}" | ")")) {
        return None;
    }
    let field = |name: &str| {
        lines.clone().find_map(|line| {
            let (key, value) = line.trim().split_once(": ")?;
            (key == name).then_some(value)?.strip_suffix(',')
        })
    };
    let text = |name| push_detail::debug_string(field(name)?);
    let hash = || text("oid");
    Some(match header {
        "CommitResult {" | "Some("
            if header == "CommitResult {" || lines.clone().next() == Some("    CommitResult {") =>
        {
            let review = match field("review")? {
                "Exact" => Review::Exact,
                "Changed" => Review::Changed,
                _ => return None,
            };
            commit(&CommitResult {
                oid: hash()?,
                review,
            })
        }
        "CreatedBranch {" => branch(i18n::tr!(TaskBranchCreated), &text("name")?, &hash()?),
        "SwitchedBranch {" => branch(i18n::tr!(TaskBranchSwitched), &text("name")?, &hash()?),
        "Pulled {" => branch(i18n::tr!(TaskBranchPulled), &text("branch")?, &hash()?),
        "AddedWorktree {" => item(i18n::tr!(TaskWorktreeCreated), &text("directory")?),
        "RemovedWorktree {" => item(i18n::tr!(TaskWorktreeRemoved), &text("directory")?),
        "SavedStash {" => stash(i18n::tr!(TaskStashSaved), &hash()?),
        "AppliedStash {" => stash(i18n::tr!(TaskStashApplied), &hash()?),
        "PoppedStash {" => stash(i18n::tr!(TaskStashPopped), &hash()?),
        "DroppedStash {" => stash(i18n::tr!(TaskStashDropped), &hash()?),
        "ResolvedConflict {" => conflict(i18n::tr!(ConflictControlsTitle), &text("file")?, None),
        "Fetched {" => {
            let mut names = lines.clone().skip_while(|line| line.trim() != "remotes: [");
            names.next()?;
            let names = names
                .take_while(|line| line.trim() != "],")
                .map(|line| push_detail::debug_string(line.trim().strip_suffix(',')?))
                .collect::<Option<Vec<_>>>()?;
            item(i18n::tr!(TaskFetchedFrom), &names.join(", "))
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_saved_stash_and_commit_show_the_same_summary_as_new_operations() {
        let oid = "0123456789abcdef0123456789abcdef01234567".to_owned();
        let result = CommitResult {
            oid: oid.clone(),
            review: Review::Changed,
        };
        assert_eq!(from_legacy(&format!("{result:#?}")), Some(commit(&result)));
        let change = Change::SavedStash { oid };
        assert_eq!(
            from_legacy(&format!("{change:#?}")),
            Some(mutation(&change))
        );
    }
}
