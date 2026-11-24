#[cfg(unix)]
use engine::operations::mutation::Change;
use engine::{
    cancel::Cancel,
    domain::{IssueKind, Publication, RepositoryResult},
    operations::{
        mutation::{Plan, Request},
        sync::{self, Unavailable},
    },
    scheduler::Manager,
    service::Service,
    state::{Completion, Config, Operation, Store},
};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{fs, path::PathBuf};
use test_support::{Fixture, git, write};

fn service(paths: Vec<PathBuf>) -> (tempfile::TempDir, Service) {
    let home = tempfile::tempdir().unwrap();
    let store = Store::custom(home.path().into());
    store
        .save_config(&Config {
            repositories: paths,
            ..Config::default()
        })
        .unwrap();
    let manager = Manager::for_current_user_in(home.path().into()).unwrap();
    (home, Service::new(store, manager))
}
fn prepare(service: &Service, fixture: &Fixture) -> Plan {
    service
        .prepare_mutation(&fixture.repo, Request::Sync, &Cancel::default())
        .unwrap()
}
#[cfg(unix)]
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
#[cfg(unix)]
fn requests(log: &PathBuf) -> usize {
    fs::read_to_string(log).unwrap().lines().count()
}

#[cfg(unix)]
#[test]
fn sync_reviews_a_fast_forward_after_one_fetch_and_updates_only_its_repository() {
    let f = Fixture::create();
    let other = Fixture::create();
    let (_home, s) = service(vec![f.repo.clone(), other.repo.clone()]);
    let log = upload_log(&f);
    f.advance();
    let before = git(&f.repo, &["rev-parse", "HEAD"]);
    let remote = git(&f.remote, &["rev-parse", "main"]);
    let plan = prepare(&s, &f);
    assert!(matches!(&plan, Plan::Sync(sync::Plan::Pull(_))));
    assert!(plan.notice().is_none());
    assert_eq!(requests(&log), 1);
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), before);
    assert_eq!(git(&f.remote, &["rev-parse", "main"]), remote);
    assert!(matches!(
        s.mutate(&plan, &Cancel::default()).unwrap(),
        Change::Synchronized {
            result: sync::Change::Pulled { .. }
        }
    ));
    assert_eq!(requests(&log), 1);
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), remote);
    assert!(s.store.repository_report(&other.repo).unwrap().is_none());
    assert_eq!(
        s.store
            .repository_report(&f.repo)
            .unwrap()
            .unwrap()
            .repositories[0]
            .synchronization()
            .current_commits(),
        Some((0, 0))
    );
    let current = prepare(&s, &f);
    assert!(matches!(current, Plan::Sync(sync::Plan::Current { .. })));
    assert_eq!(current.notice(), Some(i18n::tr!(SyncCurrent)));
    assert_eq!(requests(&log), 2);
}

#[cfg(unix)]
#[test]
fn sync_push_keeps_uncommitted_work_and_verifies_the_remote_without_refetching() {
    let f = Fixture::create();
    f.commit("tracked", "committed\n");
    write(f.repo.join("tracked"), "unstaged\n");
    write(f.repo.join("new"), "staged\n");
    git(&f.repo, &["add", "new"]);
    let files = git(&f.repo, &["status", "--porcelain"]);
    let head = git(&f.repo, &["rev-parse", "HEAD"]);
    let remote = git(&f.remote, &["rev-parse", "main"]);
    let (_home, s) = service(vec![f.repo.clone()]);
    let log = upload_log(&f);
    let plan = prepare(&s, &f);
    assert!(matches!(&plan, Plan::Sync(sync::Plan::Push(_))));
    assert_eq!(requests(&log), 2);
    assert_eq!(git(&f.remote, &["rev-parse", "main"]), remote);
    s.mutate(&plan, &Cancel::default()).unwrap();
    assert_eq!(requests(&log), 4);
    assert_eq!(git(&f.remote, &["rev-parse", "main"]), head);
    assert_eq!(git(&f.repo, &["status", "--porcelain"]), files);
    let report = s.store.repository_report(&f.repo).unwrap().unwrap();
    assert!(matches!(
        report.repositories[0].pushes(),
        [Publication::Pushed { .. }]
    ));
    assert_eq!(
        report.repositories[0].synchronization().current_commits(),
        Some((0, 0))
    );
}

