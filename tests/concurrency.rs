use engine::{
    domain::PushPolicy,
    state::{Config, Store},
};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};
use test_support::{Fixture, git, write};

struct Gate {
    started: PathBuf,
    release: PathBuf,
}
impl Gate {
    fn install(f: &Fixture) -> Self {
        let wrapper = f.temp.path().join("gated-upload-pack");
        write(
            &wrapper,
            "#!/bin/sh\ntouch \"$0.started\"\nfor i in $(seq 1 1000); do\n if [ -e \"$0.release\" ]; then exec /usr/bin/git-upload-pack \"$@\"; fi\n sleep 0.01\ndone\nexit 98\n",
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
        Self {
            started: wrapper.with_file_name("gated-upload-pack.started"),
            release: wrapper.with_file_name("gated-upload-pack.release"),
        }
    }
    fn open(&self) {
        write(&self.release, "release");
    }
}
impl Drop for Gate {
    fn drop(&mut self) {
        self.open();
    }
}
struct Process(Child);
impl Process {
    fn wait(&mut self) {
        let deadline = Instant::now() + Duration::from_secs(10);
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
    let first_gate = Gate::install(&first);
    let second_gate = Gate::install(&second);
    let home = tempfile::tempdir().unwrap();
    let store = Store::custom(home.path().to_owned());
    store
        .save_config(&Config {
            interval_seconds: 21600,
            schedule_anchor: Default::default(),
            repositories: vec![first.repo.clone(), second.repo.clone()],
            excluded_roots: vec![],
            push_policy: PushPolicy::Disabled,
            max_concurrency: 4,
            check_on_startup: false,
            chinese_command_trigger: true,
            interaction_mode: Default::default(),
            mouse: Default::default(),
        })
        .unwrap();
    let spawn = |repo: &PathBuf| {
        Process(
            Command::new(env!("CARGO_BIN_EXE_app"))
                .env("APP_LANG", "en")
                .env("HOME", home.path())
                .arg("--home")
                .arg(home.path())
                .arg("check")
                .arg(repo)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap(),
        )
    };
    let mut first_process = spawn(&first.repo);
    let mut second_process = spawn(&second.repo);
    let deadline = Instant::now() + Duration::from_secs(8);
    while !first_gate.started.exists() || !second_gate.started.exists() {
        assert!(
            Instant::now() < deadline,
            "independent checks did not overlap"
        );
        thread::sleep(Duration::from_millis(5));
    }
    let configured = Command::new(env!("CARGO_BIN_EXE_app"))
        .env("APP_LANG", "en")
        .env("HOME", home.path())
        .arg("--home")
        .arg(home.path())
        .args(["interval", "12h"])
        .output()
        .unwrap();
    assert!(configured.status.success());
    assert_eq!(store.config().unwrap().interval_seconds, 43200);
    assert!(first_process.0.try_wait().unwrap().is_none());
    assert!(second_process.0.try_wait().unwrap().is_none());
    first_gate.open();
    second_gate.open();
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
