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
    let store = Store::custom(home.path().to_owned());
    let missing = PathBuf::from("/path/to/unavailable/repository");
    let paths = vec![second.repo.clone(), missing.clone(), first.repo.clone()];
    store
        .save_config(&Config {
            interval_seconds: 21600,
            schedule_anchor: Default::default(),
            repositories: paths.clone(),
            excluded_roots: vec![],
            push_policy: PushPolicy::Disabled,
            max_concurrency: 2,
            check_on_startup: true,
            chinese_command_trigger: true,
            mouse: Default::default(),
        })
        .unwrap();
    let service = Service::new(store.clone(), Manager::new().unwrap());
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
    let saved = serde_json::to_vec(&store.report().unwrap()).unwrap();
    let cancel = Cancel::default();
    cancel.cancel();
    assert!(service.run(&cancel, |_| {}).is_err());
    assert_eq!(saved, serde_json::to_vec(&store.report().unwrap()).unwrap());
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
    let store = Store::custom(home.path().to_owned());
    store
        .save_config(&Config {
            interval_seconds: 21600,
            schedule_anchor: Default::default(),
            repositories: vec![first.repo.clone(), second.repo.clone()],
            excluded_roots: vec![],
            push_policy: PushPolicy::Disabled,
            max_concurrency: 2,
            check_on_startup: true,
            chinese_command_trigger: true,
            mouse: Default::default(),
        })
        .unwrap();
    let service = Service::new(store.clone(), Manager::new().unwrap());
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

#[test]
fn manual_checks_never_push_and_scheduled_checks_apply_configured_policy() {
    let first = Fixture::create();
    let second = Fixture::create();
    let home = tempfile::tempdir().unwrap();
    let store = Store::custom(home.path().to_owned());
    store
        .save_config(&Config {
            interval_seconds: 21600,
            schedule_anchor: Default::default(),
            repositories: vec![first.repo.clone(), second.repo.clone()],
            excluded_roots: vec![],
            push_policy: PushPolicy::FastForward,
            max_concurrency: 4,
            check_on_startup: true,
            chinese_command_trigger: true,
            mouse: Default::default(),
        })
        .unwrap();
    let service = Service::new(store.clone(), Manager::new().unwrap());
    service.run(&Cancel::default(), |_| {}).unwrap();
    let baseline = serde_json::to_vec(&store.report().unwrap()).unwrap();
    first.commit("first-ahead", "first");
    second.commit("second-ahead", "second");
    let first_before = git(&first.remote, &["rev-parse", "main"]);
    let untouched = git(&second.remote, &["rev-parse", "main"]);
    let mut checked = vec![];
    let report = service
        .check_repository(&first.repo, &Cancel::default(), |event| {
            if let engine::service::Event::Checking(path) = event {
                checked.push(path)
            }
        })
        .unwrap();
    assert_eq!(checked, vec![first.repo.clone()]);
    assert_eq!(report.repositories.len(), 1);
    assert_eq!(report.push_policy, PushPolicy::Disabled);
    assert!(report.repositories[0].pushes().is_empty());
    assert_eq!(git(&first.remote, &["rev-parse", "main"]), first_before);
    assert_eq!(git(&second.remote, &["rev-parse", "main"]), untouched);
    assert_eq!(
        serde_json::to_vec(&store.report().unwrap()).unwrap(),
        baseline
    );
    assert_eq!(report.execution.unwrap().max_concurrency, 1);
    let saved = store.repository_reports().unwrap();
    assert_eq!(saved.len(), 2);
    assert!(saved.contains_key(&first.repo));
    let saved_bytes = serde_json::to_vec(&store.repository_reports().unwrap()).unwrap();
    let cancel = Cancel::default();
    cancel.cancel();
    assert!(
        service
            .check_repository(&first.repo, &cancel, |_| {})
            .is_err()
    );
    assert!(
        service
            .check_repository(
                std::path::Path::new("/unregistered/repository"),
                &Cancel::default(),
                |_| {}
            )
            .is_err()
    );
    assert_eq!(
        serde_json::to_vec(&store.repository_reports().unwrap()).unwrap(),
        saved_bytes
    );
    let manual = service.run(&Cancel::default(), |_| {}).unwrap();
    assert_eq!(manual.push_policy, PushPolicy::Disabled);
    assert!(manual.repositories.iter().all(|r| r.pushes().is_empty()));
    assert_eq!(git(&first.remote, &["rev-parse", "main"]), first_before);
    assert_eq!(git(&second.remote, &["rev-parse", "main"]), untouched);
    assert_eq!(store.config().unwrap().push_policy, PushPolicy::FastForward);
    let scheduled = service.run_scheduled(&Cancel::default(), |_| {}).unwrap();
    assert_eq!(scheduled.push_policy, PushPolicy::FastForward);
    assert!(
        scheduled
            .repositories
            .iter()
            .all(|r| matches!(r.pushes(), [Publication::Pushed { .. }]))
    );
    for f in [&first, &second] {
        assert_eq!(
            git(&f.remote, &["rev-parse", "main"]),
            git(&f.repo, &["rev-parse", "main"])
        );
    }
}

