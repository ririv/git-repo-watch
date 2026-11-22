#[cfg(all(feature = "developer-scheduler", unix))]
use engine::{cancel::Cancel, scheduler::Manager};
use engine::{
    domain::PushPolicy,
    state::{Config, Store},
};
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
};
fn fixture() -> (tempfile::TempDir, Store) {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::custom(temp.path().to_owned());
    store
        .save_config(&Config {
            interval_seconds: 21600,
            schedule_anchor: Default::default(),
            repository_environment: Default::default(),
            repositories: vec![PathBuf::from("/path/to/unavailable/repository")],
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
    (temp, store)
}
fn cli(store: &Store, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_app"))
        .env("APP_LANG", "zh")
        .arg("--home")
        .arg(store.config_dir())
        .args(args)
        .output()
        .unwrap()
}
#[test]
fn status_and_noninteractive_tui_never_start_checks() {
    let _language = i18n::using(i18n::Language::Chinese);
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
    for args in [&[][..], &["tui"][..], &["repos"][..]] {
        let out = cli(&store, args);
        assert_eq!(out.status.code(), Some(2));
        assert!(String::from_utf8_lossy(&out.stderr).contains("交互式终端"));
        assert!(store.report().unwrap().is_none());
    }
    assert_eq!(cli(&store, &["report"]).status.code(), Some(1));
}
#[test]
fn config_commands_are_explicit_and_do_not_run_checks() {
    let _language = i18n::using(i18n::Language::Chinese);
    let (_temp, store) = fixture();
    let save = |args: &[&str]| {
        let out = cli(&store, args);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    };
    save(&["concurrency", "2"]);
    #[cfg(feature = "developer-scheduler")]
    {
        save(&["interval", "12h"]);
        save(&["autopush", "fast_forward"]);
    }
    let config = store.config().unwrap();
    #[cfg(feature = "developer-scheduler")]
    assert_eq!(config.interval_seconds, 43200);
    assert_eq!(config.max_concurrency, 2);
    #[cfg(feature = "developer-scheduler")]
    assert_eq!(
        config.scheduled_actions.push_policy(),
        PushPolicy::FastForward
    );
    assert!(store.report().unwrap().is_none());
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

#[cfg(feature = "developer-scheduler")]
#[test]
fn workflow_cli_saves_scheduled_steps_without_running_them() {
    let (_temp, store) = fixture();
    let out = cli(
        &store,
        &["scheduled-actions", "fetch", "pull", "push", "inspect"],
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        store.config().unwrap().scheduled_actions.tokens(),
        "fetch pull push inspect"
    );
    assert!(store.report().unwrap().is_none());
    let out = cli(&store, &["status", "--json"]);
    assert!(out.status.success());
    let status: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(status.get("startup_actions").is_none());
    assert!(status.get("check_on_startup").is_none());
    assert_eq!(
        status["scheduled_actions"],
        serde_json::json!(["fetch", "pull", "push", "inspect"])
    );
    let original = fs::read(store.config_path()).unwrap();
    let invalid = cli(&store, &["scheduled-actions", "push", "inspect"]);
    assert!(!invalid.status.success());
    assert_eq!(fs::read(store.config_path()).unwrap(), original);
}
#[test]
fn malformed_and_missing_reports_are_distinct() {
    let _language = i18n::using(i18n::Language::Chinese);
    let (_temp, store) = fixture();
    assert_eq!(cli(&store, &["report"]).status.code(), Some(1));
    fs::create_dir(store.state_dir()).unwrap();
    fs::write(store.state_dir().join("latest.json"), b"broken").unwrap();
    assert_eq!(cli(&store, &["report"]).status.code(), Some(2));
    assert_eq!(cli(&store, &["status", "--json"]).status.code(), Some(2));
}
#[test]
fn help_is_available_without_installation() {
    let _language = i18n::using(i18n::Language::Chinese);
    let out = Command::new(env!("CARGO_BIN_EXE_app"))
        .env("APP_LANG", "zh")
        .arg("--help")
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stdout).contains("concurrency"));
}
#[cfg(all(feature = "developer-scheduler", target_os = "macos"))]
#[test]
fn plist_escapes_paths_preserves_schedule_and_disables_run_at_load() {
    let _language = i18n::using(i18n::Language::Chinese);
    let payload = engine::scheduler::plist(
        &Store::custom("/path/中文 & folder".into()),
        std::path::Path::new("/path/bin/command"),
        21600,
        std::path::Path::new("/path/user"),
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
    assert_eq!(value["Label"], identity::SERVICE_LABEL);
    assert_eq!(value["StartInterval"], 21600);
    assert_eq!(value["RunAtLoad"], false);
    assert_eq!(value["ProgramArguments"][2], "/path/中文 & folder");
    assert_eq!(value["ProgramArguments"][3], "scheduled-check");
}
#[cfg(all(feature = "developer-scheduler", unix))]
#[test]
fn installation_refuses_unowned_binary_before_mutation() {
    let _language = i18n::using(i18n::Language::Chinese);
    let temp = tempfile::tempdir().unwrap();
    let manager = Manager::for_user(temp.path().to_owned(), unsafe { libc::getuid() }).unwrap();
    fs::create_dir_all(manager.binary().parent().unwrap()).unwrap();
    fs::write(manager.binary(), b"unowned binary").unwrap();
    assert!(
        manager
            .install(
                &Store::custom(temp.path().to_owned()),
                &std::env::current_exe().unwrap(),
                &engine::scheduler::timing::Plan::new(
                    21600,
                    engine::scheduler::timing::Basis {
                        source: engine::scheduler::timing::Source::Explicit,
                        at: chrono::NaiveDate::from_ymd_opt(2026, 10, 23)
                            .unwrap()
                            .and_hms_opt(0, 0, 0)
                            .unwrap(),
                    }
                )
                .unwrap(),
                &Cancel::default()
            )
            .is_err()
    );
    assert_eq!(fs::read(manager.binary()).unwrap(), b"unowned binary");
    assert!(!manager.definition().exists());
}

#[test]
fn text_time_uses_system_timezone_without_printing_zone() {
    let _language = i18n::using(i18n::Language::Chinese);
    let utc = "2026-10-21T04:31:16Z";
    let expected = chrono::DateTime::parse_from_rfc3339(utc)
        .unwrap()
        .with_timezone(&chrono::Local)
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();
    assert_eq!(app::cli::local_time(utc), expected);
    assert_eq!(app::cli::local_time("2026-10-21T12:31:16+08:00"), expected);
    assert_eq!(expected.len(), 19);
}

#[test]
fn language_overrides_localize_text_and_preserve_machine_output() {
    let (_temp, store) = fixture();
    let mut snapshots = Vec::new();
    for (language, help_title, status_label, tui_error) in [
        (
            "en",
            concat!(identity::command!(), ": Git repository"),
            if cfg!(feature = "developer-scheduler") {
                "System schedule:"
            } else {
                "Check process:"
            },
            "interactive terminal",
        ),
        (
            "zh",
            concat!(identity::command!(), "：Git 仓库"),
            if cfg!(feature = "developer-scheduler") {
                "系统任务："
            } else {
                "检查进程："
            },
            "交互式终端",
        ),
    ] {
        let run = |args: &[&str]| {
            Command::new(env!("CARGO_BIN_EXE_app"))
                .env("APP_LANG", language)
                .env("LC_ALL", if language == "en" { "zh_CN.UTF-8" } else { "C" })
                .arg("--home")
                .arg(store.config_dir())
                .args(args)
                .output()
                .unwrap()
        };
        let help = run(&["help"]);
        assert!(help.status.success());
        assert!(String::from_utf8_lossy(&help.stdout).starts_with(help_title));
        let status = run(&["status"]);
        assert!(status.status.success());
        assert!(String::from_utf8_lossy(&status.stdout).contains(status_label));
        let tui = run(&["tui"]);
        assert_eq!(tui.status.code(), Some(2));
        assert!(String::from_utf8_lossy(&tui.stderr).contains(tui_error));
        let machine = run(&["status", "--json"]);
        assert!(machine.status.success());
        snapshots.push(serde_json::from_slice::<serde_json::Value>(&machine.stdout).unwrap());
    }
    assert_eq!(snapshots[0], snapshots[1]);
    assert!(store.report().unwrap().is_none());
}

#[test]
fn terminal_locale_is_used_without_explicit_language_and_invalid_override_is_rejected() {
    for (locale, expected) in [
        ("zh_CN.UTF-8", concat!(identity::command!(), "：Git 仓库")),
        (
            "en_US.UTF-8",
            concat!(identity::command!(), ": Git repository"),
        ),
        (
            "fr_FR.UTF-8",
            concat!(identity::command!(), ": Git repository"),
        ),
        ("C", concat!(identity::command!(), ": Git repository")),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_app"))
            .env_remove("APP_LANG")
            .env("LC_ALL", locale)
            .arg("--help")
            .output()
            .unwrap();
        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).starts_with(expected));
    }
    let output = Command::new(env!("CARGO_BIN_EXE_app"))
        .env("APP_LANG", "fr")
        .arg("--help")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("APP_LANG must be en or zh"));
}

