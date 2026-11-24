use super::*;
use crate::{cancel::Cancel, transport::Git};
use test_support as support;
fn changes() -> support::Fixture {
    let f = support::Fixture::create();
    let text = (0..40).map(|i| format!("line {i}\n")).collect::<String>();
    f.commit("file 中文.txt", &text);
    let text = text
        .replace("line 2\n", "first topic\n")
        .replace("line 32\n", "second topic\n");
    support::write(f.repo.join("file 中文.txt"), &text);
    f
}
fn separate(snapshot: Snapshot) -> Prepared {
    let commits = snapshot
        .units
        .iter()
        .map(|unit| Step {
            message: format!("change {}\n\nbody", unit.id),
            units: vec![unit.id.clone()],
        })
        .collect();
    Prepared {
        snapshot,
        plan: Plan { commits },
        completed: Vec::new(),
    }
}
#[test]
fn unborn_repository_without_an_index_can_commit_one_new_file_at_a_time() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("unborn");
    support::init(&path);
    support::write(path.join("first.txt"), "first\n");
    support::write(path.join("second.txt"), "second\n");
    assert!(!path.join(".git/index").exists());
    let prepared = separate(capture(&path, &Cancel::default()).unwrap());
    assert!(!path.join(".git/index").exists());
    let Progress::More { prepared, .. } =
        execute(&Git::default(), prepared, &Cancel::default()).unwrap()
    else {
        panic!()
    };
    assert_eq!(
        support::git(&path, &["ls-tree", "--name-only", "HEAD"]),
        "first.txt"
    );
    assert!(support::git(&path, &["diff", "--cached"]).is_empty());
    assert_eq!(
        support::git(&path, &["status", "--porcelain"]),
        "?? second.txt"
    );
    assert!(matches!(
        execute(&Git::default(), prepared, &Cancel::default()).unwrap(),
        Progress::Complete { .. }
    ));
    assert!(support::git(&path, &["status", "--porcelain"]).is_empty());
    assert_eq!(support::git(&path, &["rev-list", "--count", "HEAD"]), "2");
}
#[test]
fn same_file_hunks_commit_separately_without_modifying_worktree_or_staging_the_rest() {
    let f = changes();
    let disk = std::fs::read(f.repo.join("file 中文.txt")).unwrap();
    let mut plan = separate(capture(&f.repo, &Cancel::default()).unwrap());
    assert_eq!(plan.snapshot.units.len(), 2);
    assert!(
        plan.snapshot
            .units
            .iter()
            .all(|unit| matches!(unit.change, Change::Hunk { .. }))
    );
    plan.validate().unwrap();
    let Progress::More { prepared, .. } =
        execute(&Git::default(), plan, &Cancel::default()).unwrap()
    else {
        panic!()
    };
    plan = prepared;
    let first = support::git(&f.repo, &["show", "HEAD:file 中文.txt"]);
    assert!(first.contains("first topic") && first.contains("line 32"));
    assert!(support::git(&f.repo, &["diff", "--cached"]).is_empty());
    assert!(support::git(&f.repo, &["diff"]).contains("second topic"));
    let Progress::Complete { completed } =
        execute(&Git::default(), plan, &Cancel::default()).unwrap()
    else {
        panic!()
    };
    assert_eq!(completed.len(), 2);
    assert_eq!(std::fs::read(f.repo.join("file 中文.txt")).unwrap(), disk);
    assert!(support::git(&f.repo, &["status", "--porcelain"]).is_empty());
}
#[test]
fn original_staging_is_retained_and_partial_staging_dependencies_are_explicit() {
    let f = changes();
    let second = std::fs::read_to_string(f.repo.join("file 中文.txt")).unwrap();
    support::write(
        f.repo.join("file 中文.txt"),
        &second.replace("second topic", "line 32"),
    );
    support::git(&f.repo, &["add", "."]);
    support::write(f.repo.join("file 中文.txt"), &second);
    support::write(f.repo.join("new 空格.txt"), "new topic\n");
    let plan = separate(capture(&f.repo, &Cancel::default()).unwrap());
    assert_eq!(plan.snapshot.units.len(), 3);
    let work = plan
        .snapshot
        .units
        .iter()
        .find(|unit| !unit.dependencies.is_empty())
        .unwrap();
    assert_eq!(work.layer, Layer::Worktree);
    let mut invalid = plan.clone();
    invalid.plan.commits.swap(0, 1);
    assert!(invalid.validate().is_err());
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    assert!(execute(&Git::default(), invalid, &Cancel::default()).is_err());
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    let Progress::More { prepared, .. } =
        execute(&Git::default(), plan, &Cancel::default()).unwrap()
    else {
        panic!()
    };
    assert!(support::git(&f.repo, &["diff", "--cached"]).is_empty());
    assert!(support::git(&f.repo, &["diff"]).contains("second topic"));
    prepared.validate().unwrap();
}
#[test]
fn stale_contents_and_an_external_index_lock_never_rewrite_the_index() {
    let f = changes();
    let plan = separate(capture(&f.repo, &Cancel::default()).unwrap());
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    support::write(f.repo.join(".git/index.lock"), "external");
    assert!(execute(&Git::default(), plan.clone(), &Cancel::default()).is_err());
    assert_eq!(
        std::fs::read(f.repo.join(".git/index.lock")).unwrap(),
        b"external"
    );
    std::fs::remove_file(f.repo.join(".git/index.lock")).unwrap();
    support::write(f.repo.join("unrelated.txt"), "new change after planning");
    assert!(execute(&Git::default(), plan, &Cancel::default()).is_err());
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    assert!(!f.repo.join(".git/index.lock").exists());
}
#[test]
fn commit_hook_failure_preserves_index_and_worktree_and_does_not_advance_head() {
    let f = changes();
    let plan = separate(capture(&f.repo, &Cancel::default()).unwrap());
    let head = support::git(&f.repo, &["rev-parse", "HEAD"]);
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    let hook = f.repo.join(".git/hooks/pre-commit");
    test_support::write_git_hook(&hook, "#!/bin/sh\nexit 1\n");

    assert!(execute(&Git::default(), plan, &Cancel::default()).is_err());
    assert_eq!(support::git(&f.repo, &["rev-parse", "HEAD"]), head);
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    assert!(!f.repo.join(".git/index.lock").exists());
}
#[test]
fn duplicate_missing_unknown_units_and_dependency_orders_are_rejected_before_execution() {
    let f = changes();
    let plan = separate(capture(&f.repo, &Cancel::default()).unwrap());
    let mut duplicate = plan.clone();
    duplicate.plan.commits[1].units = duplicate.plan.commits[0].units.clone();
    assert!(duplicate.validate().is_err());
    let mut missing = plan.clone();
    missing.plan.commits.pop();
    assert!(missing.validate().is_err());
    let mut unknown = plan.clone();
    unknown.plan.commits[0].units = vec!["invented".into()];
    assert!(unknown.validate().is_err());
    let mut reversed = plan;
    reversed.plan.commits.reverse();
    reversed.validate().unwrap();
}

