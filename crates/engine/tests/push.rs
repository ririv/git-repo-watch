use engine::{
    cancel::Cancel,
    domain::{Phase, Publication, RemoteTip},
    operations::push::{self, Preparation, PushPlan, TargetRequest},
    transport::Git,
};
use std::{fs, os::unix::fs::PermissionsExt};
use test_support::{Fixture, git, write};

fn prepare(f: &Fixture) -> Preparation {
    push::prepare_interactive(&Git::default(), &f.repo, &Cancel::default()).unwrap()
}
fn prepared(f: &Fixture) -> PushPlan {
    let Preparation::Ready(plan) = prepare(f) else {
        panic!("需要推送计划");
    };
    plan
}
fn target_request(f: &Fixture) -> TargetRequest {
    git(&f.repo, &["branch", "--unset-upstream"]);
    git(&f.repo, &["config", "push.autoSetupRemote", "false"]);
    let Preparation::Target(request) = prepare(f) else {
        panic!("需要目标选择");
    };
    request
}
fn execute(plan: &PushPlan) -> Publication {
    push::execute(&Git::default(), plan, &Cancel::default())
}
fn hook(f: &Fixture, text: &str) {
    let path = f.repo.join(".git/hooks/pre-push");
    write(&path, text);
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
}
fn target(request: &TargetRequest, remote: &str, branch: &str) -> PushPlan {
    push::prepare_target(&Git::default(), request, remote, branch, &Cancel::default()).unwrap()
}

#[test]
fn ordinary_push_is_direct_and_preserves_all_worktree_and_index_changes() {
    let f = Fixture::create();
    f.commit("tracked", "committed\n");
    write(f.repo.join("tracked"), "unstaged\n");
    write(f.repo.join("staged"), "staged\n");
    git(&f.repo, &["add", "staged"]);
    let status = git(&f.repo, &["status", "--porcelain"]);
    let before = git(&f.remote, &["rev-parse", "main"]);
    let plan = prepared(&f);
    assert!(!plan.needs_confirmation());
    assert_eq!(before, git(&f.remote, &["rev-parse", "main"]));
    assert!(matches!(execute(&plan), Publication::Pushed { .. }));
    assert_eq!(
        git(&f.remote, &["rev-parse", "main"]),
        plan.target().local_oid
    );
    assert_eq!(status, git(&f.repo, &["status", "--porcelain"]));
    assert!(matches!(prepare(&f), Preparation::Current));
}

#[test]
fn divergent_and_behind_only_pushes_require_force_but_automation_still_rejects_them() {
    for local_commit in [false, true] {
        let f = Fixture::create();
        f.advance();
        if local_commit {
            f.commit("local", "local");
        }
        let before = git(&f.remote, &["rev-parse", "main"]);
        let plan = prepared(&f);
        assert!(plan.needs_confirmation());
        assert_eq!(plan.force_behind(), Some(1));
        assert_eq!(before, git(&f.remote, &["rev-parse", "main"]));
        assert!(push::prepare(&Git::default(), &f.repo, &Cancel::default()).is_err());
        assert!(matches!(execute(&plan), Publication::Pushed { .. }));
        assert_eq!(
            git(&f.remote, &["rev-parse", "main"]),
            plan.target().local_oid
        );
    }
}

#[test]
fn remote_change_after_review_requires_a_new_force_confirmation() {
    let f = Fixture::create();
    f.advance();
    f.commit("local", "local");
    let plan = prepared(&f);
    git(
        &f.temp.path().join("peer"),
        &[
            "-c",
            "user.name=test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "--allow-empty",
            "-m",
            "changed again",
        ],
    );
    git(&f.temp.path().join("peer"), &["push"]);
    let before = git(&f.remote, &["rev-parse", "main"]);
    assert!(matches!(
        execute(&plan),
        Publication::Failed {
            phase: Phase::Validate,
            ..
        }
    ));
    assert_eq!(before, git(&f.remote, &["rev-parse", "main"]));
}

#[test]
fn exact_force_lease_rejects_a_remote_race_after_the_last_preflight() {
    let f = Fixture::create();
    f.advance();
    f.commit("local", "local");
    let plan = prepared(&f);
    let peer = f.temp.path().join("peer");
    git(
        &peer,
        &[
            "-c",
            "user.name=test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "--allow-empty",
            "-m",
            "race",
        ],
    );
    git(&peer, &["push", "origin", "HEAD:refs/heads/raced"]);
    let raced = git(&f.remote, &["rev-parse", "raced"]);
    hook(
        &f,
        &format!(
            "#!/bin/sh\n/usr/bin/git -C '{}' update-ref refs/heads/main '{}'\n",
            f.remote.display(),
            raced
        ),
    );
    assert!(matches!(
        execute(&plan),
        Publication::Failed {
            phase: Phase::Transfer,
            ..
        }
    ));
    assert_eq!(git(&f.remote, &["rev-parse", "main"]), raced);
}

