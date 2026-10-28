use engine::{
    cancel::Cancel,
    domain::{Publication, PushPolicy, RepositoryResult},
    local::guard::Guard,
    operations::{SkipReason, mutation::Change},
    scheduler::Manager,
    service::{ConfigEdit, Service},
    state::{Config, Store},
    tasks::{Outcome, Source, State},
    workflow::{Plan, Step, StepOutcome, Trigger},
};
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};
use test_support::*;

fn service(paths: Vec<PathBuf>, startup: Plan) -> (tempfile::TempDir, Service) {
    let home = tempfile::tempdir().unwrap();
    let store = Store::custom(home.path().to_owned());
    store
        .save_config(&Config {
            repositories: paths,
            startup_actions: startup,
            ..Config::default()
        })
        .unwrap();
    let manager = Manager::for_user(home.path().to_owned(), unsafe { libc::getuid() }).unwrap();
    (home, Service::new(store, manager))
}
fn plan(text: &str) -> Plan {
    text.parse().unwrap()
}
fn steps(report: &engine::domain::Report, index: usize) -> Vec<Step> {
    report.execution.as_ref().unwrap().repositories[index]
        .steps
        .iter()
        .map(|record| record.step)
        .collect()
}

#[test]
fn repositories_without_remotes_skip_remote_steps_and_finish_local_inspection() {
    for committed in [false, true] {
        for workflow in [
            "fetch inspect",
            "fetch pull push inspect",
            "pull push inspect",
        ] {
            let temp = tempfile::tempdir().unwrap();
            let repo = fs::canonicalize(temp.path()).unwrap().join("本地仓库");
            init(&repo);
            if committed {
                git(&repo, &["commit", "--allow-empty", "-m", "initial"]);
            }
            write(repo.join("文件 空格.txt"), "staged\n");
            git(&repo, &["add", "."]);
            write(repo.join("文件 空格.txt"), "unstaged\n");
            let before = Guard::capture(&repo, &Cancel::default()).unwrap();
            let index = fs::read(repo.join(".git/index")).unwrap();
            let (_home, service) = service(vec![repo.clone()], plan(workflow));
            service
                .configure(ConfigEdit::Workflow {
                    trigger: Trigger::Scheduled,
                    plan: plan(workflow),
                })
                .unwrap();
            for report in [
                service.run_startup(&Cancel::default(), |_| {}).unwrap(),
                service.run_scheduled(&Cancel::default(), |_| {}).unwrap(),
            ] {
                let result = &report.repositories[0];
                assert!(
                    !matches!(result, RepositoryResult::Failed { .. }),
                    "{result:?}"
                );
                assert!(matches!(
                    result.synchronization(),
                    engine::domain::synchronization::Synchronization::Current {
                        checkout: engine::domain::synchronization::Checkout::NoUpstream { .. }
                            | engine::domain::synchronization::Checkout::Unborn { .. }
                    }
                ));
                let records = &report.execution.as_ref().unwrap().repositories[0].steps;
                assert_eq!(records.len(), plan(workflow).steps().len());
                for record in records {
                    match record.step {
                        Step::Inspect => {
                            assert!(matches!(record.outcome, StepOutcome::Succeeded { .. }))
                        }
                        _ => assert!(matches!(
                            record.outcome,
                            StepOutcome::Skipped {
                                reason: SkipReason::NoRemotes
                            }
                        )),
                    }
                }
            }
            let saved = service.store.repository_report(&repo).unwrap().unwrap();
            assert!(matches!(
                saved.execution.unwrap().repositories[0].steps[0].outcome,
                StepOutcome::Skipped {
                    reason: SkipReason::NoRemotes
                }
            ));
            for task in service.store.tasks(10).unwrap() {
                assert!(
                    matches!(&task.state, State::Finished { outcome: Outcome::Succeeded { detail }, .. } if detail.contains(SkipReason::NoRemotes.label())),
                    "{task:?}"
                );
            }
            before.validate(&repo, &Cancel::default()).unwrap();
            assert_eq!(fs::read(repo.join(".git/index")).unwrap(), index);
            assert_eq!(
                fs::read_to_string(repo.join("文件 空格.txt")).unwrap(),
                "unstaged\n"
            );
        }
    }
}