#[test]
fn unrelated_original_staged_changes_remain_staged_after_a_worktree_only_commit() {
    let f = changes();
    support::write(f.repo.join("already staged.txt"), "keep staged\n");
    support::git(&f.repo, &["add", "already staged.txt"]);
    let mut plan = separate(capture(&f.repo, &Cancel::default()).unwrap());
    plan.plan.commits.swap(0, 1);
    plan.validate().unwrap();
    let Progress::More { prepared, .. } =
        execute(&Git::default(), plan, &Cancel::default()).unwrap()
    else {
        panic!()
    };
    assert_eq!(
        support::git(&f.repo, &["diff", "--cached", "--name-only"]),
        "already staged.txt"
    );
    assert!(support::git(&f.repo, &["diff"]).contains("second topic"));
    prepared.validate().unwrap();
}
#[test]
fn crlf_and_missing_final_newlines_survive_partial_commits() {
    for ending in ["\r\n", "\n"] {
        let f = support::Fixture::create();
        let base = (0..40)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join(ending);
        f.commit("text.txt", &base);
        let changed = base
            .replace("line 2\n", "first topic\n")
            .replace("line 2\r\n", "first topic\r\n")
            .replace("line 39", "last topic");
        support::write(f.repo.join("text.txt"), &changed);
        let mut plan = separate(capture(&f.repo, &Cancel::default()).unwrap());
        assert_eq!(plan.snapshot.units.len(), 2);
        while let Progress::More { prepared, .. } =
            execute(&Git::default(), plan, &Cancel::default()).unwrap()
        {
            plan = prepared;
        }
        assert_eq!(
            std::fs::read(f.repo.join("text.txt")).unwrap(),
            changed.as_bytes()
        );
        assert!(support::git(&f.repo, &["status", "--porcelain"]).is_empty());
    }
}
#[test]
fn binary_files_and_renames_are_indivisible_units() {
    let f = support::Fixture::create();
    f.commit("before.txt", "same contents\n");
    support::git(&f.repo, &["mv", "before.txt", "after 中文.txt"]);
    std::fs::write(f.repo.join("binary.bin"), [0, 255, 2, 128]).unwrap();
    let plan = separate(capture(&f.repo, &Cancel::default()).unwrap());
    assert_eq!(plan.snapshot.units.len(), 2);
    assert!(
        plan.snapshot
            .units
            .iter()
            .all(|unit| matches!(unit.change, Change::Whole { .. }))
    );
    let Progress::More { prepared, .. } =
        execute(&Git::default(), plan, &Cancel::default()).unwrap()
    else {
        panic!()
    };
    let Progress::Complete { .. } = execute(&Git::default(), prepared, &Cancel::default()).unwrap()
    else {
        panic!()
    };
    assert!(!f.repo.join("before.txt").exists());
    assert!(support::git(&f.repo, &["status", "--porcelain"]).is_empty());
}
#[test]
fn hooks_that_add_committed_content_stop_the_plan_and_reconcile_the_index() {
    let f = changes();
    let plan = separate(capture(&f.repo, &Cancel::default()).unwrap());
    let before = support::git(&f.repo, &["rev-parse", "HEAD"]);
    let hook = f.repo.join(".git/hooks/pre-commit");
    test_support::write_git_hook(
        &hook,
        "#!/bin/sh\nprintf 'hook content\\n' > hook.txt\ngit add -- hook.txt\n",
    );

    let error = execute(&Git::default(), plan, &Cancel::default()).unwrap_err();
    assert!(
        format!("{error:#}").contains(i18n::tr!(ComposeHookChanged)),
        "{error:#}"
    );
    assert_ne!(support::git(&f.repo, &["rev-parse", "HEAD"]), before);
    assert!(support::git(&f.repo, &["diff", "--cached"]).is_empty());
    assert_eq!(
        support::git(&f.repo, &["show", "HEAD:hook.txt"]),
        "hook content"
    );
    assert!(support::git(&f.repo, &["diff"]).contains("second topic"));
}

