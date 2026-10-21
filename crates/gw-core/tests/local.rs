use gw_core::{
    cancel::Cancel,
    domain::Publication,
    local::{
        self,
        files::{self, Area, Change, Files, IndexAction},
        review,
    },
    operations::{commit, push},
    transport::Git,
};
use gw_test_support::*;
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::Path,
};
fn change(snapshot: &Files, name: &str, area: Area) -> Change {
    snapshot
        .changes
        .iter()
        .find(|c| c.location.name() == Path::new(name) && c.area == area)
        .unwrap_or_else(|| panic!("没有文件 {name:?}/{area:?}: {:?}", snapshot.changes))
        .clone()
}
#[test]
fn staged_and_worktree_diff_stay_separate() {
    let f = Fixture::create();
    f.commit("file.txt", "baseline\n");
    write(f.repo.join("file.txt"), "staged\n");
    git(&f.repo, &["add", "file.txt"]);
    write(f.repo.join("file.txt"), "working\n");
    let s = files::read(&f.repo).unwrap();
    let staged = change(&s, "file.txt", Area::Staged);
    let working = change(&s, "file.txt", Area::Unstaged);
    assert!(files::diff(&s, &staged).unwrap().contains("+staged"));
    assert!(files::diff(&s, &working).unwrap().contains("+working"));
    let next = files::edit(&s, IndexAction::Unstage, &[staged]).unwrap();
    change(&next, "file.txt", Area::Unstaged);
    assert_eq!(
        fs::read_to_string(f.repo.join("file.txt")).unwrap(),
        "working\n"
    );
    assert!(git(&f.repo, &["diff", "--cached"]).is_empty());
}
#[test]
fn literal_selected_files_and_initial_unstage() {
    let temp = tempfile::tempdir().unwrap();
    let path = fs::canonicalize(temp.path()).unwrap().join("initial");
    init(&path);
    for name in ["file[1]", "file1", ":(glob)*", "-option", "换行\n名字"] {
        write(path.join(name), name);
    }
    write(path.join(".gitignore"), "secret\n");
    write(path.join("secret"), "private");
    let mut s = files::read(&path).unwrap();
    assert!(
        !s.changes
            .iter()
            .any(|c| c.location.name() == Path::new("secret"))
    );
    for name in ["file[1]", ":(glob)*", "-option", "换行\n名字"] {
        let c = change(&s, name, Area::Untracked);
        assert!(files::diff(&s, &c).unwrap().contains("new file"));
        s = files::edit(&s, IndexAction::Stage, &[c]).unwrap();
    }
    change(&s, "file1", Area::Untracked);
    let staged = change(&s, "file[1]", Area::Staged);
    s = files::edit(&s, IndexAction::Unstage, &[staged]).unwrap();
    change(&s, "file[1]", Area::Untracked);
    assert_eq!(fs::read_to_string(path.join("file[1]")).unwrap(), "file[1]");
}
#[test]
fn rename_and_stale_index_review() {
    let f = Fixture::create();
    f.commit("old.txt", &"baseline\n".repeat(20));
    git(&f.repo, &["mv", "old.txt", "new.txt"]);
    write(
        f.repo.join("new.txt"),
        &format!("{}more\n", "baseline\n".repeat(20)),
    );
    let s = files::read(&f.repo).unwrap();
    let c = change(&s, "new.txt", Area::Unstaged);
    let next = files::edit(&s, IndexAction::Stage, &[c]).unwrap();
    let rename = change(&next, "new.txt", Area::Staged);
    let next = files::edit(&next, IndexAction::Unstage, &[rename]).unwrap();
    change(&next, "new.txt", Area::Unstaged);
    assert!(git(&f.repo, &["diff", "--cached"]).is_empty());
    let stale = next.clone();
    write(f.repo.join("unrelated"), "other");
    git(&f.repo, &["add", "unrelated"]);
    assert!(
        files::edit(
            &stale,
            IndexAction::Stage,
            &[change(&stale, "new.txt", Area::Unstaged)]
        )
        .unwrap_err()
        .to_string()
        .contains("暂存区已变化")
    );
}
#[test]
fn untracked_symlink_does_not_read_target() {
    let f = Fixture::create();
    let target = f.temp.path().join("private");
    write(&target, "private target content");
    symlink(&target, f.repo.join("link")).unwrap();
    let s = files::read(&f.repo).unwrap();
    let text = files::diff(&s, &change(&s, "link", Area::Untracked)).unwrap();
    assert!(text.contains("新符号链接"));
    assert!(!text.contains("private target content"));
}
#[test]
fn commit_preserves_unselected_changes_and_hooks() {
    let f = Fixture::create();
    write(f.repo.join("selected"), "staged\n");
    git(&f.repo, &["add", "selected"]);
    let plan = review::prepare(&f.repo).unwrap();
    write(f.repo.join("selected"), "working\n");
    write(f.repo.join("other"), "untracked\n");
    let result = commit::execute(&Git::default(), &plan, "用户说明", &Cancel::default()).unwrap();
    assert_eq!(result.review, commit::Review::Exact);
    assert_eq!(git(&f.repo, &["show", "HEAD:selected"]), "staged");
    assert_eq!(
        fs::read_to_string(f.repo.join("selected")).unwrap(),
        "working\n"
    );
    assert!(git(&f.repo, &["status", "--porcelain"]).contains("other"));
    write(f.repo.join("hook-test"), "hook");
    git(&f.repo, &["add", "hook-test"]);
    let plan = review::prepare(&f.repo).unwrap();
    let hook = f.repo.join(".git/hooks/pre-commit");
    write(&hook, "#!/bin/sh\nexit 1\n");
    fs::set_permissions(hook, fs::Permissions::from_mode(0o700)).unwrap();
    let before = git(&f.repo, &["rev-parse", "HEAD"]);
    assert!(commit::execute(&Git::default(), &plan, "拒绝", &Cancel::default()).is_err());
    assert_eq!(before, git(&f.repo, &["rev-parse", "HEAD"]));
}
#[test]
fn commit_requires_fresh_snapshot_and_reports_hook_changes() {
    let f = Fixture::create();
    write(f.repo.join("one"), "one");
    git(&f.repo, &["add", "one"]);
    let plan = review::prepare(&f.repo).unwrap();
    write(f.repo.join("two"), "two");
    git(&f.repo, &["add", "two"]);
    assert!(commit::execute(&Git::default(), &plan, "stale", &Cancel::default()).is_err());
    let plan = review::prepare(&f.repo).unwrap();
    let hook = f.repo.join(".git/hooks/pre-commit");
    write(
        &hook,
        "#!/bin/sh\nprintf hook > hook-file\ngit add hook-file\n",
    );
    fs::set_permissions(hook, fs::Permissions::from_mode(0o700)).unwrap();
    let result =
        commit::execute(&Git::default(), &plan, "hook modifies", &Cancel::default()).unwrap();
    assert_eq!(result.review, commit::Review::Changed);
}
#[test]
fn explicit_push_preserves_upstream_policy_and_rejects_stale_review() {
    let f = Fixture::create();
    f.commit("ahead", "ahead");
    let git_runner = Git::default();
    let cancel = Cancel::default();
    let plan = push::prepare(&git_runner, &f.repo, &cancel).unwrap();
    let before = git(&f.remote, &["rev-parse", "main"]);
    assert_ne!(before, plan.target.local_oid);
    assert!(matches!(
        push::execute(&git_runner, &plan, &cancel),
        Publication::Pushed { .. }
    ));
    assert_eq!(
        git(&f.remote, &["rev-parse", "main"]),
        plan.target.local_oid
    );
    f.commit("next", "next");
    let plan = push::prepare(&git_runner, &f.repo, &cancel).unwrap();
    f.commit("later", "later");
    assert!(matches!(
        push::execute(&git_runner, &plan, &cancel),
        Publication::Failed { .. }
    ));
    assert_ne!(
        git(&f.remote, &["rev-parse", "main"]),
        plan.target.local_oid
    );
    git(&f.repo, &["branch", "--unset-upstream"]);
    assert!(
        push::prepare(&git_runner, &f.repo, &cancel)
            .unwrap_err()
            .to_string()
            .contains("远程上游")
    );
}
#[test]
fn detached_conflict_and_behind_are_explicit() {
    let f = Fixture::create();
    git(&f.repo, &["switch", "--detach"]);
    assert!(
        files::read(&f.repo)
            .unwrap_err()
            .to_string()
            .contains("游离")
    );
    git(&f.repo, &["switch", "main"]);
    f.advance();
    assert!(
        push::prepare(&Git::default(), &f.repo, &Cancel::default())
            .unwrap_err()
            .to_string()
            .contains("落后")
    );
    let repo = local::open(&f.repo).unwrap();
    assert_eq!(local::head(&repo).unwrap().branch(), "main");
}

