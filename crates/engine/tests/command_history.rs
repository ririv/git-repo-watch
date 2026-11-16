use engine::state::Store;
use std::{fs, os::unix::fs::PermissionsExt};

#[test]
fn command_history_is_private_bounded_deduplicated_and_isolated_per_profile() {
    let home = tempfile::tempdir().unwrap();
    let store = Store::custom(home.path().join("first"));
    assert!(store.command_history().unwrap().is_empty());
    for index in 0..130 {
        store.record_command(&format!("/test {index}")).unwrap();
    }
    store.record_command("  /test 12  ").unwrap();
    let reloaded = Store::custom(store.config_dir()).command_history().unwrap();
    assert_eq!(reloaded.len(), 128);
    assert_eq!(reloaded[0], "/test 12");
    assert_eq!(reloaded[1], "/test 129");
    assert!(!reloaded.contains(&"/test 1".into()));
    assert_eq!(
        reloaded
            .iter()
            .filter(|entry| entry.as_str() == "/test 12")
            .count(),
        1
    );
    assert!(
        Store::custom(home.path().join("second"))
            .command_history()
            .unwrap()
            .is_empty()
    );
    let path = store.state_dir().join("command-history.json");
    assert_eq!(
        fs::metadata(path).unwrap().permissions().mode() & 0o777,
        0o600
    );
}

#[test]
fn unreadable_history_is_reported_and_never_overwritten() {
    let home = tempfile::tempdir().unwrap();
    let store = Store::custom(home.path().into());
    store.record_command("/settings").unwrap();
    let path = store.state_dir().join("command-history.json");
    for data in ["not json", "[1]", "[\"invalid\"]"] {
        fs::write(&path, data).unwrap();
        assert!(store.command_history().is_err());
        assert!(store.record_command("/tasks").is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), data);
    }
}