#[test]
fn sync_refuses_divergence_dirty_pulls_and_stale_reviews_without_merging() {
    let f = Fixture::create();
    f.commit("tracked", "base\n");
    git(&f.repo, &["push"]);
    f.advance();
    let (_home, s) = service(vec![f.repo.clone()]);
    write(f.repo.join("tracked"), "unfinished\n");
    assert!(
        s.prepare_mutation(&f.repo, Request::Sync, &Cancel::default())
            .is_err()
    );
    assert_eq!(
        fs::read_to_string(f.repo.join("tracked")).unwrap(),
        "unfinished\n"
    );
    git(&f.repo, &["restore", "tracked"]);
    let pull = prepare(&s, &f);
    git(&f.repo, &["commit", "--allow-empty", "-m", "local ahead"]);
    let head = git(&f.repo, &["rev-parse", "HEAD"]);
    assert!(s.mutate(&pull, &Cancel::default()).is_err());
    let error = s
        .prepare_mutation(&f.repo, Request::Sync, &Cancel::default())
        .unwrap_err();
    assert!(
        error.to_string().contains(i18n::tr!(SyncDiverged)),
        "{error:#}"
    );
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), head);
    assert!(!f.repo.join(".git/MERGE_HEAD").exists());
}

#[test]
fn sync_failed_push_retains_publication_failure_and_refreshes_actual_state() {
    let f = Fixture::create();
    f.commit("ahead", "local\n");
    let (_home, s) = service(vec![f.repo.clone()]);
    let plan = prepare(&s, &f);
    let hook = f.remote.join("hooks/pre-receive");
    test_support::write_git_hook(&hook, "#!/bin/sh\necho rejected-sync >&2\nexit 1\n");

    let error = s.mutate(&plan, &Cancel::default()).unwrap_err();
    assert!(format!("{error:#}").contains("rejected-sync"));
    let report = s.store.repository_report(&f.repo).unwrap().unwrap();
    assert!(matches!(
        report.repositories[0].pushes(),
        [Publication::Failed { .. }]
    ));
    assert!(
        matches!(&report.repositories[0], RepositoryResult::Attention { issues, .. } if issues.iter().any(|issue| issue.kind == IssueKind::PushFailed))
    );
    let history = s.store.operation_history(&f.repo, 5).unwrap();
    assert!(matches!(
        history[0].record.operation,
        Operation::Push {
            publication: Publication::Failed { .. }
        }
    ));
}

#[test]
fn sync_push_reviews_reject_changed_heads_remote_tips_and_upstream_configuration() {
    #[derive(Debug)]
    enum Edited {
        Head,
        Remote,
        Upstream,
    }
    for change in [Edited::Head, Edited::Remote, Edited::Upstream] {
        let f = Fixture::create();
        f.commit("ahead", "outgoing\n");
        let (_home, s) = service(vec![f.repo.clone()]);
        let plan = prepare(&s, &f);
        match change {
            Edited::Head => {
                git(&f.repo, &["commit", "--allow-empty", "-m", "new head"]);
            }
            Edited::Remote => f.advance(),
            Edited::Upstream => {
                git(
                    &f.repo,
                    &["config", "branch.main.merge", "refs/heads/other"],
                );
            }
        }
        let head = git(&f.repo, &["rev-parse", "HEAD"]);
        let remote = git(&f.remote, &["rev-parse", "main"]);
        assert!(s.mutate(&plan, &Cancel::default()).is_err(), "{change:?}");
        assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), head);
        assert_eq!(git(&f.remote, &["rev-parse", "main"]), remote);
    }
}

#[test]
fn sync_unavailable_states_are_explicit_successful_notices_without_configuring_or_publishing() {
    for reason in [
        Unavailable::NoRemotes,
        Unavailable::NoUpstream,
        Unavailable::LocalUpstream,
        Unavailable::Detached,
        Unavailable::Unborn,
    ] {
        let f = Fixture::create();
        match reason {
            Unavailable::NoRemotes => {
                git(&f.repo, &["remote", "remove", "origin"]);
            }
            Unavailable::NoUpstream => {
                git(&f.repo, &["branch", "--unset-upstream"]);
            }
            Unavailable::LocalUpstream => {
                git(&f.repo, &["branch", "local-upstream"]);
                git(&f.repo, &["branch", "--set-upstream-to=local-upstream"]);
            }
            Unavailable::Detached => {
                git(&f.repo, &["checkout", "--detach"]);
            }
            Unavailable::Unborn => {
                git(&f.repo, &["checkout", "--orphan", "empty"]);
            }
        }
        let (_home, s) = service(vec![f.repo.clone()]);
        let config = fs::read(f.repo.join(".git/config")).unwrap();
        let plan = prepare(&s, &f);
        assert!(
            matches!(plan, Plan::Sync(sync::Plan::Unavailable { reason: actual, .. }) if actual == reason)
        );
        assert_eq!(plan.notice(), Some(reason.label()));
        assert_eq!(fs::read(f.repo.join(".git/config")).unwrap(), config);
        assert!(matches!(
            s.store.operation_history(&f.repo, 1).unwrap()[0]
                .record
                .operation,
            Operation::Workspace {
                action: engine::operations::mutation::Action::PrepareSync,
                completion: Completion::Succeeded { .. }
            }
        ));
    }
}

