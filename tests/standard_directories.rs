use engine::{directories::UserDirectories, state::Store};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn cli(home: &Path, cwd: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_app"))
        .env("HOME", home)
        .env("APP_LANG", "en")
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("XDG_STATE_HOME")
        .current_dir(cwd)
        .args(args)
        .output()
        .unwrap()
}
fn json(output: Output) -> serde_json::Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
#[test]
fn standard_configuration_and_database_ignore_cwd_and_obsolete_location_files() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("用户 目录");
    fs::create_dir(&home).unwrap();
    let cwd = temp.path().join("project");
    fs::create_dir(&cwd).unwrap();
    fs::write(cwd.join("config.json"), b"wrong project config").unwrap();
    let store = UserDirectories::defaults(&home).store();
    fs::create_dir_all(store.config_dir()).unwrap();
    let obsolete = store.config_dir().join("location.json");
    fs::write(&obsolete, b"invalid legacy metadata").unwrap();
    let paths = json(cli(&home, &cwd, &["paths"]));
    assert_eq!(
        PathBuf::from(paths["config"].as_str().unwrap()),
        store.config_path()
    );
    assert_eq!(
        PathBuf::from(paths["database"].as_str().unwrap()),
        store.database_path()
    );
    assert!(!store.config_path().exists());
    let initialized = json(cli(&home, &cwd, &["init"]));
    assert_eq!(
        initialized,
        serde_json::to_value(store.config_path()).unwrap()
    );
    let config = fs::read(store.config_path()).unwrap();
    assert_eq!(
        json(cli(&home, &cwd, &["config"]))["interval_seconds"],
        21600
    );
    assert_eq!(cli(&home, &cwd, &["init"]).status.code(), Some(2));
    assert_eq!(fs::read(store.config_path()).unwrap(), config);
    assert_eq!(cli(&home, &cwd, &["check"]).status.code(), Some(1));
    assert!(store.database_path().exists());
    assert!(store.report().unwrap().is_some());
    assert!(!cwd.join("var").exists());
    assert_eq!(fs::read(obsolete).unwrap(), b"invalid legacy metadata");
}
#[test]
fn explicit_home_keeps_its_existing_layout_and_is_independent_of_default_storage() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    let standard = UserDirectories::defaults(home).store();
    json(cli(home, home, &["init"]));
    let custom = Store::custom(home.join("独立 数据"));
    json(cli(home, home, &["--home", "~/独立 数据", "init"]));
    json(cli(home, home, &["--home", "~/独立 数据", "config"]));
    let output = cli(home, home, &["--home", "~/独立 数据", "interval", "12h"]);
    assert!(output.status.success());
    assert_eq!(custom.config().unwrap().interval_seconds, 43200);
    assert_eq!(standard.config().unwrap().interval_seconds, 21600);
    assert_eq!(
        cli(home, home, &["--home", "~/独立 数据", "check"])
            .status
            .code(),
        Some(1)
    );
    assert_eq!(
        custom.database_path(),
        home.join("独立 数据/var/state.sqlite3")
    );
    assert!(custom.database_path().exists());
    assert!(!standard.database_path().exists());
}
#[cfg(target_os = "linux")]
#[test]
fn linux_xdg_overrides_are_explicit_and_relative_values_fail_without_writes() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    let config = home.join("配置 空格");
    let state = home.join("状态 空格");
    let run = |value: &Path, args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_app"))
            .env("HOME", home)
            .env("APP_LANG", "en")
            .env("XDG_CONFIG_HOME", value)
            .env("XDG_STATE_HOME", &state)
            .args(args)
            .output()
            .unwrap()
    };
    let paths = json(run(&config, &["paths"]));
    assert_eq!(
        paths["config"],
        config
            .join(identity::DATA_DIRECTORY)
            .join("config.json")
            .to_str()
            .unwrap()
    );
    assert_eq!(
        paths["database"],
        state
            .join(identity::DATA_DIRECTORY)
            .join("state.sqlite3")
            .to_str()
            .unwrap()
    );
    json(run(&config, &["init"]));
    let bytes = fs::read(config.join(identity::DATA_DIRECTORY).join("config.json")).unwrap();
    assert_eq!(
        run(Path::new("relative"), &["config"]).status.code(),
        Some(2)
    );
    assert_eq!(
        fs::read(config.join(identity::DATA_DIRECTORY).join("config.json")).unwrap(),
        bytes
    );
}

#[test]
fn explicit_import_transfers_a_custom_home_and_normal_commands_survive_source_removal() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path();
    json(cli(home, home, &["--home", "~/旧 数据", "init"]));
    assert_eq!(
        cli(home, home, &["--home", "~/旧 数据", "check"])
            .status
            .code(),
        Some(1)
    );
    let source = Store::custom(home.join("旧 数据"));
    let source_report = serde_json::to_value(source.report().unwrap()).unwrap();
    let copied = json(cli(home, home, &["migrate", "--from", "~/旧 数据"]));
    assert_eq!(copied["status"], "database");
    assert!(source.config_path().exists());
    assert_eq!(
        cli(home, home, &["migrate", "--from", "~/旧 数据"])
            .status
            .code(),
        Some(2)
    );
    fs::remove_dir_all(home.join("旧 数据")).unwrap();
    assert_eq!(json(cli(home, home, &["report", "--json"])), source_report);
    assert_eq!(
        json(cli(home, home, &["config"]))["interval_seconds"],
        21600
    );
}
