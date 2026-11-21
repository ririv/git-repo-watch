use engine::{
    cancel::Cancel,
    check,
    domain::{
        IssueKind, PushPolicy, RepositoryResult,
        synchronization::{Checkout, RemoteBranch, RemoteComparison, Synchronization},
    },
    scheduler::Manager,
    service::Service,
    state::{Config, Store},
    transport::Git,
};
use test_support::*;

#[test]
fn current_branch_counts_commits_and_stays_correct_after_shared_upstream_pushes() {
    let f = Fixture::create();
    git(&f.repo, &["branch", "second"]);
    git(
        &f.repo,
        &["branch", "--set-upstream-to=origin/main", "second"],
    );
    git(&f.repo, &["commit", "--allow-empty", "-m", "ahead one"]);
    git(&f.repo, &["commit", "--allow-empty", "-m", "ahead two"]);
    git(&f.repo, &["commit", "--allow-empty", "-m", "ahead three"]);
    git(&f.repo, &["branch", "--force", "second", "main"]);
    let (result, _) = check::local_repository(&f.repo, &Cancel::default());
    assert_eq!(result.synchronization().current_commits(), Some((3, 0)));
    let (pushed, _) = check::repository(
        &Git::default(),
        &f.repo,
        PushPolicy::FastForward,
        &Cancel::default(),
    );
    assert_eq!(pushed.synchronization().current_commits(), Some((0, 0)));
    assert_eq!(pushed.pushes().len(), 1);
    let branches = pushed.synchronization().remote_branches().unwrap();
    assert_eq!(
        branches,
        ["main", "second"].map(|name| RemoteBranch {
            name: name.into(),
            comparison: RemoteComparison::Compared {
                ahead: 0,
                behind: 0
            },
        })
    );
    f.advance();
    let (behind, _) = check::repository(
        &Git::default(),
        &f.repo,
        PushPolicy::Disabled,
        &Cancel::default(),
    );
    assert_eq!(behind.synchronization().current_commits(), Some((0, 1)));
    git(&f.repo, &["commit", "--allow-empty", "-m", "diverged"]);
    let (diverged, _) = check::local_repository(&f.repo, &Cancel::default());
    assert_eq!(diverged.synchronization().current_commits(), Some((1, 1)));
}

#[test]
fn the_current_checkout_is_specific_to_each_worktree_and_other_branches_do_not_contribute() {
    let f = Fixture::create();
    git(
        &f.repo,
        &["checkout", "-b", "feature", "--track", "origin/main"],
    );
    for message in ["one", "two", "three"] {
        git(&f.repo, &["commit", "--allow-empty", "-m", message]);
    }
    let worktree = f.temp.path().join("独立 工作区");
    git(&f.repo, &["checkout", "main"]);
    git(
        &f.repo,
        &["worktree", "add", worktree.to_str().unwrap(), "feature"],
    );
    let (main, _) = check::local_repository(&f.repo, &Cancel::default());
    let (feature, _) = check::local_repository(&worktree, &Cancel::default());
    assert_eq!(main.synchronization().current_commits(), Some((0, 0)));
    assert_eq!(feature.synchronization().current_commits(), Some((3, 0)));
    assert_eq!(
        main.synchronization().remote_branches().unwrap(),
        [("feature", 3), ("main", 0)].map(|(name, ahead)| RemoteBranch {
            name: name.into(),
            comparison: RemoteComparison::Compared { ahead, behind: 0 },
        })
    );
    assert_eq!(
        main.synchronization().remote_branches(),
        feature.synchronization().remote_branches()
    );
    assert!(
        matches!(main.synchronization(), Synchronization::Current {checkout:Checkout::Compared {branch,..}, ..} if branch == "main")
    );
    assert!(
        matches!(feature.synchronization(), Synchronization::Current {checkout:Checkout::Compared {branch,..}, ..} if branch == "feature")
    );
    let snapshot = engine::local::branches::catalog::read(&f.repo, &Cancel::default()).unwrap();
    assert!(snapshot.entries.iter().any(|entry| entry.name == "feature"
        && matches!(
            entry.kind,
            engine::local::branches::catalog::Kind::Local(Ok(
                engine::local::branches::upstream::Tracking::Remote {
                    comparison: engine::local::branches::upstream::Comparison::Known {
                        ahead: 3,
                        behind: 0
                    },
                    ..
                }
            ))
        )));
}