#[test]
fn first_publication_selects_a_remote_and_unicode_branch_and_sets_upstream_only_on_success() {
    let f = Fixture::create();
    let request = target_request(&f);
    let plan = target(&request, "origin", "发布/功能");
    assert!(!plan.needs_confirmation());
    assert!(matches!(plan.target().remote_oid, RemoteTip::Absent));
    assert_eq!(
        engine::local::open(&f.repo)
            .unwrap()
            .config()
            .unwrap()
            .get_string("branch.main.remote")
            .unwrap_err()
            .code(),
        git2::ErrorCode::NotFound
    );
    assert!(matches!(execute(&plan), Publication::Pushed { .. }));
    assert_eq!(
        git(&f.remote, &["rev-parse", "发布/功能"]),
        plan.target().local_oid
    );
    assert_eq!(git(&f.repo, &["config", "branch.main.remote"]), "origin");
    assert_eq!(
        git(&f.repo, &["config", "branch.main.merge"]),
        "refs/heads/发布/功能"
    );
    assert_eq!(
        git(&f.repo, &["rev-parse", "--abbrev-ref", "@{upstream}"]),
        "origin/发布/功能"
    );
}

#[test]
fn failed_first_publication_never_saves_an_upstream() {
    let f = Fixture::create();
    let request = target_request(&f);
    let plan = target(&request, "origin", "new");
    hook(&f, "#!/bin/sh\nexit 1\n");
    assert!(matches!(
        execute(&plan),
        Publication::Failed {
            phase: Phase::Transfer,
            ..
        }
    ));
    let repo = engine::local::open(&f.repo).unwrap();
    assert_eq!(
        repo.config()
            .unwrap()
            .get_string("branch.main.remote")
            .unwrap_err()
            .code(),
        git2::ErrorCode::NotFound
    );
    assert!(push::prepare(&Git::default(), &f.repo, &Cancel::default()).is_err());
    assert!(git(&f.remote, &["for-each-ref", "refs/heads/new"]).is_empty());
}

#[test]
fn creating_a_branch_uses_an_absence_lease_and_never_overwrites_a_racing_publication() {
    let f = Fixture::create();
    let request = target_request(&f);
    f.advance();
    let raced = git(&f.remote, &["rev-parse", "main"]);
    let plan = target(&request, "origin", "new");
    hook(
        &f,
        &format!(
            "#!/bin/sh\n/usr/bin/git -C '{}' update-ref refs/heads/new '{}'\n",
            f.remote.display(),
            raced
        ),
    );
    assert!(matches!(
        execute(&plan),
        Publication::Failed {
            phase: Phase::Transfer,
            ..
        }
    ));
    assert_eq!(git(&f.remote, &["rev-parse", "new"]), raced);
}

#[test]
fn automatic_upstream_respects_git_configuration_and_never_hides_force_review() {
    let f = Fixture::create();
    let _request = target_request(&f);
    git(&f.repo, &["config", "push.autoSetupRemote", "true"]);
    git(&f.repo, &["config", "push.default", "simple"]);
    f.commit("ahead", "ahead");
    let plan = prepared(&f);
    assert!(!plan.needs_confirmation());
    assert!(matches!(execute(&plan), Publication::Pushed { .. }));
    assert_eq!(
        git(&f.repo, &["config", "branch.main.merge"]),
        "refs/heads/main"
    );
    git(&f.repo, &["branch", "--unset-upstream"]);
    git(&f.repo, &["config", "push.default", "matching"]);
    assert!(matches!(prepare(&f), Preparation::Target(_)));
    git(&f.repo, &["config", "push.default", "current"]);
    f.advance();
    assert_eq!(prepared(&f).force_behind(), Some(1));
}

#[test]
fn existing_equal_target_only_sets_tracking_and_a_deleted_upstream_is_reviewed_before_recreation() {
    let f = Fixture::create();
    let request = target_request(&f);
    let plan = target(&request, "origin", "main");
    hook(&f, "#!/bin/sh\nexit 1\n");
    assert!(matches!(execute(&plan), Publication::Pushed { .. }));
    fs::remove_file(f.repo.join(".git/hooks/pre-push")).unwrap();
    git(&f.remote, &["update-ref", "-d", "refs/heads/main"]);
    let plan = prepared(&f);
    assert!(plan.needs_confirmation());
    assert!(plan.force_behind().is_none());
    assert!(matches!(execute(&plan), Publication::Pushed { .. }));
}

