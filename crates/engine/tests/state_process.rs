use engine::{
    cancel::Cancel,
    domain::PushPolicy,
    process,
    state::{self, Config, Store},
};
use std::{
    ffi::OsString,
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    thread,
    time::{Duration, Instant},
};
#[test]
fn missing_optional_preferences_and_explicit_values_survive_toml_roundtrips() {
    let _language = i18n::using(i18n::Language::Chinese);
    let temp = tempfile::tempdir().unwrap();
    let store = Store::custom(temp.path().to_owned());
    let old = "interval_seconds = 21600\nrepositories = []\nexcluded_roots = []\npush_policy = \"disabled\"\n";
    fs::write(store.config_path(), old).unwrap();
    let mut config = store.config().unwrap();
    assert_eq!(config.max_concurrency, 4);
    let serialized = serde_json::to_value(&config).unwrap();
    assert!(serialized.get("max_concurrency").is_none());
    config.max_concurrency = 2;
    store.save_config(&config).unwrap();
    assert_eq!(store.config().unwrap().max_concurrency, 2);
    let permissions = fs::metadata(store.config_path())
        .unwrap()
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(permissions, 0o600);
    for value in [0, 33] {
        config.max_concurrency = value;
        assert!(config.validate().is_err());
    }
}
#[test]
fn process_lock_is_shared_and_automatically_released() {
    let _language = i18n::using(i18n::Language::Chinese);
    let temp = tempfile::tempdir().unwrap();
    let store = Store::custom(temp.path().to_owned());
    assert!(!store.running().unwrap());
    let lock = store.batch_lock().unwrap();
    assert!(store.running().unwrap());
    assert!(store.batch_lock().is_err());
    assert!(store.lock().is_ok());
    drop(lock);
    assert!(!store.running().unwrap());
}
#[test]
fn cancellation_and_timeout_kill_descendants() {
    let _language = i18n::using(i18n::Language::Chinese);
    for timeout in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let sentinel = temp.path().join("sentinel");
        let cancel = Cancel::default();
        let task_cancel = cancel.clone();
        let script = format!("(sleep 1; touch '{}') & wait", sentinel.display());
        let started = Instant::now();
        let handle = thread::spawn(move || {
            process::run(
                Path::new("/bin/sh"),
                &[OsString::from("-c"), script.into()],
                None,
                &task_cancel,
                if timeout {
                    Duration::from_millis(100)
                } else {
                    Duration::from_secs(5)
                },
            )
        });
        if !timeout {
            thread::sleep(Duration::from_millis(100));
            cancel.cancel();
        }
        assert!(handle.join().unwrap().is_err());
        assert!(started.elapsed() < Duration::from_secs(1));
        thread::sleep(Duration::from_millis(1100));
        assert!(!sentinel.exists());
    }
}
#[test]
fn intervals_and_schema_boundaries() {
    let _language = i18n::using(i18n::Language::Chinese);
    for (value, want) in [
        ("6h", 21600),
        ("30m", 1800),
        ("1d", 86400),
        ("60s", 60),
        ("1.5h", 5400),
    ] {
        assert_eq!(state::interval(value).unwrap(), want)
    }
    assert!(state::interval("1s").is_err());
    assert!(state::interval("NaNh").is_err());
    let c = Config {
        interval_seconds: 21600,
        schedule_anchor: Default::default(),
        repositories: vec![],
        scheduled_actions: engine::workflow::Plan::from_policy(PushPolicy::Disabled),
        max_concurrency: 32,
        fetch_on_startup: false,
        auto_fetch: Default::default(),
        chinese_command_trigger: true,
        text_wrap: false,
        file_icons: true,
        interaction_mode: Default::default(),
        view: Default::default(),
        mouse: Default::default(),
        editor: Default::default(),
        ai: Default::default(),
    };
    assert!(c.validate().is_ok());
}