#[test]
fn local_upstream_pull_still_runs_without_configured_remotes() {
    let f = Fixture::create();
    git(&f.repo, &["remote", "remove", "origin"]);
    git(&f.repo, &["checkout", "-b", "local-source"]);
    f.commit("source", "local upstream\n");
    let target = git(&f.repo, &["rev-parse", "HEAD"]);
    git(&f.repo, &["checkout", "main"]);
    git(
        &f.repo,
        &["branch", "--set-upstream-to=local-source", "main"],
    );
    let (_home, service) = service(vec![f.repo.clone()], plan("fetch pull push inspect"));
    let report = service.run_startup(&Cancel::default(), |_| {}).unwrap();
    assert!(
        !matches!(report.repositories[0], RepositoryResult::Failed { .. }),
        "{report:?}"
    );
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), target);
    let records = &report.execution.unwrap().repositories[0].steps;
    assert!(matches!(records[0].outcome, StepOutcome::Skipped { .. }));
    assert!(matches!(records[1].outcome, StepOutcome::Succeeded { .. }));
    assert!(matches!(records[2].outcome, StepOutcome::Skipped { .. }));
}

#[test]
fn skipped_fetch_is_successful_and_adding_a_remote_restores_fetching() {
    let f = Fixture::create();
    git(&f.repo, &["remote", "remove", "origin"]);
    let (_home, service) = service(vec![f.repo.clone()], Plan::fetch());
    assert!(matches!(
        service.fetch(&f.repo, &Cancel::default()).unwrap(),
        Change::FetchSkipped {
            reason: SkipReason::NoRemotes
        }
    ));
    let saved = service.store.repository_report(&f.repo).unwrap().unwrap();
    assert!(!matches!(
        saved.repositories[0],
        RepositoryResult::Failed { .. }
    ));
    git(
        &f.repo,
        &["remote", "add", "origin", f.remote.to_str().unwrap()],
    );
    f.advance();
    assert!(
        matches!(service.fetch(&f.repo, &Cancel::default()).unwrap(), Change::Fetched { remotes } if remotes == ["origin"])
    );
    assert_eq!(
        git(&f.repo, &["rev-parse", "origin/main"]),
        git(&f.remote, &["rev-parse", "main"])
    );
}

#[test]
fn configured_remote_failures_and_excluded_remotes_are_not_reported_as_absent() {
    for excluded in [false, true] {
        let f = Fixture::create();
        if excluded {
            git(&f.repo, &["config", "remote.origin.skipFetchAll", "true"]);
        } else {
            git(
                &f.repo,
                &[
                    "remote",
                    "set-url",
                    "origin",
                    f.temp.path().join("missing.git").to_str().unwrap(),
                ],
            );
        }
        let (_home, service) = service(vec![f.repo.clone()], Plan::fetch());
        let report = service.run_startup(&Cancel::default(), |_| {}).unwrap();
        assert!(matches!(
            report.repositories[0],
            RepositoryResult::Failed { .. }
        ));
        assert_eq!(steps(&report, 0), [Step::Fetch]);
        assert!(matches!(
            report.execution.unwrap().repositories[0].steps[0].outcome,
            StepOutcome::Failed { .. }
        ));
        assert!(service.fetch(&f.repo, &Cancel::default()).is_err());
    }
}

#[test]
fn startup_and_scheduled_plans_roundtrip_independently_and_preserve_legacy_policy() {
    let (_home, service) = service(vec![], Plan::local());
    service
        .configure(ConfigEdit::Workflow {
            trigger: Trigger::Scheduled,
            plan: plan("pull push inspect"),
        })
        .unwrap();
    let config = service.store.config().unwrap();
    assert_eq!(config.startup_actions, Plan::local());
    assert_eq!(config.scheduled_actions, plan("pull push inspect"));
    let text = fs::read_to_string(service.store.config_path()).unwrap();
    assert!(text.contains("startup_actions = [\"inspect\"]"), "{text}");
    assert!(!text.contains("push_policy ="));
    for policy in [PushPolicy::Disabled, PushPolicy::FastForward] {
        let legacy = format!(
            "# 用户注释\ninterval_seconds = 21600\nrepositories = []\nexcluded_roots = []\npush_policy = \"{policy}\" # 推送偏好\ncheck_on_startup = false\n"
        );
        fs::write(service.store.config_path(), &legacy).unwrap();
        let config = service.store.config().unwrap();
        assert!(!config.check_on_startup);
        assert_eq!(config.startup_actions, Plan::fetch());
        assert_eq!(config.scheduled_actions, Plan::from_policy(policy));
        assert_eq!(
            fs::read_to_string(service.store.config_path()).unwrap(),
            legacy
        );
        service.store.save_config(&config).unwrap();
        let text = fs::read_to_string(service.store.config_path()).unwrap();
        assert!(text.contains("# 用户注释") && text.contains("# 推送偏好"));
        assert!(!text.contains("push_policy ="));
        assert_eq!(service.store.config().unwrap(), config);
    }
}