#[cfg(unix)]
#[test]
fn cancelling_a_commit_hook_releases_the_index_lock_and_retains_pending_changes() {
    use std::{
        os::unix::fs::PermissionsExt,
        time::{Duration, Instant},
    };
    let f = changes();
    let plan = separate(capture(&f.repo, &Cancel::default()).unwrap());
    let before = support::git(&f.repo, &["rev-parse", "HEAD"]);
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    let hook = f.repo.join(".git/hooks/pre-commit");
    support::write(&hook, "#!/bin/sh\ntouch .git/started\nsleep 20\n");
    std::fs::set_permissions(hook, std::fs::Permissions::from_mode(0o755)).unwrap();
    let cancel = Cancel::default();
    let worker_cancel = cancel.clone();
    let worker = std::thread::spawn(move || execute(&Git::default(), plan, &worker_cancel));
    let start = Instant::now();
    while !f.repo.join(".git/started").exists() {
        assert!(start.elapsed() < Duration::from_secs(8));
        std::thread::sleep(Duration::from_millis(3));
    }
    cancel.cancel();
    assert!(worker.join().unwrap().is_err());
    assert_eq!(support::git(&f.repo, &["rev-parse", "HEAD"]), before);
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    assert!(!f.repo.join(".git/index.lock").exists());
    assert!(support::git(&f.repo, &["diff"]).contains("second topic"));
}

#[cfg(unix)]
#[test]
fn unrelated_index_flags_and_permissions_survive_partial_commits() {
    use std::os::unix::fs::PermissionsExt;
    let f = changes();
    // 先保留现有改动，再加入一个与编排无关的已跟踪文件。
    let pending = std::fs::read(f.repo.join("file 中文.txt")).unwrap();
    support::git(&f.repo, &["restore", "--", "file 中文.txt"]);
    f.commit("metadata.txt", "kept\n");
    std::fs::write(f.repo.join("file 中文.txt"), pending).unwrap();
    support::git(
        &f.repo,
        &["update-index", "--assume-unchanged", "metadata.txt"],
    );
    let index = f.repo.join(".git/index");
    std::fs::set_permissions(&index, std::fs::Permissions::from_mode(0o600)).unwrap();
    let plan = separate(capture(&f.repo, &Cancel::default()).unwrap());
    assert!(matches!(
        execute(&Git::default(), plan, &Cancel::default()).unwrap(),
        Progress::More { .. }
    ));
    assert!(support::git(&f.repo, &["ls-files", "-v", "metadata.txt"]).starts_with('h'));
    assert_eq!(
        std::fs::metadata(index).unwrap().permissions().mode() & 0o777,
        0o600
    );
}
