use gw_core::{
    cancel::Cancel,
    domain::PushPolicy,
    scheduler::{self, Manager},
    state::{Config, Store},
};
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
};
fn fixture() -> (tempfile::TempDir, Store) {
    let temp = tempfile::tempdir().unwrap();
    let store = Store {
        home: temp.path().to_owned(),
    };
    store
        .save_config(&Config {
            interval_seconds: 21600,
            repositories: vec![PathBuf::from("/path/to/unavailable/repository")],
            excluded_roots: vec![],
            push_policy: PushPolicy::Disabled,
            max_concurrency: 4,
        })
        .unwrap();
    (temp, store)
}
fn cli(store: &Store, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_gw"))
        .arg("--home")
        .arg(&store.home)
        .args(args)
        .output()
        .unwrap()
}
#[test]
fn status_and_default_tui_never_check_or_fallback() {
    let (_temp, store) = fixture();
    let out = cli(&store, &["status", "--json"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["repository_count"], 1);
    assert_eq!(value["max_concurrency"], 4);
    assert_eq!(value["report"]["status"], "missing");
    for args in [&[][..], &["tui"][..]] {
        let out = cli(&store, args);
        assert_eq!(out.status.code(), Some(2));
        assert!(String::from_utf8_lossy(&out.stderr).contains("交互式终端"));
        assert!(!store.state_dir().join("latest.json").exists());
    }
    assert_eq!(cli(&store, &["report"]).status.code(), Some(1));
}
#[test]
fn config_commands_are_explicit_and_do_not_run_checks() {
    let (_temp, store) = fixture();
    for args in [
        &["interval", "12h"][..],
        &["concurrency", "2"][..],
        &["autopush", "fast_forward"][..],
    ] {
        let out = cli(&store, args);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    let config = store.config().unwrap();
    assert_eq!(config.interval_seconds, 43200);
    assert_eq!(config.max_concurrency, 2);
    assert_eq!(config.push_policy, PushPolicy::FastForward);
    assert!(!store.state_dir().join("latest.json").exists());
    assert_eq!(
        cli(&store, &["report", "--problems", "--pushes"])
            .status
            .code(),
        Some(2)
    );
    assert_eq!(
        cli(&store, &["status", "unexpected"]).status.code(),
        Some(2)
    );
}
#[test]
fn malformed_and_missing_reports_are_distinct() {
    let (_temp, store) = fixture();
    assert_eq!(cli(&store, &["report"]).status.code(), Some(1));
    fs::create_dir(store.state_dir()).unwrap();
    fs::write(store.state_dir().join("latest.json"), b"broken").unwrap();
    assert_eq!(cli(&store, &["report"]).status.code(), Some(2));
    assert_eq!(cli(&store, &["status", "--json"]).status.code(), Some(2));
}
#[test]
fn help_is_available_without_installation() {
    let out = Command::new(env!("CARGO_BIN_EXE_gw"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("concurrency"));
}
#[test]
fn plist_escapes_paths_preserves_schedule_and_disables_run_at_load() {
    let payload = scheduler::plist(
        std::path::Path::new("/path/中文 & folder"),
        std::path::Path::new("/path/bin/gw"),
        21600,
    )
    .unwrap();
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("job.plist");
    fs::write(&file, payload).unwrap();
    let output = Command::new("/usr/bin/plutil")
        .args(["-convert", "json", "-o", "-"])
        .arg(&file)
        .output()
        .unwrap();
    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["Label"], "local.git-repo-watch");
    assert_eq!(value["StartInterval"], 21600);
    assert_eq!(value["RunAtLoad"], false);
    assert_eq!(value["ProgramArguments"][2], "/path/中文 & folder");
    assert_eq!(value["ProgramArguments"][3], "check");
}
#[test]
fn installation_refuses_unowned_binary_before_mutation() {
    let temp = tempfile::tempdir().unwrap();
    let manager = Manager {
        user_home: temp.path().to_owned(),
        uid: unsafe { libc::getuid() },
    };
    fs::create_dir_all(manager.binary().parent().unwrap()).unwrap();
    fs::write(manager.binary(), b"not gw").unwrap();
    assert!(
        manager
            .install(
                temp.path(),
                &std::env::current_exe().unwrap(),
                21600,
                &Cancel::default()
            )
            .is_err()
    );
    assert_eq!(fs::read(manager.binary()).unwrap(), b"not gw");
    assert!(!manager.plist().exists());
}