#[test]
fn external_filters_are_explicit_and_do_not_silently_stage_raw_data() {
    let f = Fixture::create();
    write(f.repo.join(".gitattributes"), "*.bin filter=example\n");
    git(&f.repo, &["config", "filter.example.clean", "cat"]);
    write(f.repo.join("data.bin"), "raw");
    let snapshot = files::read(&f.repo).unwrap();
    let error = files::edit(
        &snapshot,
        IndexAction::Stage,
        &[change(&snapshot, "data.bin", Area::Untracked)],
    )
    .unwrap_err();
    assert!(error.to_string().contains("外部 filter"));
    assert!(git(&f.repo, &["diff", "--cached"]).is_empty());
}
#[test]
fn submodule_worktree_changes_override_ignore_configuration() {
    let parent = Fixture::create();
    let sub = Fixture::create();
    git(
        &parent.repo,
        &[
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "add",
            sub.remote.to_str().unwrap(),
            "sub",
        ],
    );
    git(&parent.repo, &["commit", "-am", "submodule"]);
    git(&parent.repo, &["config", "submodule.sub.ignore", "all"]);
    write(parent.repo.join("sub/untracked"), "dirty");
    let repo = local::open(&parent.repo).unwrap();
    assert!(files::dirty(&repo).unwrap());
    let snapshot = files::read(&parent.repo).unwrap();
    change(&snapshot, "sub", Area::Unstaged);
}
#[test]
fn first_commit_is_supported_with_system_git() {
    let temp = tempfile::tempdir().unwrap();
    let path = fs::canonicalize(temp.path()).unwrap().join("initial");
    init(&path);
    write(path.join("first"), "first");
    let s = files::read(&path).unwrap();
    files::edit(
        &s,
        IndexAction::Stage,
        &[change(&s, "first", Area::Untracked)],
    )
    .unwrap();
    let plan = review::prepare(&path).unwrap();
    let result =
        commit::execute(&Git::default(), &plan, "first commit", &Cancel::default()).unwrap();
    assert_eq!(result.review, commit::Review::Exact);
    assert_eq!(git(&path, &["show", "HEAD:first"]), "first");
}
#[test]
fn exact_push_ignores_mirror_tag_following_and_remote_push_defaults() {
    let f = Fixture::create();
    f.commit("ahead", "ahead");
    git(&f.repo, &["branch", "other"]);
    git(&f.repo, &["tag", "-a", "local-tag", "-m", "tag"]);
    let plan = push::prepare(&Git::default(), &f.repo, &Cancel::default()).unwrap();
    git(&f.repo, &["config", "remote.origin.mirror", "true"]);
    git(&f.repo, &["config", "push.followTags", "true"]);
    git(
        &f.repo,
        &[
            "config",
            "remote.origin.push",
            "refs/heads/other:refs/heads/other",
        ],
    );
    assert!(matches!(
        push::execute(&Git::default(), &plan, &Cancel::default()),
        Publication::Pushed { .. }
    ));
    assert_eq!(
        git(&f.remote, &["for-each-ref", "--format=%(refname)"]),
        "refs/heads/main"
    );
}
#[test]
fn changed_push_url_and_server_rejection_preserve_remote() {
    let f = Fixture::create();
    f.commit("ahead", "ahead");
    let plan = push::prepare(&Git::default(), &f.repo, &Cancel::default()).unwrap();
    let before = git(&f.remote, &["rev-parse", "main"]);
    git(
        &f.repo,
        &[
            "config",
            "remote.origin.pushurl",
            f.temp.path().join("other.git").to_str().unwrap(),
        ],
    );
    assert!(matches!(
        push::execute(&Git::default(), &plan, &Cancel::default()),
        Publication::Failed { .. }
    ));
    assert_eq!(before, git(&f.remote, &["rev-parse", "main"]));
    git(&f.repo, &["config", "--unset", "remote.origin.pushurl"]);
    let hook = f.remote.join("hooks/pre-receive");
    write(&hook, "#!/bin/sh\nprintf rejected >&2\nexit 1\n");
    fs::set_permissions(hook, fs::Permissions::from_mode(0o700)).unwrap();
    assert!(matches!(
        push::execute(&Git::default(), &plan, &Cancel::default()),
        Publication::Failed {
            phase: gw_core::domain::Phase::Transfer,
            ..
        }
    ));
    assert_eq!(before, git(&f.remote, &["rev-parse", "main"]));
}

#[test]
fn url_rewrites_compare_effective_fetch_and_push_destinations() {
    let f = Fixture::create();
    f.commit("ahead", "ahead");
    let real = f.remote.to_str().unwrap();
    git(&f.repo, &["remote", "set-url", "origin", "alias:remote"]);
    git(
        &f.repo,
        &["config", &format!("url.{real}.insteadOf"), "alias:remote"],
    );
    assert!(push::prepare(&Git::default(), &f.repo, &Cancel::default()).is_ok());
    git(
        &f.repo,
        &[
            "config",
            &format!(
                "url.{}.pushInsteadOf",
                f.temp.path().join("other.git").display()
            ),
            "alias:",
        ],
    );
    assert!(push::prepare(&Git::default(), &f.repo, &Cancel::default()).is_err());
}