#[test]
fn explicit_single_check_creates_a_repository_report_without_creating_a_batch() {
    let (_temp, store) = fixture();
    let repo = store.config_dir().join("repository");
    let output = Command::new("/usr/bin/git")
        .args(["init", "-b", "main"])
        .arg(&repo)
        .output()
        .unwrap();
    assert!(output.status.success());
    let repo = fs::canonicalize(repo).unwrap();
    let mut config = store.config().unwrap();
    config.repositories = vec![repo.clone()];
    store.save_config(&config).unwrap();
    let output = cli(&store, &["fetch", repo.to_str().unwrap()]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(store.report().unwrap().is_none());
    let reports = store.repository_reports().unwrap();
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[&repo].repositories.len(), 1);
    assert_eq!(
        reports[&repo].execution.as_ref().unwrap().max_concurrency,
        1
    );
}

#[cfg(feature = "developer-scheduler")]
#[test]
fn scheduled_check_is_an_explicit_separate_entry_and_rejects_repository_arguments() {
    let (_temp, store) = fixture();
    let mut config = store.config().unwrap();
    config.repositories.clear();
    config.scheduled_actions = engine::workflow::Plan::from_policy(PushPolicy::FastForward);
    store.save_config(&config).unwrap();
    assert_eq!(cli(&store, &["fetch"]).status.code(), Some(1));
    assert_eq!(
        store.report().unwrap().unwrap().push_policy,
        PushPolicy::Disabled
    );
    assert_eq!(
        cli(&store, &["scheduled-check", "/unexpected"])
            .status
            .code(),
        Some(2)
    );
    assert_eq!(cli(&store, &["scheduled-check"]).status.code(), Some(1));
    assert_eq!(
        store.report().unwrap().unwrap().push_policy,
        PushPolicy::FastForward
    );
}

