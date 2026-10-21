use engine::{
    cancel::Cancel,
    check,
    domain::{IssueKind, Publication, PushPolicy, RepositoryResult},
    scheduler::Manager,
    service::{Service, group},
    state::{Config, Store},
    transport::Git,
};
use std::{fs, path::PathBuf};
use test_support::*;
#[test]
fn clean_dirty_untracked_and_experimental_branches() {
    let _language = i18n::using(i18n::Language::Chinese);
    let f = Fixture::create();
    git(&f.repo, &["branch", "experiment"]);
    let (result, timing) = check::repository(
        &Git::default(),
        &f.repo,
        PushPolicy::Disabled,
        &Cancel::default(),
    );
    assert!(timing.elapsed_ms > 0);
    match result {
        RepositoryResult::Clean {
            branches_without_upstream,
            ..
        } => assert_eq!(branches_without_upstream.len(), 1),
        other => panic!("{other:?}"),
    };
    write(f.repo.join("new"), "new");
    let (result, _) = check::repository(
        &Git::default(),
        &f.repo,
        PushPolicy::Disabled,
        &Cancel::default(),
    );
    match result {
        RepositoryResult::Attention { issues, .. } => {
            assert!(issues.iter().any(|i| i.kind == IssueKind::Dirty))
        }
        other => panic!("{other:?}"),
    }
}
#[test]
fn automatic_push_only_publishes_ahead_commits() {
    let _language = i18n::using(i18n::Language::Chinese);
    let f = Fixture::create();
    f.commit("ahead", "ahead");
    write(f.repo.join("untracked"), "private");
    let (result, _) = check::repository(
        &Git::default(),
        &f.repo,
        PushPolicy::FastForward,
        &Cancel::default(),
    );
    assert_eq!(
        git(&f.repo, &["rev-parse", "HEAD"]),
        git(&f.remote, &["rev-parse", "main"])
    );
    assert!(matches!(result.pushes(), [Publication::Pushed { .. }]));
    assert!(!git(&f.remote, &["ls-tree", "--name-only", "main"]).contains("untracked"));
    assert!(matches!(result, RepositoryResult::Attention { .. }));
}
#[test]
fn disabled_and_behind_never_push() {
    let _language = i18n::using(i18n::Language::Chinese);
    let f = Fixture::create();
    f.commit("ahead", "ahead");
    let before = git(&f.remote, &["rev-parse", "main"]);
    let (result, _) = check::repository(
        &Git::default(),
        &f.repo,
        PushPolicy::Disabled,
        &Cancel::default(),
    );
    assert!(result.pushes().is_empty());
    assert_eq!(before, git(&f.remote, &["rev-parse", "main"]));
    f.advance();
    let (result, _) = check::repository(
        &Git::default(),
        &f.repo,
        PushPolicy::FastForward,
        &Cancel::default(),
    );
    assert!(result.pushes().is_empty());
    assert!(matches!(result, RepositoryResult::Attention { .. }));
}
#[test]
fn missing_remote_branch_and_stale_refspec_are_distinct() {
    let _language = i18n::using(i18n::Language::Chinese);
    let f = Fixture::create();
    git(&f.remote, &["update-ref", "-d", "refs/heads/main"]);
    let (result, _) = check::repository(
        &Git::default(),
        &f.repo,
        PushPolicy::Disabled,
        &Cancel::default(),
    );
    match result {
        RepositoryResult::Attention { issues, .. } => {
            assert!(issues.iter().any(|i| i.kind == IssueKind::MissingUpstream))
        }
        other => panic!("{other:?}"),
    }
}
#[test]
fn linked_worktrees_share_one_scheduling_group() {
    let _language = i18n::using(i18n::Language::Chinese);
    let f = Fixture::create();
    let linked = f.temp.path().join("linked");
    git(
        &f.repo,
        &["worktree", "add", "-b", "linked", linked.to_str().unwrap()],
    );
    let groups = group(&[f.repo.clone(), fs::canonicalize(linked).unwrap()]);
    assert!(groups.failures.is_empty());
    assert_eq!(groups.jobs.len(), 1);
    assert_eq!(groups.jobs[0].len(), 2);
}
#[test]
fn report_order_failure_isolation_and_cancellation_preserve_history() {
    let _language = i18n::using(i18n::Language::Chinese);
    let first = Fixture::create();
    let second = Fixture::create();
    let home = tempfile::tempdir().unwrap();
    let store = Store {
        home: home.path().to_owned(),
    };
    let missing = PathBuf::from("/path/to/unavailable/repository");
    let paths = vec![second.repo.clone(), missing.clone(), first.repo.clone()];
    store
        .save_config(&Config {
            interval_seconds: 21600,
            repositories: paths.clone(),
            excluded_roots: vec![],
            push_policy: PushPolicy::Disabled,
            max_concurrency: 2,
        })
        .unwrap();
    let service = Service::new(store.home.clone(), Manager::new().unwrap());
    let report = service.run(&Cancel::default(), |_| {}).unwrap();
    assert_eq!(
        report
            .repositories
            .iter()
            .map(|r| r.path().to_owned())
            .collect::<Vec<_>>(),
        paths
    );
    assert!(matches!(
        report.repositories[1],
        RepositoryResult::Failed { .. }
    ));
    let saved = fs::read(store.state_dir().join("latest.json")).unwrap();
    let cancel = Cancel::default();
    cancel.cancel();
    assert!(service.run(&cancel, |_| {}).is_err());
    assert_eq!(
        saved,
        fs::read(store.state_dir().join("latest.json")).unwrap()
    );
}
#[test]
fn git_transport_failures_do_not_become_clean() {
    let _language = i18n::using(i18n::Language::Chinese);
    let f = Fixture::create();
    git(
        &f.repo,
        &["remote", "set-url", "origin", "/path/to/missing/remote.git"],
    );
    let (result, _) = check::repository(
        &Git::default(),
        &f.repo,
        PushPolicy::Disabled,
        &Cancel::default(),
    );
    assert!(matches!(result, RepositoryResult::Failed { .. }));
}
#[test]
fn failed_pre_push_hook_is_preserved() {
    let _language = i18n::using(i18n::Language::Chinese);
    use std::os::unix::fs::PermissionsExt;
    let f = Fixture::create();
    f.commit("ahead", "ahead");
    let hook = f.repo.join(".git/hooks/pre-push");
    write(&hook, "#!/bin/sh\nprintf rejected >&2\nexit 1\n");
    fs::set_permissions(hook, fs::Permissions::from_mode(0o700)).unwrap();
    let before = git(&f.remote, &["rev-parse", "main"]);
    let (result, _) = check::repository(
        &Git::default(),
        &f.repo,
        PushPolicy::FastForward,
        &Cancel::default(),
    );
    assert!(matches!(result.pushes(), [Publication::Failed { .. }]));
    assert_eq!(before, git(&f.remote, &["rev-parse", "main"]));
}

