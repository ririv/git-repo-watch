#![cfg(not(feature = "developer-scheduler"))]

use std::{fs, process::Command};

#[test]
fn release_exposes_manual_checks_and_preserves_existing_schedule_configuration() {
    let home = tempfile::tempdir().unwrap();
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_rvgit"))
            .args(["--home", home.path().to_str().unwrap()])
            .args(args)
            .env("APP_LANG", "en")
            .output()
            .unwrap()
    };
    let help = run(&["--help"]);
    let help = String::from_utf8(help.stdout).unwrap();
    assert!(help.starts_with("rvgit: Git repository"));
    assert!(help.contains("fetch [PATH]"));
    assert!(!help.contains("schedule"));

    for command in [
        "schedule",
        "scheduled-check",
        "scheduled-tick",
        "interval",
        "anchor",
        "autopush",
        "scheduled-actions",
        "uninstall",
    ] {
        let output = run(&[command]);
        assert_eq!(output.status.code(), Some(2), "{command}");
        assert!(String::from_utf8_lossy(&output.stderr).contains("Unknown command"));
    }

    let config = home.path().join("config.toml");
    assert!(run(&["init"]).status.success());
    let template = fs::read_to_string(&config).unwrap();
    assert!(!template.contains("scheduled_actions"));
    let scheduler_settings =
        "interval_seconds = 43200\nscheduled_actions = [\"fetch\", \"push\", \"inspect\"]\n";
    fs::write(&config, format!("{scheduler_settings}{template}")).unwrap();
    assert!(run(&["startup-fetch", "true"]).status.success());
    assert!(
        fs::read_to_string(&config)
            .unwrap()
            .starts_with(scheduler_settings)
    );

    let output = run(&["status", "--json"]);
    assert!(output.status.success());
    let status: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(status["fetch_on_startup"], true);
    assert!(status.get("registration").is_none());
    assert!(status.get("schedule").is_none());
    let output = run(&["config"]);
    assert!(output.status.success());
    let config: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(config.get("scheduled_actions").is_none());
}
