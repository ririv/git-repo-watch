use engine::{
    domain::PushPolicy,
    state::{Config, Store},
};
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
};

struct Fixture {
    _temp: tempfile::TempDir,
    user_home: PathBuf,
    store: Store,
    repository: PathBuf,
}
impl Fixture {
    fn create() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let user_home = temp.path().join("用户 主目录");
        let store = Store::custom(user_home.join("应用 数据"));
        let repository = user_home.join("Proj/仓库 空格");
        fs::create_dir_all(store.config_dir()).unwrap();
        fs::create_dir_all(&repository).unwrap();
        assert!(
            Command::new("git")
                .args(["init", "-q"])
                .arg(&repository)
                .status()
                .unwrap()
                .success()
        );
        store
            .save_config(&Config {
                interval_seconds: 21600,
                schedule_anchor: Default::default(),
                repositories: vec![],
                excluded_roots: vec![],
                push_policy: PushPolicy::Disabled,
                max_concurrency: 4,
                check_on_startup: true,
                chinese_command_trigger: true,
                text_wrap: false,
                interaction_mode: Default::default(),
                mouse: Default::default(),
                ai: Default::default(),
            })
            .unwrap();
        Self {
            _temp: temp,
            user_home,
            store,
            repository,
        }
    }
    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_app"));
        command.env("APP_LANG", "en").env("HOME", &self.user_home);
        command
    }
    fn run(&self, args: &[&str]) -> Output {
        self.command()
            .args(["--home", "~/应用 数据"])
            .args(args)
            .output()
            .unwrap()
    }
    fn success(&self, args: &[&str]) {
        let result = self.run(args);
        assert!(
            result.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
}

#[test]
fn literal_tilde_paths_expand_for_cli_config_discovery_and_single_checks() {
    let f = Fixture::create();
    let canonical = fs::canonicalize(&f.repository).unwrap();
    f.success(&["add", "~/Proj/仓库 空格"]);
    assert_eq!(
        f.store.config().unwrap().repositories.as_slice(),
        std::slice::from_ref(&canonical)
    );
    f.success(&["exclude", "~/Proj"]);
    let config = f.store.config().unwrap();
    assert!(config.repositories.is_empty());
    assert_eq!(
        config.excluded_roots,
        [fs::canonicalize(f.user_home.join("Proj")).unwrap()]
    );
    f.success(&["unexclude", "~/Proj"]);
    f.success(&["scan", "~/Proj"]);
    assert!(f.store.config().unwrap().repositories.is_empty());
    f.success(&["import-tree", "~/Proj"]);
    assert_eq!(
        f.store.config().unwrap().repositories.as_slice(),
        std::slice::from_ref(&canonical)
    );
    assert!(f.store.report().unwrap().is_none());
    let checked = f.run(&["check", "~//Proj/仓库 空格"]);
    assert_eq!(
        checked.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&checked.stderr)
    );
    assert!(
        f.store
            .repository_reports()
            .unwrap()
            .contains_key(&canonical)
    );
    assert!(f.store.report().unwrap().is_none());
    f.success(&["remove", "~/Proj/仓库 空格"]);
    assert!(f.store.config().unwrap().repositories.is_empty());
}

#[test]
fn bare_home_selects_the_user_home_and_invalid_home_forms_do_not_mutate_configuration() {
    let f = Fixture::create();
    let original = fs::read(f.store.config_path()).unwrap();
    fs::copy(f.store.config_path(), f.user_home.join("config.toml")).unwrap();
    let output = f
        .command()
        .args(["--home", "~", "config"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    for command in [
        "add",
        "remove",
        "exclude",
        "unexclude",
        "scan",
        "import-tree",
        "check",
    ] {
        let output = f.run(&[command, "~someone/Proj"]);
        assert_eq!(output.status.code(), Some(2));
        assert!(String::from_utf8_lossy(&output.stderr).contains("~user"));
        assert_eq!(fs::read(f.store.config_path()).unwrap(), original);
    }
    for home in [None, Some("relative")] {
        let mut command = f.command();
        match home {
            None => {
                command.env_remove("HOME");
            }
            Some(home) => {
                command.env("HOME", home);
            }
        }
        let output = command
            .args(["--home", "~/应用 数据", "config"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(String::from_utf8_lossy(&output.stderr).contains("home directory"));
        assert_eq!(fs::read(f.store.config_path()).unwrap(), original);
    }
}
