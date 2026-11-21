//! Typed TOML configuration and explicit legacy JSON conversion.
use super::{Store, atomic};
use crate::cancel::Cancel;
use crate::platform::files;
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{fs, io::Write, path::PathBuf};
use toml_edit::{Array, DocumentMut, Item, Value};
mod ai;
mod retired;
mod workflows;

impl Default for Config {
    fn default() -> Self {
        Self {
            interval_seconds: 21600,
            schedule_anchor: Default::default(),
            repositories: vec![],
            repository_environment: Default::default(),
            scheduled_actions: crate::workflow::Plan::fetch(),
            max_concurrency: 4,
            fetch_on_startup: false,
            auto_fetch: AutoFetchConfig::default(),
            text_wrap: false,
            file_icons: true,
            experimental_features: true,
            chinese_command_trigger: true,
            interaction_mode: Default::default(),
            view: Default::default(),
            mouse: MouseConfig::default(),
            editor: Default::default(),
            ai: Default::default(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub interval_seconds: u64,
    #[serde(
        default,
        skip_serializing_if = "crate::scheduler::timing::Anchor::is_unset"
    )]
    pub schedule_anchor: crate::scheduler::timing::Anchor,
    pub repositories: Vec<PathBuf>,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub repository_environment:
        std::collections::BTreeMap<PathBuf, crate::git_environment::Environment>,
    #[serde(default)]
    pub scheduled_actions: crate::workflow::Plan,
    #[serde(
        default = "default_concurrency",
        skip_serializing_if = "is_default_concurrency"
    )]
    pub max_concurrency: usize,
    #[serde(default)]
    pub fetch_on_startup: bool,
    #[serde(default)]
    pub auto_fetch: AutoFetchConfig,
    #[serde(default)]
    pub text_wrap: bool,
    #[serde(default = "default_file_icons")]
    pub file_icons: bool,
    #[serde(default = "default_experimental_features")]
    pub experimental_features: bool,
    #[serde(default = "default_chinese_command_trigger")]
    pub chinese_command_trigger: bool,
    #[serde(default)]
    pub interaction_mode: super::InteractionMode,
    #[serde(default)]
    pub view: super::view::Preferences,
    #[serde(default)]
    pub mouse: MouseConfig,
    #[serde(default)]
    pub editor: crate::editor::Preference,
    #[serde(default, alias = "commit_message")]
    pub ai: crate::ai::Config,
}
/// Preferences for periodic remote refresh while the TUI is focused.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AutoFetchConfig {
    pub enabled: bool,
    pub interval_seconds: u64,
}
impl Default for AutoFetchConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_seconds: 180,
        }
    }
}
/// Independently configurable pointer capabilities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MouseConfig {
    pub precise_click: bool,
    pub scroll: bool,
    pub select_pane: bool,
}
impl Default for MouseConfig {
    fn default() -> Self {
        Self {
            precise_click: true,
            scroll: true,
            select_pane: true,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseSetting {
    PreciseClick,
    Scroll,
    SelectPane,
}
impl MouseConfig {
    pub fn enabled(&self, setting: MouseSetting) -> bool {
        match setting {
            MouseSetting::PreciseClick => self.precise_click,
            MouseSetting::Scroll => self.scroll,
            MouseSetting::SelectPane => self.select_pane,
        }
    }
    pub fn set(&mut self, setting: MouseSetting, enabled: bool) {
        match setting {
            MouseSetting::PreciseClick => self.precise_click = enabled,
            MouseSetting::Scroll => self.scroll = enabled,
            MouseSetting::SelectPane => self.select_pane = enabled,
        }
    }
}
fn default_experimental_features() -> bool {
    true
}
fn default_file_icons() -> bool {
    true
}
fn default_chinese_command_trigger() -> bool {
    true
}
fn default_concurrency() -> usize {
    4
}
fn is_default_concurrency(value: &usize) -> bool {
    *value == 4
}
impl Config {
    pub fn validate(&self) -> Result<()> {
        self.ai.validate()?;
        crate::scheduler::duration::validate(self.auto_fetch.interval_seconds)?;
        crate::scheduler::duration::validate(self.interval_seconds)?;
        if let Some(basis) = self.schedule_anchor.basis() {
            basis.validate()?;
        }
        ensure!(
            (1..=32).contains(&self.max_concurrency),
            i18n::tr!(ConcurrencyMustBeBetween1And32)
        );
        let mut seen = std::collections::HashSet::new();
        for p in &self.repositories {
            ensure!(
                p.is_absolute()
                    && p.components().all(|c| !matches!(
                        c,
                        std::path::Component::ParentDir | std::path::Component::CurDir
                    )),
                i18n::tf!(PathMustBeANormalizedAbsolutePath, p.display())
            );
            ensure!(seen.insert(p), i18n::tf!(DuplicatePath, p.display()));
        }
        for (path, environment) in &self.repository_environment {
            ensure!(
                path.is_absolute()
                    && !path.components().any(|part| matches!(
                        part,
                        std::path::Component::ParentDir | std::path::Component::CurDir
                    )),
                i18n::tf!(PathMustBeANormalizedAbsolutePath, path.display())
            );
            environment.validate()?;
        }
        Ok(())
    }
}

const TEMPLATE: &str = include_str!("../../../../config.example.toml");
const FIELDS: [&str; 12] = [
    "interval_seconds",
    "repositories",
    "scheduled_actions",
    "max_concurrency",
    "fetch_on_startup",
    "text_wrap",
    "file_icons",
    "experimental_features",
    "chinese_command_trigger",
    "schedule_anchor",
    "interaction_mode",
    "editor",
];
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ConfigurationMigration {
    Ready,
    Converted,
}

pub(super) fn parse(text: &str) -> Result<Config> {
    let mut document: DocumentMut = text.parse().context(i18n::tr!(InvalidToml))?;
    ai::migrate(&mut document)?;
    workflows::migrate(&mut document)?;
    retired::exclusions(&mut document)?;
    retired::startup(&mut document)?;
    let config: Config =
        toml_edit::de::from_str(&document.to_string()).context(i18n::tr!(InvalidToml))?;
    config.validate()?;
    Ok(config)
}
pub(super) fn encode(config: &Config, existing: Option<&str>) -> Result<String> {
    config.validate()?;
    let original = existing.map(parse).transpose()?;
    let mut document: DocumentMut = existing
        .unwrap_or(TEMPLATE)
        .parse()
        .context(i18n::tr!(InvalidToml))?;
    ai::migrate(&mut document)?;
    workflows::migrate(&mut document)?;
    retired::exclusions(&mut document)?;
    retired::startup(&mut document)?;
    // 旧版将 AI 选项保存在提交专用表；下次显式保存配置时整体更名，保留表内注释。
    if let Some(legacy) = document.remove("commit_message") {
        document.insert("ai", legacy);
    }
    let before = original.as_ref().map(serde_json::to_value).transpose()?;
    let after = serde_json::to_value(config)?;
    let mut desired = toml_edit::ser::to_document(config)?;
    desired["max_concurrency"] = toml_edit::value(config.max_concurrency as i64);
    for key in FIELDS {
        // 旧配置缺少可选偏好时沿用默认值，不因无关配置更新而补写字段。
        if matches!(
            key,
            "interaction_mode"
                | "editor"
                | "text_wrap"
                | "file_icons"
                | "experimental_features"
                | "scheduled_actions"
                | "fetch_on_startup"
        ) && before
            .as_ref()
            .is_some_and(|before| before.get(key) == after.get(key))
        {
            continue;
        }
        if key == "schedule_anchor" {
            if before
                .as_ref()
                .is_some_and(|before| before.get(key) == after.get(key))
            {
                continue;
            }
            match desired.remove(key) {
                Some(value) => {
                    document[key] = value;
                }
                None => {
                    document.remove(key);
                }
            }
            continue;
        }
        if document.contains_key(key)
            && before
                .as_ref()
                .is_some_and(|before| before.get(key) == after.get(key))
        {
            continue;
        }
        let mut replacement = desired[key]
            .as_value()
            .context(i18n::tr!(InvalidToml))?
            .clone();
        if let Some(current) = document.get(key).and_then(Item::as_value) {
            if let (Some(old), Some(new)) = (current.as_array(), replacement.as_array())
                && (key == "repositories" || old.to_string().contains(['\n', '#']))
            {
                replacement = Value::Array(paths_array(old, new));
            }
            *replacement.decor_mut() = current.decor().clone();
        } else if let Some(array) = replacement.as_array()
            && key == "repositories"
        {
            replacement = Value::Array(paths_array(&Array::new(), array));
        }
        document[key] = Item::Value(replacement);
    }
    if before
        .as_ref()
        .is_none_or(|before| before["repository_environment"] != after["repository_environment"])
    {
        if let Some(table) = desired.remove("repository_environment") {
            let mut table = table
                .into_table()
                .expect("repository environments serialize as a table");
            if let Some(current) = document
                .get("repository_environment")
                .and_then(Item::as_table)
            {
                for (key, profile) in &config.repository_environment {
                    let key = key.to_str().expect("serialized repository path is UTF-8");
                    if original.as_ref().and_then(|config| {
                        config.repository_environment.get(std::path::Path::new(key))
                    }) == Some(profile)
                        && let Some(existing) = current.get(key)
                    {
                        table.insert(key, existing.clone());
                    }
                }
            }
            document["repository_environment"] = Item::Table(table);
        } else {
            document.remove("repository_environment");
        }
    }
    for (table, keys) in [
        ("mouse", &["precise_click", "scroll", "select_pane"][..]),
        ("auto_fetch", &["enabled", "interval_seconds"][..]),
    ] {
        for &key in keys {
            if before
                .as_ref()
                .is_some_and(|before| before[table][key] == after[table][key])
            {
                continue;
            }
            if !document.contains_key(table) {
                document[table] = Item::Table(toml_edit::Table::new());
            }
            let mut value = desired[table][key]
                .as_value()
                .expect("preference scalar")
                .clone();
            if let Some(current) = document[table].get(key).and_then(Item::as_value) {
                *value.decor_mut() = current.decor().clone();
            }
            document[table][key] = Item::Value(value);
        }
    }
    for key in [
        "file_layout",
        "diff_layout",
        "diff_header",
        "diff_context",
        "repository_order",
    ] {
        if before
            .as_ref()
            .is_some_and(|before| before["view"][key] == after["view"][key])
        {
            continue;
        }
        if !document.contains_key("view") {
            document["view"] = Item::Table(toml_edit::Table::new());
        }
        let mut value = desired["view"][key]
            .as_value()
            .expect("view enum serialized as string")
            .clone();
        if let Some(current) = document["view"].get(key).and_then(Item::as_value) {
            *value.decor_mut() = current.decor().clone();
        }
        document["view"][key] = Item::Value(value);
    }
    for path in [
        &["enabled"][..],
        &["agent"][..],
        &["codex", "model"][..],
        &["claude", "model"][..],
    ] {
        let lookup = |value: &serde_json::Value| {
            path.iter()
                .fold(&value["ai"], |value, key| &value[*key])
                .clone()
        };
        if before
            .as_ref()
            .is_some_and(|before| lookup(before) == lookup(&after))
        {
            continue;
        }
        if !document.contains_key("ai") {
            document["ai"] = Item::Table(toml_edit::Table::new());
        }
        let mut source = &desired["ai"];
        let mut destination = &mut document["ai"];
        for key in &path[..path.len() - 1] {
            source = &source[*key];
            if destination.get(*key).is_none() {
                destination[*key] = Item::Table(toml_edit::Table::new());
            }
            destination = &mut destination[*key];
        }
        let key = path[path.len() - 1];
        let mut value = source[key].as_value().expect("agent scalar").clone();
        if let Some(current) = destination.get(key).and_then(Item::as_value) {
            *value.decor_mut() = current.decor().clone();
        }
        destination[key] = Item::Value(value);
    }
    if before
        .as_ref()
        .is_none_or(|before| before["ai"]["prompt"] != after["ai"]["prompt"])
    {
        ai::replace(&mut document, &config.ai.prompt)?;
    }
    Ok(document.to_string())
}
fn paths_array(old: &Array, desired: &Array) -> Array {
    let mut result = Array::new();
    for next in desired.iter() {
        let retained = old.iter().find(|item| item.as_str() == next.as_str());
        let mut value = retained.unwrap_or(next).clone();
        if !value
            .decor()
            .prefix()
            .and_then(|prefix| prefix.as_str())
            .is_some_and(|prefix| prefix.contains('\n'))
        {
            value.decor_mut().set_prefix("\n    ");
        }
        result.push_formatted(value);
    }
    if !result.is_empty() {
        result.set_trailing_comma(true);
        if old
            .trailing()
            .as_str()
            .is_some_and(|trailing| trailing.contains('#'))
        {
            result.set_trailing(old.trailing().clone());
        } else {
            result.set_trailing("\n");
        }
    }
    result
}
impl Store {
    pub fn legacy_config_path(&self) -> PathBuf {
        self.config_dir().join("config.json")
    }
    pub fn config(&self) -> Result<Config> {
        ensure!(
            !self.legacy_config_path().try_exists()?,
            i18n::tr!(ConfigurationMigrationRequired)
        );
        ensure!(
            self.config_path().try_exists()?,
            i18n::tr!(ConfigurationMissingRunInit)
        );
        parse(
            &fs::read_to_string(self.config_path())
                .with_context(|| i18n::tf!(CannotRead, self.config_path().display()))?,
        )
    }
    pub fn save_config(&self, config: &Config) -> Result<()> {
        ensure!(
            !self.legacy_config_path().try_exists()?,
            i18n::tr!(ConfigurationMigrationRequired)
        );
        let existing = match fs::read_to_string(self.config_path()) {
            Ok(text) => Some(text),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };
        atomic(
            &self.config_path(),
            encode(config, existing.as_deref())?.as_bytes(),
            0o600,
        )
    }
    pub fn migrate_configuration(&self, cancel: &Cancel) -> Result<ConfigurationMigration> {
        let _lock = self.transaction(cancel)?;
        self.migrate_configuration_locked(cancel)
    }
    pub(super) fn migrate_configuration_locked(
        &self,
        cancel: &Cancel,
    ) -> Result<ConfigurationMigration> {
        cancel.check()?;
        let legacy = self.legacy_config_path();
        if !legacy.try_exists()? {
            if self.config_path().try_exists()? {
                let text = fs::read_to_string(self.config_path())?;
                let config = parse(&text)?;
                let document: DocumentMut = text.parse()?;
                if document.contains_key("push_policy") || retired::pending(&document) {
                    atomic(
                        &self.config_path(),
                        encode(&config, Some(&text))?.as_bytes(),
                        0o600,
                    )?;
                    return Ok(ConfigurationMigration::Converted);
                }
            }
            return Ok(ConfigurationMigration::Ready);
        }
        let bytes = fs::read(&legacy)?;
        let mut value: serde_json::Value =
            serde_json::from_slice(&bytes).context(i18n::tr!(InvalidJson))?;
        workflows::migrate_json(&mut value)?;
        retired::json_exclusions(&mut value)?;
        retired::json_startup(&mut value)?;
        let config: Config = serde_json::from_value(value).context(i18n::tr!(InvalidJson))?;
        config.validate()?;
        if self.config_path().try_exists()? {
            ensure!(
                parse(&fs::read_to_string(self.config_path())?)? == config,
                i18n::tr!(ConfigurationMigrationConflict)
            );
            let text = fs::read_to_string(self.config_path())?;
            let document: DocumentMut = text.parse()?;
            if document.contains_key("push_policy") || retired::pending(&document) {
                atomic(
                    &self.config_path(),
                    encode(&config, Some(&text))?.as_bytes(),
                    0o600,
                )?;
            }
        } else {
            let mut staged = tempfile::NamedTempFile::new_in(self.config_dir())?;
            files::private_path(staged.path(), 0o600)?;
            staged.write_all(encode(&config, None)?.as_bytes())?;
            staged.as_file().sync_all()?;
            cancel.check()?;
            files::publish(staged, &self.config_path(), false)?;
            files::sync_directory(&self.config_dir())?;
        }
        ensure!(
            parse(&fs::read_to_string(self.config_path())?)? == config,
            i18n::tr!(ConfigurationMigrationConflict)
        );
        ensure!(
            fs::read(&legacy)? == bytes,
            i18n::tr!(ConfigurationMigrationConflict)
        );
        fs::remove_file(legacy)?;
        files::sync_directory(&self.config_dir())?;
        Ok(ConfigurationMigration::Converted)
    }
}