#[test]
fn detached_unborn_and_missing_upstream_snapshots_do_not_reuse_another_branch() {
    let f = Fixture::create();
    git(&f.remote, &["update-ref", "-d", "refs/heads/main"]);
    let (missing, _) = check::repository(
        &Git::default(),
        &f.repo,
        PushPolicy::Disabled,
        &Cancel::default(),
    );
    assert!(
        matches!(missing.synchronization(), Synchronization::Current {checkout:Checkout::MissingUpstream {branch}, ..} if branch == "main")
    );
    assert_eq!(missing.synchronization().current_commits(), None);
    assert_eq!(
        missing.synchronization().remote_branches().unwrap(),
        [RemoteBranch {
            name: "main".into(),
            comparison: RemoteComparison::MissingUpstream
        },]
    );
    git(&f.repo, &["checkout", "--detach"]);
    let (detached, _) = check::local_repository(&f.repo, &Cancel::default());
    assert!(matches!(
        detached.synchronization(),
        Synchronization::Current {
            checkout: Checkout::Detached,
            ..
        }
    ));
    let empty = f.temp.path().join("empty");
    init(&empty);
    let (unborn, _) = check::local_repository(&empty, &Cancel::default());
    assert!(matches!(
        unborn.synchronization(),
        Synchronization::Current { checkout: Checkout::Unborn { branch }, .. } if branch == "main"
    ));
    assert_eq!(
        unborn.synchronization().remote_branches(),
        Some([].as_slice())
    );
}

#[test]
fn a_local_upstream_also_reports_commit_counts_for_the_checked_out_branch() {
    let f = Fixture::create();
    git(&f.repo, &["remote", "remove", "origin"]);
    git(&f.repo, &["branch", "base"]);
    git(&f.repo, &["branch", "--set-upstream-to=base", "main"]);
    for message in ["one", "two"] {
        git(&f.repo, &["commit", "--allow-empty", "-m", message]);
    }
    let (result, _) = check::local_repository(&f.repo, &Cancel::default());
    assert_eq!(result.synchronization().current_commits(), Some((2, 0)));
}

#[test]
fn missing_tracking_and_legacy_records_are_explicitly_distinct() {
    let f = Fixture::create();
    git(&f.repo, &["branch", "--unset-upstream"]);
    let (result, _) = check::local_repository(&f.repo, &Cancel::default());
    assert!(
        matches!(result.synchronization(),Synchronization::Current { checkout:Checkout::NoUpstream {branch}, .. } if branch == "main")
    );
    let old:RepositoryResult=serde_json::from_value(serde_json::json!({"status":"clean","path":"/example/repo","branches_without_upstream":[],"pushes":[]})).unwrap();
    assert_eq!(old.synchronization(), &Synchronization::Unknown);
    let previous: Synchronization = serde_json::from_value(serde_json::json!({
        "status": "current",
        "checkout": {"state": "compared", "branch": "main", "ahead": 2, "behind": 0}
    }))
    .unwrap();
    assert_eq!(previous.current_commits(), Some((2, 0)));
    assert_eq!(previous.remote_branches(), None);
    for branches in ["[]", "[\"ahead\",\"diverged\"]"] {
        let legacy: Synchronization =
            serde_json::from_str(&format!("{{\"status\":\"known\",\"branches\":{branches}}}"))
                .unwrap();
        assert!(matches!(legacy, Synchronization::LegacyBranches { .. }));
        assert_eq!(legacy.current_commits(), None);
    }
    assert_eq!(
        serde_json::from_value::<RepositoryResult>(serde_json::to_value(&result).unwrap())
            .unwrap()
            .synchronization(),
        result.synchronization()
    );
}

#[test]
fn failed_writes_persist_the_execution_error_and_a_successful_check_clears_it() {
    let f = Fixture::create();
    f.commit("tracked", "first\n");
    let home = tempfile::tempdir().unwrap();
    let store = Store::custom(home.path().into());
    store
        .save_config(&Config {
            repositories: vec![f.repo.clone()],
            ..Config::default()
        })
        .unwrap();
    let manager = Manager::for_user(home.path().into(), unsafe { libc::getuid() }).unwrap();
    let service = Service::new(store, manager);
    write(f.repo.join("tracked"), "second\n");
    git(&f.repo, &["add", "tracked"]);
    let plan = service.prepare_commit(&f.repo, &Cancel::default()).unwrap();
    assert!(service.commit(&plan, "", &Cancel::default()).is_err());
    let report = service.store.repository_report(&f.repo).unwrap().unwrap();
    assert!(
        matches!(&report.repositories[0],RepositoryResult::Attention { issues,.. } if issues.iter().any(|issue|issue.kind==IssueKind::OperationFailed))
    );
    assert_eq!(
        report.repositories[0].synchronization().current_commits(),
        Some((1, 0))
    );
    let next = service.run(&Cancel::default(), |_| {}).unwrap();
    assert!(
        matches!(&next.repositories[0],RepositoryResult::Attention { issues,.. } if issues.iter().all(|issue|issue.kind!=IssueKind::OperationFailed))
    );
}
