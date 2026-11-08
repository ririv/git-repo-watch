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
fn agent_preferences_preserve_comments_and_independent_models_without_paths() {
    use engine::{
        ai::Agent,
        scheduler::Manager,
        service::{ConfigEdit, Service},
    };
    let (_temp, store) = fixture();
    store.initialize().unwrap();
    let original = fs::read_to_string(store.config_path()).unwrap();
    let legacy = original.split("# AI 功能").next().unwrap();
    fs::write(store.config_path(), legacy).unwrap();
    let config = store.config().unwrap();
    assert_eq!(config.ai.agent, Agent::Codex);
    store.save_config(&config).unwrap();
    assert_eq!(fs::read_to_string(store.config_path()).unwrap(), legacy);
    let manager = Manager::for_user(store.config_dir(), unsafe { libc::getuid() }).unwrap();
    let service = Service::new(store.clone(), manager);
    service
        .configure(ConfigEdit::AiModel {
            agent: Agent::Codex,
            model: "test-codex-model".into(),
        })
        .unwrap();
    service
        .configure(ConfigEdit::AiAgent(Agent::Claude))
        .unwrap();
    service
        .configure(ConfigEdit::AiModel {
            agent: Agent::Claude,
            model: "test-claude-model".into(),
        })
        .unwrap();
    service
        .configure(ConfigEdit::AiPrompt(engine::ai::Prompt::Custom {
            text: "使用简洁中文".into(),
        }))
        .unwrap();
    let text = fs::read_to_string(store.config_path())
        .unwrap()
        .replace("test-codex-model\"", "test-codex-model\" # 保留模型注释");
    fs::write(store.config_path(), text).unwrap();
    service
        .configure(ConfigEdit::AiAgent(Agent::Codex))
        .unwrap();
    let saved = store.config().unwrap();
    assert_eq!(saved.ai.codex.model, "test-codex-model");
    assert_eq!(saved.ai.claude.model, "test-claude-model");
    assert_eq!(
        saved.ai.prompt,
        engine::ai::Prompt::Custom {
            text: "使用简洁中文".into()
        }
    );
    assert_eq!(saved.mouse, config.mouse);
    let text = fs::read_to_string(store.config_path()).unwrap();
    assert!(text.contains("# 保留模型注释") && !text.contains("executable"));
}
#[test]
fn legacy_ai_table_keeps_preferences_and_comments_when_saved_under_its_new_name() {
    use engine::{
        scheduler::Manager,
        service::{ConfigEdit, Service},
    };
    let (_temp, store) = fixture();
    store.initialize().unwrap();
    let legacy = fs::read_to_string(store.config_path())
        .unwrap()
        .replace("[ai", "[commit_message")
        .replace("enabled = true\n", "")
        .replace(
            "agent = \"codex\"",
            "agent = \"codex\"\ninstructions = \"keep style\" # 用户偏好",
        );
    let mut document: toml_edit::DocumentMut = legacy.parse().unwrap();
    document["commit_message"]
        .as_table_like_mut()
        .unwrap()
        .remove("prompt");
    let legacy = document.to_string();
    fs::write(store.config_path(), &legacy).unwrap();
    let original = store.config().unwrap();
    assert!(original.ai.enabled);
    assert_eq!(
        original.ai.prompt.text(""),
        format!(
            "{}\n\nkeep style",
            engine::commit_message::DEFAULT_PROMPT.trim()
        )
    );
    assert_eq!(fs::read_to_string(store.config_path()).unwrap(), legacy);
    let manager = Manager::for_user(store.config_dir(), unsafe { libc::getuid() }).unwrap();
    let service = Service::new(store.clone(), manager);
    service.configure(ConfigEdit::AiEnabled(false)).unwrap();
    let saved = store.config().unwrap();
    assert!(!saved.ai.enabled);
    assert_eq!(saved.ai.prompt, original.ai.prompt);
    let text = fs::read_to_string(store.config_path()).unwrap();
    assert!(text.contains("[ai]") && !text.contains("[commit_message"));
    assert!(text.contains("# 用户偏好"));
    service.configure(ConfigEdit::AiEnabled(true)).unwrap();
    assert_eq!(store.config().unwrap(), original);
}
#[test]
fn template_and_initial_configuration_have_all_explicit_defaults_and_private_permissions() {
    let (_temp, store) = fixture();
    store.initialize().unwrap();
    assert_eq!(store.config().unwrap(), Config::default());
    let text = fs::read_to_string(store.config_path()).unwrap();
    assert!(text.contains("# 定时检查间隔"));
    assert!(text.contains("max_concurrency = 4"));
    assert!(text.contains("fetch_on_startup = false"));
    assert!(!text.contains("check_on_startup") && !text.contains("startup_actions"));
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
fn retired_startup_configuration_is_removed_without_changing_the_schedule_or_user_comments() {
    use engine::state::ConfigurationMigration;
    let (_temp, store) = fixture();
    let legacy = "# 用户配置\ninterval_seconds = 21600\nrepositories = []\n# 启动流程：按数组顺序执行。\nstartup_actions = ['fetch', 'pull', 'push', 'inspect'] # 用户备注\ncheck_on_startup = true\nscheduled_actions = ['inspect']\n";
    fs::write(store.config_path(), legacy).unwrap();
    let config = store.config().unwrap();
    assert!(!config.fetch_on_startup);
    assert_eq!(config.scheduled_actions, engine::workflow::Plan::local());
    assert_eq!(fs::read_to_string(store.config_path()).unwrap(), legacy);
    assert!(matches!(
        store.migrate_configuration(&Cancel::default()).unwrap(),
        ConfigurationMigration::Converted
    ));
    let saved = fs::read_to_string(store.config_path()).unwrap();
    assert!(!saved.contains("startup_actions") && !saved.contains("check_on_startup"));
    assert!(saved.contains("# 用户配置") && saved.contains("# 用户备注"));
    assert!(!saved.contains("# 启动流程"));
    assert_eq!(store.config().unwrap(), config);
    assert!(matches!(
        store.migrate_configuration(&Cancel::default()).unwrap(),
        ConfigurationMigration::Ready
    ));
    assert_eq!(fs::read_to_string(store.config_path()).unwrap(), saved);
    assert!(store.report().unwrap().is_none());
    assert!(store.tasks(10).unwrap().is_empty());
}

#[test]
fn legacy_json_startup_settings_are_retired_without_enabling_scheduled_steps() {
    let (_temp, store) = fixture();
    let mut legacy = serde_json::to_value(Config {
        scheduled_actions: engine::workflow::Plan::local(),
        ..Config::default()
    })
    .unwrap();
    legacy["check_on_startup"] = serde_json::json!(true);
    legacy["startup_actions"] = serde_json::json!(["fetch", "pull", "push", "inspect"]);
    fs::write(
        store.legacy_config_path(),
        serde_json::to_vec(&legacy).unwrap(),
    )
    .unwrap();
    store.migrate_configuration(&Cancel::default()).unwrap();
    assert_eq!(
        store.config().unwrap().scheduled_actions,
        engine::workflow::Plan::local()
    );
    assert!(!store.legacy_config_path().exists());
    let saved = fs::read_to_string(store.config_path()).unwrap();
    assert!(!saved.contains("startup_actions") && !saved.contains("check_on_startup"));
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
scheduled_actions = ['fetch', 'inspect']
max_concurrency = 4
text_wrap = false # 自动换行
chinese_command_trigger = true # 中文输入
"#;
    fs::write(store.config_path(), text).unwrap();
    let mut config = store.config().unwrap();
    config.text_wrap = true;
    store.save_config(&config).unwrap();
    let next = fs::read_to_string(store.config_path()).unwrap();
    assert_eq!(
        next,
        text.replace("text_wrap = false", "text_wrap = true")
            .replace("excluded_roots = []\n", "")
    );
    assert_eq!(store.config().unwrap(), config);
    config
        .repositories
        .push(PathBuf::from("/example/new \"quoted\" 仓库"));
    store.save_config(&config).unwrap();
    let next = fs::read_to_string(store.config_path()).unwrap();
    assert!(
        next.contains("# 用户配置") && next.contains("# 仓库一") && next.contains("# 自动换行")
    );
    assert_eq!(store.config().unwrap(), config);
}

#[test]
fn automatic_and_explicit_bases_persist_and_interval_edits_keep_the_original_basis() {
    use engine::scheduler::timing::{Anchor, Basis, parse_anchor};
    let (_temp, store) = fixture();
    store.initialize().unwrap();
    assert!(store.config().unwrap().schedule_anchor.is_unset());
    let mut config = store.config().unwrap();
    config.schedule_anchor = Anchor::Set(Basis::automatic(chrono::Local::now()));
    store.save_config(&config).unwrap();
    let basis = store.config().unwrap().schedule_anchor;
    config.interval_seconds = 43200;
    store.save_config(&config).unwrap();
    assert_eq!(store.config().unwrap().schedule_anchor, basis);
    config.schedule_anchor =
        parse_anchor("2026-10-23 00:00:00", chrono::Local::now().date_naive()).unwrap();
    store.save_config(&config).unwrap();
    assert_eq!(store.config().unwrap(), config);
    config.schedule_anchor = Anchor::Unset;
    store.save_config(&config).unwrap();
    assert!(store.config().unwrap().schedule_anchor.is_unset());
    assert!(
        !fs::read_to_string(store.config_path())
            .unwrap()
            .contains("[schedule_anchor]")
    );
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
        fetch_on_startup: false,
        auto_fetch: Default::default(),
        experimental_features: true,
        chinese_command_trigger: true,
        text_wrap: false,
        file_icons: true,
        view: Default::default(),
        mouse: Default::default(),
        editor: Default::default(),
        ai: Default::default(),
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

#[test]
fn chinese_trigger_defaults_without_rewriting_old_config_and_persists_explicit_choice() {
    let (_temp, store) = fixture();
    store.initialize().unwrap();
    let text = fs::read_to_string(store.config_path())
        .unwrap()
        .replace("chinese_command_trigger = true\n", "");
    fs::write(store.config_path(), &text).unwrap();
    let mut config = store.config().unwrap();
    assert!(config.chinese_command_trigger);
    assert_eq!(fs::read_to_string(store.config_path()).unwrap(), text);
    config.chinese_command_trigger = false;
    store.save_config(&config).unwrap();
    assert!(!store.config().unwrap().chinese_command_trigger);
    assert!(
        fs::read_to_string(store.config_path())
            .unwrap()
            .contains("chinese_command_trigger = false")
    );
}

#[test]
fn mouse_preferences_default_independently_and_preserve_table_comments() {
    use engine::state::MouseSetting;
    let (_temp, store) = fixture();
    store.initialize().unwrap();
    let original = fs::read_to_string(store.config_path()).unwrap();
    let legacy = original.split("# 鼠标能力").next().unwrap();
    fs::write(store.config_path(), legacy).unwrap();
    let mut config = store.config().unwrap();
    assert_eq!(config.mouse, Default::default());
    config.mouse.set(MouseSetting::Scroll, false);
    store.save_config(&config).unwrap();
    assert_eq!(store.config().unwrap().mouse, config.mouse);
    assert!(store.config().unwrap().mouse.precise_click);
    let text = fs::read_to_string(store.config_path())
        .unwrap()
        .replace("scroll = false", "scroll = false # 保留此注释");
    fs::write(store.config_path(), &text).unwrap();
    config.mouse.set(MouseSetting::SelectPane, false);
    store.save_config(&config).unwrap();
    assert!(
        fs::read_to_string(store.config_path())
            .unwrap()
            .contains("scroll = false # 保留此注释")
    );
    assert_eq!(store.config().unwrap().mouse, config.mouse);
    let invalid = fs::read_to_string(store.config_path()).unwrap() + "unknown_mouse = true\n";
    fs::write(store.config_path(), invalid).unwrap();
    assert!(store.config().is_err());
}

#[test]
fn interaction_mode_defaults_without_rewriting_legacy_and_preserves_pointer_preferences() {
    use engine::{
        scheduler::Manager,
        service::{ConfigEdit, Service},
        state::InteractionMode,
    };
    let (_temp, store) = fixture();
    store.initialize().unwrap();
    let text = fs::read_to_string(store.config_path())
        .unwrap()
        .replace("interaction_mode = \"application\"\n", "");
    fs::write(store.config_path(), &text).unwrap();
    let config = store.config().unwrap();
    assert_eq!(config.interaction_mode, InteractionMode::Application);
    store.save_config(&config).unwrap();
    assert_eq!(fs::read_to_string(store.config_path()).unwrap(), text);
    let manager = Manager::for_user(store.config_dir(), unsafe { libc::getuid() }).unwrap();
    let service = Service::new(store.clone(), manager);
    for mode in [InteractionMode::TextSelection, InteractionMode::Application] {
        service.configure(ConfigEdit::Interaction(mode)).unwrap();
        let saved = store.config().unwrap();
        assert_eq!(saved.interaction_mode, mode);
        assert_eq!(saved.mouse, config.mouse);
        assert_eq!(saved.repositories, config.repositories);
    }
    assert!(
        fs::read_to_string(store.config_path())
            .unwrap()
            .contains("# 交互模式")
    );
}

#[test]
fn complete_prompt_roundtrips_multiline_and_reset_removes_the_old_custom_text() {
    use engine::ai::Prompt;
    let (_temp, store) = fixture();
    store.initialize().unwrap();
    let mut config = store.config().unwrap();
    config.ai.prompt = Prompt::Custom {
        text: "只生成标题。\n保留中文、空格和 \"引号\"。\n".into(),
    };
    store.save_config(&config).unwrap();
    assert_eq!(store.config().unwrap(), config);
    let text = fs::read_to_string(store.config_path())
        .unwrap()
        .replace("mode = \"custom\"", "mode = \"custom\" # 自定义注释");
    fs::write(store.config_path(), &text).unwrap();
    config.ai.codex.model = "another-model".into();
    store.save_config(&config).unwrap();
    assert!(
        fs::read_to_string(store.config_path())
            .unwrap()
            .contains("# 自定义注释")
    );
    config.ai.prompt = Prompt::Default;
    store.save_config(&config).unwrap();
    assert_eq!(store.config().unwrap().ai.prompt, Prompt::Default);
    let text = fs::read_to_string(store.config_path()).unwrap();
    assert!(!text.contains("只生成标题"));
    assert!(text.contains("# 自定义注释"));
}
#[test]
fn conflicting_legacy_instructions_and_full_prompt_leave_configuration_untouched() {
    let (_temp, store) = fixture();
    store.initialize().unwrap();
    let text = fs::read_to_string(store.config_path()).unwrap().replace(
        "agent = \"codex\"",
        "agent = \"codex\"\ninstructions = \"old preference\"",
    );
    fs::write(store.config_path(), &text).unwrap();
    assert_eq!(
        store.config().unwrap_err().to_string(),
        i18n::tr!(AgentPromptConflict)
    );
    assert!(store.save_config(&Config::default()).is_err());
    assert_eq!(fs::read_to_string(store.config_path()).unwrap(), text);
}

#[test]
fn text_wrap_defaults_off_and_updates_only_its_preference_with_comments_retained() {
    use engine::{
        scheduler::Manager,
        service::{ConfigEdit, Service},
    };
    let (_temp, store) = fixture();
    store.initialize().unwrap();
    let text = fs::read_to_string(store.config_path())
        .unwrap()
        .replace("text_wrap = false\n", "");
    fs::write(store.config_path(), &text).unwrap();
    let original = store.config().unwrap();
    assert!(!original.text_wrap);
    let manager = Manager::for_user(store.config_dir(), unsafe { libc::getuid() }).unwrap();
    let service = Service::new(store.clone(), manager);
    service.configure(ConfigEdit::TextWrap(true)).unwrap();
    let mut saved = store.config().unwrap();
    assert!(saved.text_wrap);
    saved.text_wrap = false;
    assert_eq!(saved, original);
    assert!(
        fs::read_to_string(store.config_path())
            .unwrap()
            .contains("# 文件文本预览")
    );
    service.configure(ConfigEdit::TextWrap(false)).unwrap();
    assert_eq!(store.config().unwrap(), original);
}

#[test]
fn file_icons_default_on_and_only_explicit_changes_extend_older_configuration() {
    use engine::{
        scheduler::Manager,
        service::{ConfigEdit, Service},
    };
    let (_temp, store) = fixture();
    store.initialize().unwrap();
    let original = fs::read_to_string(store.config_path())
        .unwrap()
        .replace("file_icons = true\n", "");
    fs::write(store.config_path(), &original).unwrap();
    assert!(store.config().unwrap().file_icons);
    let manager = Manager::for_user(store.config_dir(), unsafe { libc::getuid() }).unwrap();
    let service = Service::new(store.clone(), manager);
    service.configure(ConfigEdit::Concurrency(7)).unwrap();
    assert!(
        !fs::read_to_string(store.config_path())
            .unwrap()
            .contains("file_icons =")
    );
    service.configure(ConfigEdit::FileIcons(false)).unwrap();
    assert!(!store.config().unwrap().file_icons);
    assert_eq!(store.config().unwrap().max_concurrency, 7);
    assert!(
        fs::read_to_string(store.config_path())
            .unwrap()
            .contains("file_icons = false")
    );
    service.configure(ConfigEdit::FileIcons(true)).unwrap();
    assert!(store.config().unwrap().file_icons);
}

#[test]
fn retiring_exclusions_preserves_registered_paths_comments_and_other_preferences() {
    let (_temp, store) = fixture();
    store.initialize().unwrap();
    let mut config = store.config().unwrap();
    config.repositories = vec!["/example/keep".into()];
    config.text_wrap = true;
    store.save_config(&config).unwrap();
    let current = fs::read_to_string(store.config_path()).unwrap();
    let legacy = format!("# 用户备注\nexcluded_roots = [\"/example/old\"]\n{current}");
    fs::write(store.config_path(), &legacy).unwrap();
    assert_eq!(store.config().unwrap(), config);
    assert_eq!(fs::read_to_string(store.config_path()).unwrap(), legacy);
    assert!(matches!(
        store.migrate_configuration(&Cancel::default()).unwrap(),
        ConfigurationMigration::Converted
    ));
    let migrated = fs::read_to_string(store.config_path()).unwrap();
    assert!(!migrated.contains("excluded_roots"));
    assert!(migrated.contains("# 用户备注"));
    assert_eq!(store.config().unwrap(), config);
    assert!(matches!(
        store.migrate_configuration(&Cancel::default()).unwrap(),
        ConfigurationMigration::Ready
    ));
}

#[test]
fn experimental_features_default_on_preserve_legacy_files_and_persist_explicit_changes() {
    use engine::{
        scheduler::Manager,
        service::{ConfigEdit, Service},
    };
    let (_home, store) = fixture();
    store.initialize().unwrap();
    assert!(store.config().unwrap().experimental_features);
    let legacy = fs::read_to_string(store.config_path())
        .unwrap()
        .replace("experimental_features = true\n", "");
    fs::write(store.config_path(), &legacy).unwrap();
    store.save_config(&store.config().unwrap()).unwrap();
    assert_eq!(fs::read_to_string(store.config_path()).unwrap(), legacy);
    let service = Service::new(
        store.clone(),
        Manager::for_user(store.config_dir(), unsafe { libc::getuid() }).unwrap(),
    );
    service
        .configure(ConfigEdit::ExperimentalFeatures(false))
        .unwrap();
    assert!(!store.config().unwrap().experimental_features);
    assert!(store.config().unwrap().ai.enabled);
    assert_eq!(
        service
            .compose(
                std::path::Path::new("/missing"),
                &engine::ai::Config::default(),
                &Cancel::default()
            )
            .unwrap_err()
            .to_string(),
        i18n::tr!(ExperimentalDisabled)
    );
    service.configure(ConfigEdit::FileIcons(false)).unwrap();
    assert!(!store.config().unwrap().experimental_features);
    service
        .configure(ConfigEdit::ExperimentalFeatures(true))
        .unwrap();
    assert!(store.config().unwrap().experimental_features);
}