#[test]
fn sync_honors_custom_refspecs_and_deleted_or_excluded_upstreams() {
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
    let (_home, s) = service(vec![f.repo.clone()]);
    let plan = prepare(&s, &f);
    assert!(
        matches!(&plan, Plan::Sync(sync::Plan::Pull(plan)) if plan.upstream() == "refs/remotes/custom/main")
    );
    s.mutate(&plan, &Cancel::default()).unwrap();
    git(&f.remote, &["update-ref", "-d", "refs/heads/main"]);
    assert!(
        s.prepare_mutation(&f.repo, Request::Sync, &Cancel::default())
            .is_err()
    );
    assert!(
        git2::Repository::open(&f.repo)
            .unwrap()
            .find_reference("refs/remotes/custom/main")
            .is_err()
    );
    let excluded = Fixture::create();
    git(
        &excluded.repo,
        &["config", "--add", "remote.origin.fetch", "^refs/heads/main"],
    );
    let (_home, s) = service(vec![excluded.repo.clone()]);
    assert!(
        s.prepare_mutation(&excluded.repo, Request::Sync, &Cancel::default())
            .is_err()
    );
}

#[test]
fn sync_progress_distinguishes_pull_push_and_failure_stages() {
    use engine::tasks::progress::Stage;
    let f = Fixture::create();
    f.advance();
    let (_home, s) = service(vec![f.repo.clone()]);
    let mut stages = Vec::new();
    let plan = s
        .prepare_mutation_observed(&f.repo, Request::Sync, &Cancel::default(), &mut |stage| {
            stages.push(stage);
            Ok(())
        })
        .unwrap();
    assert_eq!(
        stages,
        vec![
            Stage::WaitingRepository,
            Stage::WaitingCapacity,
            Stage::InspectingRepository,
            Stage::FetchingRemote,
            Stage::ComparingBranches,
            Stage::CheckingWorktree,
            Stage::PreparingPreview,
            Stage::RefreshingState
        ]
    );
    s.mutate(&plan, &Cancel::default()).unwrap();
    f.commit("outgoing", "committed\n");
    stages.clear();
    s.prepare_mutation_observed(&f.repo, Request::Sync, &Cancel::default(), &mut |stage| {
        stages.push(stage);
        Ok(())
    })
    .unwrap();
    assert!(stages.contains(&Stage::VerifyingRemote));
    assert!(!stages.contains(&Stage::CheckingWorktree));

    let dirty = Fixture::create();
    dirty.commit("tracked", "base\n");
    git(&dirty.repo, &["push"]);
    dirty.advance();
    write(dirty.repo.join("tracked"), "unfinished\n");
    let (_home, s) = service(vec![dirty.repo.clone()]);
    stages.clear();
    assert!(
        s.prepare_mutation_observed(
            &dirty.repo,
            Request::Sync,
            &Cancel::default(),
            &mut |stage| {
                stages.push(stage);
                Ok(())
            }
        )
        .is_err()
    );
    assert!(stages.contains(&Stage::CheckingWorktree));
    assert!(!stages.contains(&Stage::PreparingPreview));
    assert_eq!(stages.last(), Some(&Stage::RefreshingState));
    assert_eq!(
        fs::read_to_string(dirty.repo.join("tracked")).unwrap(),
        "unfinished\n"
    );
}

#[test]
fn failed_progress_reporting_does_not_skip_refreshing_fetched_references() {
    use engine::tasks::progress::Stage;
    let f = Fixture::create();
    f.advance();
    let (_home, s) = service(vec![f.repo.clone()]);
    let before = git(&f.repo, &["rev-parse", "HEAD"]);
    let error = s
        .prepare_mutation_observed(&f.repo, Request::Sync, &Cancel::default(), &mut |stage| {
            if stage == Stage::RefreshingState {
                anyhow::bail!("progress write failed")
            }
            Ok(())
        })
        .unwrap_err();
    assert!(error.to_string().contains("progress write failed"));
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), before);
    assert_eq!(
        s.store
            .repository_report(&f.repo)
            .unwrap()
            .unwrap()
            .repositories[0]
            .synchronization()
            .current_commits(),
        Some((0, 1))
    );
}