#[test]
fn local_startup_reads_dirty_files_without_contacting_remote_or_changing_the_index() {
    let f = Fixture::create();
    f.commit("文件 空格.txt", "base\n");
    write(f.repo.join("文件 空格.txt"), "staged\n");
    git(&f.repo, &["add", "."]);
    write(f.repo.join("文件 空格.txt"), "unstaged\n");
    write(f.repo.join("untracked"), "private\n");
    let marker = f.temp.path().join("network");
    let wrapper = f.temp.path().join("upload-pack");
    write(
        &wrapper,
        &format!("#!/bin/sh\ntouch '{}'\nexit 99\n", marker.display()),
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
    let (_home, service) = service(vec![f.repo.clone()], Plan::local());
    let before = Guard::capture(&f.repo, &Cancel::default()).unwrap();
    let index = fs::read(f.repo.join(".git/index")).unwrap();
    let report = service.run_startup(&Cancel::default(), |_| {}).unwrap();
    assert!(matches!(
        report.repositories[0],
        RepositoryResult::Attention { .. }
    ));
    assert_eq!(steps(&report, 0), vec![Step::Inspect]);
    before.validate(&f.repo, &Cancel::default()).unwrap();
    assert_eq!(fs::read(f.repo.join(".git/index")).unwrap(), index);
    assert!(!marker.exists());
    let records = service.store.tasks(10).unwrap();
    assert_eq!(records[0].spec.source, Source::Startup);
    assert!(records[0].spec.title.contains(Step::Inspect.label()));
}

#[test]
fn startup_fetch_only_updates_tracking_refs_even_when_remote_history_is_rewound() {
    let f = Fixture::create();
    let old = git(&f.repo, &["rev-parse", "HEAD"]);
    f.commit("source", "new source\n");
    git(&f.repo, &["push"]);
    write(f.repo.join("source"), "unsaved source\n");
    git(&f.remote, &["update-ref", "refs/heads/main", &old]);
    let (_home, service) = service(vec![f.repo.clone()], Plan::fetch());
    let before = Guard::capture(&f.repo, &Cancel::default()).unwrap();
    let report = service.run_startup(&Cancel::default(), |_| {}).unwrap();
    before.validate(&f.repo, &Cancel::default()).unwrap();
    assert_eq!(git(&f.repo, &["rev-parse", "origin/main"]), old);
    assert_eq!(steps(&report, 0), vec![Step::Fetch, Step::Inspect]);
    assert!(report.repositories[0].pushes().is_empty());
}

#[test]
fn explicit_pull_fast_forwards_and_records_the_step_result() {
    let f = Fixture::create();
    f.advance();
    let (_home, service) = service(vec![f.repo.clone()], plan("pull inspect"));
    let old = git(&f.repo, &["rev-parse", "HEAD"]);
    let report = service.run_startup(&Cancel::default(), |_| {}).unwrap();
    assert_ne!(git(&f.repo, &["rev-parse", "HEAD"]), old);
    assert_eq!(
        git(&f.repo, &["rev-parse", "HEAD"]),
        git(&f.remote, &["rev-parse", "main"])
    );
    assert_eq!(steps(&report, 0), vec![Step::Pull, Step::Inspect]);
    let records = service.store.tasks(10).unwrap();
    let State::Finished {
        outcome: Outcome::Succeeded { detail },
        ..
    } = &records[0].state
    else {
        panic!("{:?}", records[0]);
    };
    assert!(detail.contains(Step::Pull.label()), "{detail}");
}

#[test]
fn failed_pull_preserves_dirty_files_stops_push_and_does_not_block_another_repository() {
    let first = Fixture::create();
    first.commit("source", "committed\n");
    write(first.repo.join("source"), "keep this\n");
    let second = Fixture::create();
    second.advance();
    let (_home, service) = service(
        vec![first.repo.clone(), second.repo.clone()],
        plan("pull push inspect"),
    );
    let before = Guard::capture(&first.repo, &Cancel::default()).unwrap();
    let remote = git(&first.remote, &["rev-parse", "main"]);
    let report = service.run_startup(&Cancel::default(), |_| {}).unwrap();
    before.validate(&first.repo, &Cancel::default()).unwrap();
    assert_eq!(remote, git(&first.remote, &["rev-parse", "main"]));
    assert!(matches!(
        report.repositories[0],
        RepositoryResult::Failed { .. }
    ));
    assert!(matches!(
        report.repositories[1],
        RepositoryResult::Clean { .. }
    ));
    assert_eq!(steps(&report, 0), vec![Step::Pull]);
    assert!(matches!(
        report.execution.as_ref().unwrap().repositories[0].steps[0].outcome,
        StepOutcome::Failed { .. }
    ));
    assert_eq!(
        steps(&report, 1),
        vec![Step::Pull, Step::Push, Step::Inspect]
    );
    assert_eq!(
        git(&second.repo, &["rev-parse", "HEAD"]),
        git(&second.remote, &["rev-parse", "main"])
    );
}

#[test]
fn scheduled_and_startup_execution_use_only_their_own_saved_steps() {
    let f = Fixture::create();
    f.commit("ahead", "committed\n");
    write(f.repo.join("private"), "not committed\n");
    let (_home, service) = service(vec![f.repo.clone()], Plan::local());
    service
        .configure(ConfigEdit::Workflow {
            trigger: Trigger::Scheduled,
            plan: plan("fetch push inspect"),
        })
        .unwrap();
    let before = Guard::capture(&f.repo, &Cancel::default()).unwrap();
    let startup = service.run_startup(&Cancel::default(), |_| {}).unwrap();
    assert!(startup.repositories[0].pushes().is_empty());
    let scheduled = service.run_scheduled(&Cancel::default(), |_| {}).unwrap();
    assert!(matches!(
        scheduled.repositories[0].pushes(),
        [Publication::Pushed { .. }]
    ));
    before.validate(&f.repo, &Cancel::default()).unwrap();
    assert_eq!(
        steps(&scheduled, 0),
        vec![Step::Fetch, Step::Push, Step::Inspect]
    );
    assert!(!git(&f.remote, &["ls-tree", "--name-only", "main"]).contains("private"));
}

#[test]
fn external_plan_input_reports_unknown_duplicate_and_dependency_errors() {
    for input in [
        "",
        "fetch",
        "push inspect",
        "fetch fetch inspect",
        "restore inspect",
        "inspect fetch inspect",
    ] {
        assert!(input.parse::<Plan>().is_err(), "{input}");
    }
    assert_eq!(
        plan("pull fetch push inspect").tokens(),
        "pull fetch push inspect"
    );
    assert!(serde_json::from_str::<Plan>("[\"fetch\", \"reset\", \"inspect\"]").is_err());
}

#[test]
fn explicit_configuration_migration_preserves_behavior_and_rejects_ambiguous_fields() {
    use engine::state::ConfigurationMigration;
    let home = tempfile::tempdir().unwrap();
    let store = Store::custom(home.path().to_owned());
    let legacy = "# 保留注释\ninterval_seconds = 21600\nrepositories = []\nexcluded_roots = []\npush_policy = 'fast_forward'\ncheck_on_startup = false\n";
    fs::write(store.config_path(), legacy).unwrap();
    let expected = store.config().unwrap();
    assert!(matches!(
        store.migrate_configuration(&Cancel::default()).unwrap(),
        ConfigurationMigration::Converted
    ));
    assert_eq!(store.config().unwrap(), expected);
    assert_eq!(expected.scheduled_actions, plan("fetch push inspect"));
    let text = fs::read_to_string(store.config_path()).unwrap();
    assert!(!text.contains("push_policy =") && text.contains("# 保留注释"));
    assert!(matches!(
        store.migrate_configuration(&Cancel::default()).unwrap(),
        ConfigurationMigration::Ready
    ));
    assert_eq!(fs::read_to_string(store.config_path()).unwrap(), text);
    let conflicting = legacy.to_owned() + "scheduled_actions = ['inspect']\n";
    fs::write(store.config_path(), &conflicting).unwrap();
    assert!(store.migrate_configuration(&Cancel::default()).is_err());
    assert_eq!(
        fs::read_to_string(store.config_path()).unwrap(),
        conflicting
    );
    fs::remove_file(store.config_path()).unwrap();
    let mut json = serde_json::to_value(Config::default()).unwrap();
    json.as_object_mut().unwrap().remove("startup_actions");
    json.as_object_mut().unwrap().remove("scheduled_actions");
    json["push_policy"] = serde_json::json!("fast_forward");
    fs::write(
        store.legacy_config_path(),
        serde_json::to_vec(&json).unwrap(),
    )
    .unwrap();
    store.migrate_configuration(&Cancel::default()).unwrap();
    assert_eq!(
        store.config().unwrap().scheduled_actions,
        plan("fetch push inspect")
    );
    assert_eq!(store.config().unwrap().startup_actions, Plan::fetch());
    assert!(!store.legacy_config_path().exists());
    assert!(store.report().unwrap().is_none());
}

#[test]
fn fetch_then_pull_reuses_the_remote_refresh() {
    let f = Fixture::create();
    f.advance();
    let wrapper = f.temp.path().join("upload-pack");
    write(
        &wrapper,
        "#!/bin/sh\nprintf 'fetch\\n' >> \"$0.log\"\nexec /usr/bin/git-upload-pack \"$@\"\n",
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
    let (_home, service) = service(vec![f.repo.clone()], plan("fetch pull inspect"));
    let report = service.run_startup(&Cancel::default(), |_| {}).unwrap();
    assert!(matches!(
        report.repositories[0],
        RepositoryResult::Clean { .. }
    ));
    assert_eq!(
        fs::read_to_string(wrapper.with_extension("log"))
            .unwrap()
            .lines()
            .count(),
        1
    );
}

#[test]
fn cancellation_after_pull_retains_the_completed_write_and_the_step_history() {
    let f = Fixture::create();
    f.advance();
    let wrapper = f.temp.path().join("upload-pack");
    let blocked = f.temp.path().join("blocked");
    write(wrapper.with_extension("count"), "0\n");
    write(
        &wrapper,
        &format!(
            "#!/bin/sh\nn=$(cat \"$0.count\")\nn=$((n+1))\necho \"$n\" > \"$0.count\"\nif [ \"$n\" = 2 ]; then touch '{}'; sleep 30; fi\nexec /usr/bin/git-upload-pack \"$@\"\n",
            blocked.display()
        ),
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
    let (_home, service) = service(vec![f.repo.clone()], plan("pull fetch inspect"));
    let worker = service.clone();
    let cancel = Cancel::default();
    let worker_cancel = cancel.clone();
    let handle = std::thread::spawn(move || worker.run_startup(&worker_cancel, |_| {}));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !blocked.exists() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(blocked.exists());
    cancel.cancel();
    assert!(handle.join().unwrap().is_err());
    assert_eq!(
        git(&f.repo, &["rev-parse", "HEAD"]),
        git(&f.remote, &["rev-parse", "main"])
    );
    let reports = service.store.repository_reports().unwrap();
    let saved = &reports[&f.repo];
    assert_eq!(steps(saved, 0), vec![Step::Pull, Step::Fetch]);
    let records = service.store.tasks(10).unwrap();
    let State::Finished {
        outcome: Outcome::Cancelled { detail },
        ..
    } = &records[0].state
    else {
        panic!("{:?}", records[0]);
    };
    assert!(
        detail.contains(Step::Pull.label()) && detail.contains(i18n::tr!(WorkflowStopped)),
        "{detail}"
    );
}
