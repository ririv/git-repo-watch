use engine::{
    cancel::Cancel,
    state::{Config, ConfigurationMigration, Store},
};
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};
fn fixture() -> (tempfile::TempDir, Store) {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::custom(temp.path().to_owned());
    (temp, store)
}
#[test]
fn template_and_initial_configuration_have_all_explicit_defaults_and_private_permissions() {
    let (_temp, store) = fixture();
    store.initialize().unwrap();
    assert_eq!(store.config().unwrap(), Config::default());
    let text = fs::read_to_string(store.config_path()).unwrap();
    assert!(text.contains("# 定时检查间隔"));
    assert!(text.contains("max_concurrency = 4"));
    assert!(text.contains("check_on_startup = true"));
    assert_eq!(
        fs::metadata(store.config_path())
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    assert!(!store.legacy_config_path().exists());
}
#[test]
fn scalar_changes_keep_comments_literal_paths_and_unchanged_field_formatting() {
    let (_temp, store) = fixture();
    let text = r#"# 用户配置
interval_seconds = 21_600 # 原间隔
repositories = [
    '/example/中文 空格/one', # 仓库一
    '/example/a \\ path',
]
excluded_roots = []
push_policy = 'disabled'
max_concurrency = 4
check_on_startup = true # 启动检查
"#;
    fs::write(store.config_path(), text).unwrap();
    let mut config = store.config().unwrap();
    config.check_on_startup = false;
    store.save_config(&config).unwrap();
    let next = fs::read_to_string(store.config_path()).unwrap();
    assert_eq!(
        next,
        text.replace("check_on_startup = true", "check_on_startup = false")
    );
    assert_eq!(store.config().unwrap(), config);
    config
        .repositories
        .push(PathBuf::from("/example/new \"quoted\" 仓库"));
    store.save_config(&config).unwrap();
    let next = fs::read_to_string(store.config_path()).unwrap();
    assert!(
        next.contains("# 用户配置") && next.contains("# 仓库一") && next.contains("# 启动检查")
    );
    assert_eq!(store.config().unwrap(), config);
}
#[test]
fn invalid_or_unknown_toml_is_reported_without_replacing_the_file() {
    let (_temp, store) = fixture();
    for text in [
        "repositories = [",
        "interval_seconds = '6h'",
        "interval_seconds = 21600\nrepositories = []\nexcluded_roots = []\npush_policy = 'disabled'\ncheck_on_startp = false\n",
    ] {
        fs::write(store.config_path(), text).unwrap();
        assert!(store.config().is_err());
        assert!(store.save_config(&Config::default()).is_err());
        assert_eq!(fs::read_to_string(store.config_path()).unwrap(), text);
    }
}
#[test]
fn json_requires_explicit_conversion_and_values_survive_without_a_backup() {
    let (_temp, store) = fixture();
    let config = Config {
        repositories: vec!["/example/中文 空格".into()],
        max_concurrency: 8,
        check_on_startup: false,
        ..Config::default()
    };
    let json = serde_json::to_vec_pretty(&config).unwrap();
    fs::write(store.legacy_config_path(), &json).unwrap();
    assert!(store.config().is_err());
    assert!(store.initialize().is_err());
    assert!(store.save_config(&Config::default()).is_err());
    assert!(!store.config_path().exists());
    assert!(matches!(
        store.migrate_configuration(&Cancel::default()).unwrap(),
        ConfigurationMigration::Converted
    ));
    assert_eq!(store.config().unwrap(), config);
    assert!(!store.legacy_config_path().exists());
    assert!(matches!(
        store.migrate_configuration(&Cancel::default()).unwrap(),
        ConfigurationMigration::Ready
    ));
    assert!(fs::read_dir(store.config_dir()).unwrap().all(|item| {
        !item
            .unwrap()
            .file_name()
            .to_string_lossy()
            .ends_with(".bak")
    }));
}
#[test]
fn interrupted_conversion_resumes_only_for_equivalent_files_and_never_falls_back() {
    let (_temp, store) = fixture();
    store.save_config(&Config::default()).unwrap();
    let json = serde_json::to_vec(&Config::default()).unwrap();
    fs::write(store.legacy_config_path(), &json).unwrap();
    assert!(store.config().is_err());
    store.migrate_configuration(&Cancel::default()).unwrap();
    assert!(!store.legacy_config_path().exists());
    let original = fs::read(store.config_path()).unwrap();
    let other = Config {
        interval_seconds: 43200,
        ..Config::default()
    };
    let conflicting = serde_json::to_vec(&other).unwrap();
    fs::write(store.legacy_config_path(), &conflicting).unwrap();
    assert!(store.migrate_configuration(&Cancel::default()).is_err());
    assert_eq!(fs::read(store.config_path()).unwrap(), original);
    assert_eq!(fs::read(store.legacy_config_path()).unwrap(), conflicting);
    fs::write(store.config_path(), "invalid toml [").unwrap();
    fs::write(store.legacy_config_path(), &json).unwrap();
    assert!(store.migrate_configuration(&Cancel::default()).is_err());
    assert_eq!(fs::read(store.config_path()).unwrap(), b"invalid toml [");
    assert_eq!(fs::read(store.legacy_config_path()).unwrap(), json);
}
#[test]
fn malformed_json_and_cancelled_conversion_preserve_the_original() {
    let (_temp, store) = fixture();
    fs::write(store.legacy_config_path(), b"broken JSON").unwrap();
    assert!(store.migrate_configuration(&Cancel::default()).is_err());
    assert!(!store.config_path().exists());
    assert_eq!(
        fs::read(store.legacy_config_path()).unwrap(),
        b"broken JSON"
    );
    let cancel = Cancel::default();
    cancel.cancel();
    assert!(store.migrate_configuration(&cancel).is_err());
    assert!(!store.config_path().exists());
}