#[cfg(feature = "developer-scheduler")]
#[test]
fn schedule_control_arguments_are_explicit_and_invalid_requests_do_not_write() {
    let (_temp, store) = fixture();
    for args in [
        &["schedule"][..],
        &["schedule", "unknown"][..],
        &["schedule", "enable", "extra"][..],
    ] {
        assert_eq!(cli(&store, args).status.code(), Some(2));
    }
    assert!(store.report().unwrap().is_none());
}

#[cfg(all(feature = "developer-scheduler", target_os = "macos"))]
#[test]
fn standard_plist_uses_user_storage_logs_and_home_without_checkout_arguments() {
    let home = std::path::Path::new("/example/用户 空格");
    let store = engine::directories::UserDirectories::macos(home).store();
    let payload =
        engine::scheduler::plist(&store, std::path::Path::new("/example/bin"), 21600, home)
            .unwrap();
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("task.plist");
    fs::write(&file, payload).unwrap();
    let output = Command::new("/usr/bin/plutil")
        .args(["-convert", "json", "-o", "-"])
        .arg(file)
        .output()
        .unwrap();
    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        value["ProgramArguments"],
        serde_json::json!(["/example/bin", "scheduled-check"])
    );
    assert_eq!(
        value["WorkingDirectory"],
        store.state_dir().to_str().unwrap()
    );
    assert_eq!(
        value["StandardOutPath"],
        store.log_dir().join("launchd.stdout.log").to_str().unwrap()
    );
    assert_eq!(
        value["EnvironmentVariables"]["HOME"],
        home.to_str().unwrap()
    );
    assert_eq!(value["RunAtLoad"], false);
}

#[test]
fn retired_startup_commands_are_rejected_without_changing_configuration() {
    let (_home, store) = fixture();
    let before = fs::read(store.config_path()).unwrap();
    for args in [
        &["startup-check", "true"][..],
        &["startup-actions", "fetch", "inspect"][..],
    ] {
        assert_eq!(cli(&store, args).status.code(), Some(2));
        assert_eq!(fs::read(store.config_path()).unwrap(), before);
    }
    assert!(store.report().unwrap().is_none());
}

#[test]
fn startup_fetch_setting_is_persistent_opt_in_and_never_performs_network_work() {
    let (_home, store) = fixture();
    assert!(!store.config().unwrap().fetch_on_startup);
    for enabled in [true, false] {
        assert!(
            cli(
                &store,
                &["startup-fetch", if enabled { "true" } else { "false" }]
            )
            .status
            .success()
        );
        assert_eq!(store.config().unwrap().fetch_on_startup, enabled);
        let status: serde_json::Value =
            serde_json::from_slice(&cli(&store, &["status", "--json"]).stdout).unwrap();
        assert_eq!(status["fetch_on_startup"], enabled);
        assert!(store.report().unwrap().is_none());
        assert!(store.tasks(10).unwrap().is_empty());
    }
    let saved = fs::read(store.config_path()).unwrap();
    assert_eq!(
        cli(&store, &["startup-fetch", "yes"]).status.code(),
        Some(2)
    );
    assert_eq!(cli(&store, &["check"]).status.code(), Some(2));
    assert_eq!(fs::read(store.config_path()).unwrap(), saved);
}