#[test]
fn file_reads_remain_available_while_repository_writes_wait_for_the_same_git_storage() {
    let f = Fixture::create();
    write(f.repo.join("new"), "new");
    let home = tempfile::tempdir().unwrap();
    let store = Store::custom(home.path().to_owned());
    store
        .save_config(&Config {
            interval_seconds: 21600,
            schedule_anchor: Default::default(),
            repositories: vec![f.repo.clone()],
            excluded_roots: vec![],
            push_policy: PushPolicy::Disabled,
            max_concurrency: 4,
            check_on_startup: true,
            chinese_command_trigger: true,
            mouse: Default::default(),
        })
        .unwrap();
    let service = Service::new(store.clone(), Manager::new().unwrap());
    let common = engine::local::common_dir(&engine::local::open(&f.repo).unwrap()).unwrap();
    let _lock = store.repository_lock(&common, &Cancel::default()).unwrap();
    let files = service.files(&f.repo, &Cancel::default()).unwrap();
    let cancel = Cancel::default();
    let worker_cancel = cancel.clone();
    let worker = std::thread::spawn(move || {
        service.edit_index(
            &files,
            engine::local::files::IndexAction::Stage,
            &[],
            &worker_cancel,
        )
    });
    std::thread::sleep(std::time::Duration::from_millis(50));
    assert!(!worker.is_finished());
    cancel.cancel();
    assert!(
        worker
            .join()
            .unwrap()
            .unwrap_err()
            .downcast_ref::<engine::cancel::Cancelled>()
            .is_some()
    );
    assert!(git(&f.repo, &["diff", "--cached", "--name-only"]).is_empty());
}

#[test]
fn corrupt_single_repository_cache_is_rejected_before_checking() {
    let f = Fixture::create();
    f.commit("ahead", "ahead");
    let before = git(&f.remote, &["rev-parse", "main"]);
    let home = tempfile::tempdir().unwrap();
    let store = Store::custom(home.path().to_owned());
    store
        .save_config(&Config {
            interval_seconds: 21600,
            schedule_anchor: Default::default(),
            repositories: vec![f.repo.clone()],
            excluded_roots: vec![],
            push_policy: PushPolicy::FastForward,
            max_concurrency: 4,
            check_on_startup: true,
            chinese_command_trigger: true,
            mouse: Default::default(),
        })
        .unwrap();
    {
        let _lock = store.lock().unwrap();
        write(store.state_dir().join("repository-checks.json"), "broken");
    }
    let service = Service::new(store, Manager::new().unwrap());
    assert!(
        service
            .check_repository(&f.repo, &Cancel::default(), |_| {})
            .is_err()
    );
    assert_eq!(git(&f.remote, &["rev-parse", "main"]), before);
}