#[test]
fn independent_repositories_overlap_real_fetches() {
    let _language = i18n::using(i18n::Language::Chinese);
    use std::os::unix::fs::PermissionsExt;
    let first = Fixture::create();
    let second = Fixture::create();
    let control = tempfile::tempdir().unwrap();
    let arrivals = control.path().join("arrivals");
    fs::create_dir(&arrivals).unwrap();
    let wrapper = control.path().join("upload-pack");
    write(
        &wrapper,
        &format!(
            "#!/bin/sh\ntouch '{}/'$$\nfor i in $(seq 1 300); do count=$(ls '{}' | wc -l); [ \"$count\" -ge 2 ] && break; sleep 0.01; done\n[ \"$count\" -ge 2 ] || exit 99\nexec /usr/bin/git-upload-pack \"$@\"\n",
            arrivals.display(),
            arrivals.display()
        ),
    );
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o700)).unwrap();
    for fixture in [&first, &second] {
        git(
            &fixture.repo,
            &[
                "config",
                "remote.origin.uploadpack",
                wrapper.to_str().unwrap(),
            ],
        );
    }
    let home = tempfile::tempdir().unwrap();
    let store = Store {
        home: home.path().to_owned(),
    };
    store
        .save_config(&Config {
            interval_seconds: 21600,
            repositories: vec![first.repo.clone(), second.repo.clone()],
            excluded_roots: vec![],
            push_policy: PushPolicy::Disabled,
            max_concurrency: 2,
        })
        .unwrap();
    let service = Service::new(store.home.clone(), Manager::new().unwrap());
    let report = service.run(&Cancel::default(), |_| {}).unwrap();
    assert!(
        report
            .repositories
            .iter()
            .all(|r| matches!(r, RepositoryResult::Clean { .. })),
        "{:?}",
        report.repositories
    );
}
