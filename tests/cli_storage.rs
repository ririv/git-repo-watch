use engine::{
    domain::PushPolicy,
    state::{Config, Store},
};
use std::{
    fs,
    process::{Command, Output},
};

fn fixture() -> (tempfile::TempDir, Store) {
    let home = tempfile::tempdir().unwrap();
    let store = Store::custom(home.path().to_owned());
    store
        .save_config(&Config {
            interval_seconds: 21600,
            repositories: vec![],
            excluded_roots: vec![],
            push_policy: PushPolicy::Disabled,
            max_concurrency: 4,
            check_on_startup: false,
        })
        .unwrap();
    (home, store)
}
fn cli(store: &Store, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_app"))
        .env("APP_LANG", "en")
        .env("HOME", store.config_dir())
        .arg("--home")
        .arg(store.config_dir())
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn explicit_migration_preserves_public_report_and_config_and_can_be_repeated() {
    let (_home, store) = fixture();
    let config = fs::read(store.config_path()).unwrap();
    let report = serde_json::json!({
        "schema_version":3,
        "started_at":"2026-10-22T12:00:00Z",
        "finished_at":"2026-10-22T12:01:00Z",
        "push_policy":"disabled", "notifications":[],
        "status":"unconfigured", "repositories":[]
    });
    fs::create_dir(store.state_dir()).unwrap();
    let bytes = serde_json::to_vec_pretty(&report).unwrap();
    fs::write(store.state_dir().join("latest.json"), &bytes).unwrap();
    let blocked = cli(&store, &["report", "--json"]);
    assert_eq!(blocked.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&blocked.stderr).contains("migrate"));
    assert!(!store.database_path().exists());
    for (expected, files) in [("imported", Some(1)), ("ready", None)] {
        let output = cli(&store, &["migrate"]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["status"], expected);
        assert_eq!(value.get("files").and_then(|v| v.as_u64()), files);
        assert_eq!(
            fs::read(store.state_dir().join("legacy-json/latest.json")).unwrap(),
            bytes
        );
        assert_eq!(fs::read(store.config_path()).unwrap(), config);
        let output = cli(&store, &["report", "--json"]);
        assert!(output.status.success());
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap(),
            report
        );
    }
    assert!(
        String::from_utf8(cli(&store, &["help"]).stdout)
            .unwrap()
            .contains("migrate")
    );
}

#[test]
fn malformed_legacy_data_stays_available_and_does_not_become_an_empty_report() {
    let (_home, store) = fixture();
    fs::create_dir(store.state_dir()).unwrap();
    let path = store.state_dir().join("repository-checks.json");
    fs::write(&path, b"broken JSON").unwrap();
    assert_eq!(cli(&store, &["migrate"]).status.code(), Some(2));
    assert_eq!(cli(&store, &["status", "--json"]).status.code(), Some(2));
    assert_eq!(fs::read(&path).unwrap(), b"broken JSON");
    assert!(!store.state_dir().join("legacy-json").exists());
}

#[test]
fn configuration_conversion_is_explicit_preserves_json_output_and_does_not_create_a_database() {
    let (_home, store) = fixture();
    let before = cli(&store, &["config"]);
    assert!(before.status.success());
    let value: serde_json::Value = serde_json::from_slice(&before.stdout).unwrap();
    fs::write(
        store.legacy_config_path(),
        serde_json::to_vec_pretty(&value).unwrap(),
    )
    .unwrap();
    fs::remove_file(store.config_path()).unwrap();
    let blocked = cli(&store, &["config"]);
    assert_eq!(blocked.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&blocked.stderr).contains("migrate config"));
    for expected in ["converted", "ready"] {
        let output = cli(&store, &["migrate", "config"]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()["status"],
            expected
        );
        assert!(!store.database_path().exists());
        assert!(!store.legacy_config_path().exists());
        let after = cli(&store, &["config"]);
        assert!(after.status.success());
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&after.stdout).unwrap(),
            value
        );
    }
    let original = fs::read(store.config_path()).unwrap();
    let mut conflicting = value;
    conflicting["interval_seconds"] = 43200.into();
    let json = serde_json::to_vec(&conflicting).unwrap();
    fs::write(store.legacy_config_path(), &json).unwrap();
    let rejected = cli(&store, &["migrate", "config"]);
    assert_eq!(rejected.status.code(), Some(2));
    assert_eq!(fs::read(store.config_path()).unwrap(), original);
    assert_eq!(fs::read(store.legacy_config_path()).unwrap(), json);
    assert!(!store.database_path().exists());
}
