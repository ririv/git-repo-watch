//! Typed TOML configuration and explicit legacy JSON conversion.
use super::{Store, atomic};
use crate::{cancel::Cancel, domain::PushPolicy};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::Write,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
};
use toml_edit::{Array, DocumentMut, Item, Value};

impl Default for Config {
    fn default() -> Self {
        Self {
            interval_seconds: 21600,
            schedule_anchor: Default::default(),
            repositories: vec![],
            excluded_roots: vec![],
            push_policy: PushPolicy::Disabled,
            max_concurrency: 4,
            check_on_startup: true,
            chinese_command_trigger: true,
            mouse: MouseConfig::default(),
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
    pub excluded_roots: Vec<PathBuf>,
    pub push_policy: PushPolicy,
    #[serde(
        default = "default_concurrency",
        skip_serializing_if = "is_default_concurrency"
    )]
    pub max_concurrency: usize,
    #[serde(default = "default_startup_check")]
    pub check_on_startup: bool,
    #[serde(default = "default_chinese_command_trigger")]
    pub chinese_command_trigger: bool,
    #[serde(default)]
    pub mouse: MouseConfig,
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
fn default_chinese_command_trigger() -> bool {
    true
}
fn default_startup_check() -> bool {
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
        crate::scheduler::duration::validate(self.interval_seconds)?;
        if let Some(basis) = self.schedule_anchor.basis() {
            basis.validate()?;
        }
        ensure!(
            (1..=32).contains(&self.max_concurrency),
            i18n::tr!(ConcurrencyMustBeBetween1And32)
        );
        for paths in [&self.repositories, &self.excluded_roots] {
            let mut seen = std::collections::HashSet::new();
            for p in paths {
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
        }
        for p in &self.repositories {
            ensure!(
                !self.excluded_roots.iter().any(|x| p.starts_with(x)),
                i18n::tf!(RepositoryIsUnderAnExcludedDirectory, p.display())
            )
        }
        Ok(())
    }
}

const TEMPLATE: &str = include_str!("../../../../config.example.toml");
const FIELDS: [&str; 8] = [
    "interval_seconds",
    "repositories",
    "excluded_roots",
    "push_policy",
    "max_concurrency",
    "check_on_startup",
    "chinese_command_trigger",
    "schedule_anchor",
];
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ConfigurationMigration {
    Ready,
    Converted,
}

pub(super) fn parse(text: &str) -> Result<Config> {
    let config: Config = toml_edit::de::from_str(text).context(i18n::tr!(InvalidToml))?;
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
    let before = original.as_ref().map(serde_json::to_value).transpose()?;
    let after = serde_json::to_value(config)?;
    let mut desired = toml_edit::ser::to_document(config)?;
    desired["max_concurrency"] = toml_edit::value(config.max_concurrency as i64);
    for key in FIELDS {
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
            if let (Some(old), Some(new)) = (current.as_array(), replacement.as_array()) {
                replacement = Value::Array(paths_array(old, new));
            }
            *replacement.decor_mut() = current.decor().clone();
        } else if let Some(array) = replacement.as_array() {
            replacement = Value::Array(paths_array(&Array::new(), array));
        }
        document[key] = Item::Value(replacement);
    }
    for key in ["precise_click", "scroll", "select_pane"] {
        if before
            .as_ref()
            .is_some_and(|before| before["mouse"][key] == after["mouse"][key])
        {
            continue;
        }
        if !document.contains_key("mouse") {
            document["mouse"] = Item::Table(toml_edit::Table::new());
        }
        let mut value = desired["mouse"][key]
            .as_value()
            .expect("mouse boolean")
            .clone();
        if let Some(current) = document["mouse"].get(key).and_then(Item::as_value) {
            *value.decor_mut() = current.decor().clone();
        }
        document["mouse"][key] = Item::Value(value);
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
                parse(&fs::read_to_string(self.config_path())?)?;
            }
            return Ok(ConfigurationMigration::Ready);
        }
        let bytes = fs::read(&legacy)?;
        let config: Config = serde_json::from_slice(&bytes).context(i18n::tr!(InvalidJson))?;
        config.validate()?;
        if self.config_path().try_exists()? {
            ensure!(
                parse(&fs::read_to_string(self.config_path())?)? == config,
                i18n::tr!(ConfigurationMigrationConflict)
            );
        } else {
            let mut staged = tempfile::NamedTempFile::new_in(self.config_dir())?;
            staged
                .as_file()
                .set_permissions(fs::Permissions::from_mode(0o600))?;
            staged.write_all(encode(&config, None)?.as_bytes())?;
            staged.as_file().sync_all()?;
            cancel.check()?;
            staged.persist_noclobber(self.config_path())?;
            File::open(self.config_dir())?.sync_all()?;
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
        File::open(self.config_dir())?.sync_all()?;
        Ok(ConfigurationMigration::Converted)
    }
}
