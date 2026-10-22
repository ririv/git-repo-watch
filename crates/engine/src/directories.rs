//! Resolve standard user directories without reading installation metadata.
use crate::state::Store;
use anyhow::{Result, ensure};
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct UserDirectories {
    pub config_base: PathBuf,
    pub state_base: PathBuf,
    pub log_base: PathBuf,
}
impl UserDirectories {
    pub fn defaults(home: &Path) -> Self {
        #[cfg(target_os = "macos")]
        {
            Self::macos(home)
        }
        #[cfg(target_os = "linux")]
        {
            Self::linux(home, None, None).expect("absolute user home")
        }
    }
    pub fn from_environment(home: &Path) -> Result<Self> {
        #[cfg(target_os = "macos")]
        {
            Ok(Self::macos(home))
        }
        #[cfg(target_os = "linux")]
        {
            Self::linux(
                home,
                std::env::var_os("XDG_CONFIG_HOME"),
                std::env::var_os("XDG_STATE_HOME"),
            )
        }
    }
    pub fn macos(home: &Path) -> Self {
        let support = home.join("Library/Application Support");
        Self {
            config_base: support.clone(),
            state_base: support,
            log_base: home.join("Library/Logs"),
        }
    }
    pub fn linux(home: &Path, config: Option<OsString>, state: Option<OsString>) -> Result<Self> {
        let config_base = xdg_directory("XDG_CONFIG_HOME", config, home.join(".config"))?;
        let state_base = xdg_directory("XDG_STATE_HOME", state, home.join(".local/state"))?;
        Ok(Self {
            config_base,
            log_base: state_base.clone(),
            state_base,
        })
    }
    pub fn store(&self) -> Store {
        Store::Standard {
            configuration: self.config_base.join(identity::DATA_DIRECTORY),
            state: self.state_base.join(identity::DATA_DIRECTORY),
            logs: self.log_base.join(identity::DATA_DIRECTORY),
        }
    }
}
pub fn xdg_directory(name: &str, value: Option<OsString>, default: PathBuf) -> Result<PathBuf> {
    match value.filter(|v| !v.is_empty()) {
        Some(value) => {
            let path = PathBuf::from(value);
            ensure!(
                path.is_absolute(),
                "{name}: {}",
                i18n::tr!(XdgDirectoryMustBeAbsolute)
            );
            Ok(path)
        }
        None => Ok(default),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn platform_defaults_and_xdg_overrides_resolve_configuration_state_and_logs() {
        let home = Path::new("/example/用户 空格");
        let mac = UserDirectories::macos(home).store();
        assert_eq!(
            mac.config_path(),
            home.join("Library/Application Support")
                .join(identity::DATA_DIRECTORY)
                .join("config.toml")
        );
        assert_eq!(
            mac.database_path(),
            home.join("Library/Application Support")
                .join(identity::DATA_DIRECTORY)
                .join("state.sqlite3")
        );
        assert_eq!(
            mac.log_dir(),
            home.join("Library/Logs").join(identity::DATA_DIRECTORY)
        );
        let linux = UserDirectories::linux(home, None, None).unwrap().store();
        assert_eq!(
            linux.config_path(),
            home.join(".config")
                .join(identity::DATA_DIRECTORY)
                .join("config.toml")
        );
        assert_eq!(
            linux.database_path(),
            home.join(".local/state")
                .join(identity::DATA_DIRECTORY)
                .join("state.sqlite3")
        );
        let custom = UserDirectories::linux(
            home,
            Some("/example/配置 空格".into()),
            Some("/example/状态 空格".into()),
        )
        .unwrap()
        .store();
        assert_eq!(
            custom.config_dir(),
            Path::new("/example/配置 空格").join(identity::DATA_DIRECTORY)
        );
        assert_eq!(
            custom.state_dir(),
            Path::new("/example/状态 空格").join(identity::DATA_DIRECTORY)
        );
        assert!(UserDirectories::linux(home, None, Some("relative".into())).is_err());
    }
}
