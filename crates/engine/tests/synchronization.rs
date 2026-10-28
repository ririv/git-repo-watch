use engine::{
    cancel::Cancel,
    check,
    domain::{IssueKind, PushPolicy, RepositoryResult, synchronization::Synchronization},
    scheduler::Manager,
    service::Service,
    state::{Config, Store},
    transport::Git,
};
use test_support::*;

#[test]
fn comparisons_count_branches_and_preserve_sync_after_successful_pushes() {
    let f = Fixture::create();
    git(&f.repo, &["branch", "second"]);
    git(
        &f.repo,
        &["branch", "--set-upstream-to=origin/main", "second"],
    );
    git(&f.repo, &["commit", "--allow-empty", "-m", "ahead one"]);
    git(&f.repo, &["commit", "--allow-empty", "-m", "ahead two"]);
    git(&f.repo, &["branch", "--force", "second", "main"]);
    let (result, _) = check::local_repository(&f.repo, &Cancel::default());
    assert_eq!(result.synchronization().counts(), Some((2, 0)));
    let (pushed, _) = check::repository(
        &Git::default(),
        &f.repo,
        PushPolicy::FastForward,
        &Cancel::default(),
    );
    assert_eq!(pushed.synchronization().counts(), Some((0, 0)));
    assert_eq!(pushed.pushes().len(), 1);
    f.advance();
    let (behind, _) = check::repository(
        &Git::default(),
        &f.repo,
        PushPolicy::Disabled,
        &Cancel::default(),
    );
    assert_eq!(behind.synchronization().counts(), Some((0, 2)));
    git(&f.repo, &["commit", "--allow-empty", "-m", "diverged"]);
    let (diverged, _) = check::local_repository(&f.repo, &Cancel::default());
    assert_eq!(diverged.synchronization().counts(), Some((1, 2)));
}

#[test]
fn missing_tracking_and_legacy_records_are_explicitly_distinct() {
    let f = Fixture::create();
    git(&f.repo, &["branch", "--unset-upstream"]);
    let (result, _) = check::local_repository(&f.repo, &Cancel::default());
    assert!(
        matches!(result.synchronization(),Synchronization::Known { branches } if branches.is_empty())
    );
    let old:RepositoryResult=serde_json::from_value(serde_json::json!({"status":"clean","path":"/example/repo","branches_without_upstream":[],"pushes":[]})).unwrap();
    assert_eq!(old.synchronization(), &Synchronization::Unknown);
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
        report.repositories[0].synchronization().counts(),
        Some((1, 0))
    );
    let next = service.run(&Cancel::default(), |_| {}).unwrap();
    assert!(
        matches!(&next.repositories[0],RepositoryResult::Attention { issues,.. } if issues.iter().all(|issue|issue.kind!=IssueKind::OperationFailed))
    );
}
