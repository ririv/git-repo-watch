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
            schedule_anchor: Default::default(),
            repositories: vec![],
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
fn old_state_requires_manual_conversion_and_cli_leaves_it_intact() {
    let (_home, store) = fixture();
    let config = fs::read(store.config_path()).unwrap();
    fs::create_dir(store.state_dir()).unwrap();
    let path = store.state_dir().join("latest.json");
    let bytes = br#"{"schema_version":3}"#;
    fs::write(&path, bytes).unwrap();

    for args in [["report", "--json"], ["status", "--json"]] {
        let blocked = cli(&store, &args);
        assert_eq!(blocked.status.code(), Some(2));
        assert!(String::from_utf8_lossy(&blocked.stderr).contains("manual conversion"));
    }
    assert_eq!(cli(&store, &["migrate"]).status.code(), Some(2));
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert_eq!(fs::read(store.config_path()).unwrap(), config);
    assert!(!store.database_path().exists());
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
