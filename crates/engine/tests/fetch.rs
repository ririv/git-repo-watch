use engine::{
    cancel::Cancel,
    check,
    domain::{IssueKind, Publication, PushPolicy, RepositoryResult},
    transport::Git,
};
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};
use test_support::*;

fn inspect(f: &Fixture, policy: PushPolicy) -> RepositoryResult {
    check::repository(&Git::default(), &f.repo, policy, &Cancel::default()).0
}
fn upload_log(f: &Fixture) -> PathBuf {
    let wrapper = f.temp.path().join("upload-pack");
    write(
        &wrapper,
        "#!/bin/sh\nprintf 'request\\n' >> \"$0.log\"\nexec /usr/bin/git-upload-pack \"$@\"\n",
    );
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o700)).unwrap();
    git(
        &f.repo,
        &[
            "config",
            "remote.origin.uploadpack",
            wrapper.to_str().unwrap(),
        ],
    );
    wrapper.with_extension("log")
}
fn requests(log: &PathBuf) -> usize {
    match fs::read_to_string(log) {
        Ok(text) => text.lines().count(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => 0,
        Err(error) => panic!("{error}"),
    }
}
fn failed(result: RepositoryResult, expected: &str) {
    match result {
        RepositoryResult::Failed { error, .. } => assert!(error.contains(expected), "{error}"),
        other => panic!("{other:?}"),
    }
}
#[test]
fn ordinary_checks_fetch_once_and_pushes_still_verify_before_and_after() {
    let f = Fixture::create();
    let log = upload_log(&f);
    assert!(matches!(
        inspect(&f, PushPolicy::Disabled),
        RepositoryResult::Clean { .. }
    ));
    assert_eq!(requests(&log), 1);
    f.commit("ahead", "ahead");
    let result = inspect(&f, PushPolicy::FastForward);
    assert!(
        matches!(result.pushes(), [Publication::Pushed { .. }]),
        "{result:?}"
    );
    assert_eq!(requests(&log), 4);
}
#[test]
fn deleted_upstream_is_pruned_and_keeps_its_configured_tracking_identity() {
    let f = Fixture::create();
    git(&f.remote, &["update-ref", "-d", "refs/heads/main"]);
    let result = inspect(&f, PushPolicy::Disabled);
    match result {
        RepositoryResult::Attention {
            issues,
            branches_without_upstream,
            ..
        } => {
            assert!(
                issues
                    .iter()
                    .any(|issue| issue.kind == IssueKind::MissingUpstream)
            );
            assert!(branches_without_upstream.is_empty());
        }
        other => panic!("{other:?}"),
    }
    assert!(
        git2::Repository::open(&f.repo)
            .unwrap()
            .find_reference("refs/remotes/origin/main")
            .is_err()
    );
}
#[test]
fn excluded_or_unmapped_upstream_never_uses_a_stale_tracking_reference() {
    let _language = i18n::using(i18n::Language::English);
    for excluded in [true, false] {
        let f = Fixture::create();
        f.advance();
        let log = upload_log(&f);
        if excluded {
            git(
                &f.repo,
                &["config", "--add", "remote.origin.fetch", "^refs/heads/main"],
            );
        } else {
            git(
                &f.repo,
                &[
                    "config",
                    "remote.origin.fetch",
                    "+refs/heads/other*:refs/remotes/origin/other*",
                ],
            );
        }
        failed(
            inspect(&f, PushPolicy::Disabled),
            if excluded {
                "excludes upstream"
            } else {
                "one fetch destination"
            },
        );
        assert_eq!(requests(&log), 0);
    }
}
#[test]
fn custom_wildcard_mapping_fetches_and_prunes_without_touching_unrelated_refs() {
    let f = Fixture::create();
    git(
        &f.repo,
        &[
            "config",
            "remote.origin.fetch",
            "+refs/heads/*:refs/remotes/custom/*",
        ],
    );
    f.advance();
    let result = inspect(&f, PushPolicy::Disabled);
    assert!(
        matches!(result, RepositoryResult::Attention { .. }),
        "{result:?}"
    );
    assert_eq!(
        git(&f.repo, &["rev-parse", "refs/remotes/custom/main"]),
        git(&f.remote, &["rev-parse", "main"])
    );
    let stale = git(&f.repo, &["rev-parse", "refs/remotes/origin/main"]);
    git(&f.remote, &["update-ref", "-d", "refs/heads/main"]);
    let result = inspect(&f, PushPolicy::Disabled);
    assert!(
        matches!(result, RepositoryResult::Attention { .. }),
        "{result:?}"
    );
    let repo = git2::Repository::open(&f.repo).unwrap();
    assert!(repo.find_reference("refs/remotes/custom/main").is_err());
    assert_eq!(
        repo.refname_to_id("refs/remotes/origin/main")
            .unwrap()
            .to_string(),
        stale
    );
}
#[test]
fn narrow_refspec_fetches_only_its_branch_and_fails_explicitly_if_deleted() {
    let _language = i18n::using(i18n::Language::English);
    let f = Fixture::create();
    git(
        &f.repo,
        &[
            "config",
            "remote.origin.fetch",
            "+refs/heads/main:refs/remotes/origin/main",
        ],
    );
    f.advance();
    assert!(matches!(
        inspect(&f, PushPolicy::Disabled),
        RepositoryResult::Attention { .. }
    ));
    git(&f.remote, &["update-ref", "-d", "refs/heads/main"]);
    // 精确 refspec 的远程源不存在时，Git 拒绝 fetch；不能沿用旧引用判定成功。
    assert!(matches!(
        inspect(&f, PushPolicy::Disabled),
        RepositoryResult::Failed { .. }
    ));
}
#[test]
fn pruning_does_not_delete_tags_or_local_branches_even_with_tag_pruning_configured() {
    let f = Fixture::create();
    git(&f.repo, &["tag", "local-only"]);
    git(&f.repo, &["branch", "experiment"]);
    git(
        &f.repo,
        &[
            "config",
            "--add",
            "remote.origin.fetch",
            "+refs/tags/*:refs/tags/*",
        ],
    );
    git(&f.repo, &["config", "fetch.pruneTags", "true"]);
    git(&f.repo, &["config", "remote.origin.pruneTags", "true"]);
    assert!(matches!(
        inspect(&f, PushPolicy::Disabled),
        RepositoryResult::Clean { .. }
    ));
    let repo = git2::Repository::open(&f.repo).unwrap();
    assert!(repo.find_reference("refs/tags/local-only").is_ok());
    assert!(repo.find_reference("refs/heads/experiment").is_ok());
}
#[test]
fn negative_refspecs_keep_excluded_tracking_refs_but_refresh_included_branches() {
    let f = Fixture::create();
    let oid = git(&f.repo, &["rev-parse", "HEAD"]);
    git(
        &f.repo,
        &["update-ref", "refs/remotes/origin/private/old", &oid],
    );
    git(
        &f.repo,
        &[
            "config",
            "--add",
            "remote.origin.fetch",
            "^refs/heads/private/*",
        ],
    );
    f.advance();
    let result = inspect(&f, PushPolicy::Disabled);
    assert!(
        matches!(result, RepositoryResult::Attention { .. }),
        "{result:?}"
    );
    assert_eq!(
        git(&f.repo, &["rev-parse", "refs/remotes/origin/main"]),
        git(&f.remote, &["rev-parse", "main"])
    );
    assert_eq!(
        git(&f.repo, &["rev-parse", "refs/remotes/origin/private/old"]),
        oid
    );
}
#[test]
fn skipped_upstream_remote_is_reported_instead_of_using_an_old_snapshot() {
    let _language = i18n::using(i18n::Language::English);
    let f = Fixture::create();
    let log = upload_log(&f);
    git(&f.repo, &["config", "remote.origin.skipFetchAll", "true"]);
    failed(inspect(&f, PushPolicy::Disabled), "skipFetchAll");
    assert_eq!(requests(&log), 0);
}

#[test]
fn overlapping_prune_scopes_fail_before_fetching_or_deleting_refs() {
    let _language = i18n::using(i18n::Language::English);
    let f = Fixture::create();
    let log = upload_log(&f);
    git(
        &f.repo,
        &["remote", "add", "other", f.remote.to_str().unwrap()],
    );
    git(
        &f.repo,
        &[
            "config",
            "remote.other.fetch",
            "+refs/heads/*:refs/remotes/origin/*",
        ],
    );
    failed(
        inspect(&f, PushPolicy::Disabled),
        "Overlapping fetch destinations",
    );
    assert_eq!(requests(&log), 0);
    assert!(
        git2::Repository::open(&f.repo)
            .unwrap()
            .find_reference("refs/remotes/origin/main")
            .is_ok()
    );
}
#[test]
fn partial_wildcards_and_multiple_nonoverlapping_refspecs_are_supported() {
    let f = Fixture::create();
    git(
        &f.repo,
        &[
            "config",
            "remote.origin.fetch",
            "+refs/heads/ma*:refs/remotes/custom/ma*",
        ],
    );
    git(
        &f.repo,
        &[
            "config",
            "--add",
            "remote.origin.fetch",
            "+refs/heads/feature/*:refs/remotes/features/*",
        ],
    );
    f.advance();
    assert!(matches!(
        inspect(&f, PushPolicy::Disabled),
        RepositoryResult::Attention { .. }
    ));
    assert_eq!(
        git(&f.repo, &["rev-parse", "refs/remotes/custom/main"]),
        git(&f.remote, &["rev-parse", "main"])
    );
}

#[test]
fn each_remote_is_fetched_once_and_unmapped_upstreams_are_not_experimental() {
    let f = Fixture::create();
    let log = upload_log(&f);
    git(
        &f.repo,
        &["remote", "add", "other", f.remote.to_str().unwrap()],
    );
    let wrapper = f.temp.path().join("upload-pack");
    git(
        &f.repo,
        &[
            "config",
            "remote.other.uploadpack",
            wrapper.to_str().unwrap(),
        ],
    );
    let result = inspect(&f, PushPolicy::Disabled);
    assert!(
        matches!(result, RepositoryResult::Clean { .. }),
        "{result:?}"
    );
    assert_eq!(requests(&log), 2);
    git(&f.repo, &["config", "--unset-all", "remote.origin.fetch"]);
    assert!(matches!(
        inspect(&f, PushPolicy::Disabled),
        RepositoryResult::Failed { .. }
    ));
    assert_eq!(requests(&log), 2);
}
#[test]
fn upstream_mapping_into_local_branches_is_rejected_before_any_mutation() {
    let _language = i18n::using(i18n::Language::English);
    let f = Fixture::create();
    let log = upload_log(&f);
    let before = git(&f.repo, &["rev-parse", "HEAD"]);
    git(
        &f.repo,
        &[
            "config",
            "remote.origin.fetch",
            "+refs/heads/*:refs/heads/*",
        ],
    );
    failed(inspect(&f, PushPolicy::Disabled), "refs/remotes/");
    assert_eq!(requests(&log), 0);
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), before);
}