#[test]
fn target_editing_rejects_invalid_names_and_changes_to_frozen_identity() {
    let f = Fixture::create();
    let request = target_request(&f);
    for branch in ["", "bad name", "-force", "../oops"] {
        assert!(
            push::prepare_target(
                &Git::default(),
                &request,
                "origin",
                branch,
                &Cancel::default()
            )
            .is_err()
        );
    }
    f.commit("new", "new");
    assert!(
        push::prepare_target(
            &Git::default(),
            &request,
            "origin",
            "new",
            &Cancel::default()
        )
        .is_err()
    );
    let Preparation::Target(request) = prepare(&f) else {
        panic!();
    };
    let plan = target(&request, "origin", "new");
    git(&f.repo, &["branch", "--set-upstream-to=origin/main"]);
    assert!(matches!(
        execute(&plan),
        Publication::Failed {
            phase: Phase::Validate,
            ..
        }
    ));
}

#[test]
fn persisted_pushes_read_both_legacy_remote_hashes_and_new_branch_creation() {
    let existing: RemoteTip =
        serde_json::from_str("\"0123456789012345678901234567890123456789\"").unwrap();
    assert!(matches!(existing, RemoteTip::Existing(_)));
    let absent: RemoteTip =
        serde_json::from_str("\"0000000000000000000000000000000000000000\"").unwrap();
    assert_eq!(absent, RemoteTip::Absent);
    assert_eq!(
        serde_json::to_string(&absent).unwrap(),
        "\"0000000000000000000000000000000000000000\""
    );
}

#[test]
fn automatic_target_uses_push_remote_precedence_and_custom_push_rules_require_explicit_selection() {
    let f = Fixture::create();
    let _request = target_request(&f);
    let backup = f.temp.path().join("backup.git");
    fs::create_dir(&backup).unwrap();
    git(&backup, &["init", "--bare"]);
    git(
        &f.repo,
        &["remote", "add", "backup", backup.to_str().unwrap()],
    );
    git(&f.repo, &["config", "push.autoSetupRemote", "true"]);
    git(&f.repo, &["config", "remote.pushDefault", "origin"]);
    git(&f.repo, &["config", "branch.main.pushRemote", "backup"]);
    let plan = prepared(&f);
    assert_eq!(plan.target().remote, "backup");
    git(
        &f.repo,
        &["config", "remote.backup.push", "HEAD:refs/heads/review"],
    );
    assert!(matches!(prepare(&f), Preparation::Target(_)));
    git(&f.repo, &["config", "--unset", "remote.backup.push"]);
    git(&f.repo, &["config", "--unset", "branch.main.pushRemote"]);
    assert_eq!(prepared(&f).target().remote, "origin");
    git(&f.repo, &["config", "remote.pushDefault", "missing"]);
    assert!(push::prepare_interactive(&Git::default(), &f.repo, &Cancel::default()).is_err());
}

#[test]
fn new_upstream_uses_custom_fetch_mappings_and_preserves_external_configuration_changes() {
    let f = Fixture::create();
    let request = target_request(&f);
    git(
        &f.repo,
        &[
            "config",
            "remote.origin.fetch",
            "+refs/heads/*:refs/remotes/cache/*",
        ],
    );
    let plan = target(&request, "origin", "发布");
    assert!(matches!(execute(&plan), Publication::Pushed { .. }));
    assert_eq!(
        git(
            &f.repo,
            &["rev-parse", "--symbolic-full-name", "@{upstream}"]
        ),
        "refs/remotes/cache/发布"
    );
    let request = target_request(&f);
    let plan = target(&request, "origin", "another");
    hook(
        &f,
        "#!/bin/sh\n/usr/bin/git config branch.main.remote .\n/usr/bin/git config branch.main.merge refs/heads/main\n",
    );
    assert!(matches!(
        execute(&plan),
        Publication::Failed {
            phase: Phase::Verify,
            ..
        }
    ));
    assert_eq!(git(&f.repo, &["config", "branch.main.remote"]), ".");
    assert_eq!(
        git(&f.repo, &["config", "branch.main.merge"]),
        "refs/heads/main"
    );
    assert_eq!(
        git(&f.remote, &["rev-parse", "another"]),
        plan.target().local_oid
    );
}

#[test]
fn malformed_non_branch_upstreams_are_rejected_before_fetch_or_push() {
    let f = Fixture::create();
    let oid = git(&f.remote, &["rev-parse", "main"]);
    git(&f.remote, &["update-ref", "refs/tags/version", &oid]);
    git(
        &f.repo,
        &[
            "config",
            "--add",
            "remote.origin.fetch",
            "+refs/tags/*:refs/remotes/origin/tags/*",
        ],
    );
    git(
        &f.repo,
        &["config", "branch.main.merge", "refs/tags/version"],
    );
    let error =
        push::prepare_interactive(&Git::default(), &f.repo, &Cancel::default()).unwrap_err();
    assert!(error.to_string().contains(i18n::tr!(InvalidPushTarget)));
    assert!(git(&f.repo, &["for-each-ref", "refs/remotes/origin/tags"]).is_empty());
    assert_eq!(git(&f.remote, &["rev-parse", "refs/tags/version"]), oid);
}
