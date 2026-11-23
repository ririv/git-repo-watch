use engine::{
    domain::PushPolicy,
    git_environment::{Environment, Launcher},
    state::{Config, Store},
    transport::Git,
};
use std::{
    path::PathBuf,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};
use test_support::{Fixture, GitGate, GitOperation};

struct Process(Child);
impl Process {
    fn wait(&mut self) {
        let deadline = Instant::now() + Git::default().timeout;
        loop {
            if let Some(status) = self.0.try_wait().unwrap() {
                assert!(status.success(), "check process failed: {status}");
                return;
            }
            assert!(Instant::now() < deadline, "check process did not exit");
            thread::sleep(Duration::from_millis(5));
        }
    }
}
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
#[test]
fn simultaneous_cli_checks_merge_results_and_continue_revision_numbers_across_processes() {
    let first = Fixture::create();
    let second = Fixture::create();
    let first_gate = GitGate::new(&first, GitOperation::Fetch);
    let second_gate = GitGate::new(&second, GitOperation::Fetch);
    let environment = |gate: &GitGate| {
        let (program, args) = gate.launcher();
        Environment {
            launcher: Some(Launcher { program, args }),
            ..Environment::default()
        }
    };
    let home = tempfile::tempdir().unwrap();
    let store = Store::custom(home.path().to_owned());
    store
        .save_config(&Config {
            interval_seconds: 21600,
            schedule_anchor: Default::default(),
            repository_environment: [
                (first.repo.clone(), environment(&first_gate)),
                (second.repo.clone(), environment(&second_gate)),
            ]
            .into(),
            repositories: vec![first.repo.clone(), second.repo.clone()],
            scheduled_actions: engine::workflow::Plan::from_policy(PushPolicy::Disabled),
            max_concurrency: 4,
            fetch_on_startup: false,
            auto_fetch: Default::default(),
            experimental_features: true,
            chinese_command_trigger: true,
            text_wrap: false,
            file_icons: true,
            interaction_mode: Default::default(),
            view: Default::default(),
            mouse: Default::default(),
            editor: Default::default(),
            ai: Default::default(),
        })
        .unwrap();
    let spawn = |repo: &PathBuf| {
        Process(
            Command::new(env!("CARGO_BIN_EXE_rvgit"))
                .env("APP_LANG", "en")
                .env("HOME", home.path())
                .arg("--home")
                .arg(home.path())
                .arg("fetch")
                .arg(repo)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        )
    };
    let mut first_process = spawn(&first.repo);
    let mut second_process = spawn(&second.repo);
    let deadline = Instant::now() + Git::default().timeout;
    while !first_gate.started.exists() || !second_gate.started.exists() {
        for (name, process) in [
            ("first", &mut first_process),
            ("second", &mut second_process),
        ] {
            if let Some(status) = process.0.try_wait().unwrap() {
                panic!("{name} check exited before both fetches started: {status}");
            }
        }
        assert!(
            Instant::now() < deadline,
            "independent checks did not overlap: first={}, second={}",
            first_gate.started.exists(),
            second_gate.started.exists(),
        );
        thread::sleep(Duration::from_millis(5));
    }
    let configured = Command::new(env!("CARGO_BIN_EXE_rvgit"))
        .env("APP_LANG", "en")
        .env("HOME", home.path())
        .arg("--home")
        .arg(home.path())
        .args(["startup-fetch", "true"])
        .output()
        .unwrap();
    assert!(
        configured.status.success(),
        "{}",
        String::from_utf8_lossy(&configured.stderr)
    );
    assert!(store.config().unwrap().fetch_on_startup);
    assert!(first_process.0.try_wait().unwrap().is_none());
    assert!(second_process.0.try_wait().unwrap().is_none());
    first_gate.release();
    second_gate.release();
    first_process.wait();
    second_process.wait();
    let reports = store.repository_reports().unwrap();
    assert_eq!(reports.len(), 2);
    assert_eq!(reports[&first.repo].repository_revisions[&first.repo], 1);
    assert_eq!(reports[&second.repo].repository_revisions[&second.repo], 1);
    let mut next = spawn(&first.repo);
    next.wait();
    let reports = store.repository_reports().unwrap();
    assert_eq!(reports[&first.repo].repository_revisions[&first.repo], 2);
    assert_eq!(reports[&second.repo].repository_revisions[&second.repo], 1);
    assert!(store.report().unwrap().is_none());
}
