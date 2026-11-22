//! Per-worktree environment and optional launcher for system Git and its hooks.
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    env,
    ffi::OsString,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Environment {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub path_prepend: Vec<PathBuf>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub variables: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub launcher: Option<Launcher>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Launcher {
    pub program: PathBuf,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum Edit {
    Path(Vec<PathBuf>),
    Set { name: String, value: String },
    Unset(String),
    Launcher(Option<Launcher>),
}

impl Edit {
    pub(crate) fn apply(self, environment: &mut Environment) {
        match self {
            Self::Path(paths) => environment.path_prepend = paths,
            Self::Set { name, value } => {
                environment.variables.insert(name, value);
            }
            Self::Unset(name) => {
                environment.variables.remove(&name);
            }
            Self::Launcher(launcher) => environment.launcher = launcher,
        }
    }
}

pub fn parse_words(value: &str) -> Result<Vec<String>> {
    shlex::split(value).context(i18n::tr!(GitEnvironmentInvalidInput))
}

pub fn display_words<'a>(words: impl IntoIterator<Item = &'a str>) -> String {
    shlex::try_join(words).expect("validated Git environment contains no NUL bytes")
}

pub fn validate_variable(name: &str) -> Result<()> {
    let mut bytes = name.bytes();
    #[cfg(windows)]
    let name = name.to_ascii_uppercase();
    ensure!(
        bytes
            .next()
            .is_some_and(|first| first.is_ascii_alphabetic() || first == b'_')
            && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
            && name != "PATH"
            && !name.starts_with("GIT_"),
        i18n::tr!(GitEnvironmentInvalidVariable)
    );
    Ok(())
}

impl Environment {
    pub(crate) fn validate(&self) -> Result<()> {
        for path in &self.path_prepend {
            let value = path.as_os_str().as_encoded_bytes();
            ensure!(
                !value.is_empty() && !value.contains(&0) && env::join_paths([path]).is_ok(),
                i18n::tr!(GitEnvironmentInvalidPath)
            );
        }
        for (name, value) in &self.variables {
            validate_variable(name)?;
            ensure!(!value.contains('\0'), i18n::tr!(GitEnvironmentInvalidInput));
        }
        if let Some(launcher) = &self.launcher {
            let program = launcher.program.as_os_str().as_encoded_bytes();
            ensure!(
                !program.is_empty()
                    && !program.contains(&0)
                    && launcher.args.iter().all(|arg| !arg.contains('\0')),
                i18n::tr!(GitEnvironmentInvalidInput)
            );
        }
        Ok(())
    }

    pub fn path_directories<'a>(&'a self, root: &'a Path) -> impl Iterator<Item = PathBuf> + 'a {
        self.path_prepend.iter().map(|path| root.join(path))
    }

    pub(crate) fn variables_for(&self, root: &Path) -> Result<Vec<(OsString, OsString)>> {
        let mut environment: Vec<_> = self
            .variables
            .iter()
            .map(|(k, v)| (k.into(), v.into()))
            .collect();
        if !self.path_prepend.is_empty() {
            let mut directories = Vec::new();
            for path in self.path_directories(root) {
                ensure!(
                    path.is_dir(),
                    "{} {}",
                    i18n::tr!(GitEnvironmentMissingPath),
                    path.display()
                );
                directories.push(path);
            }
            if let Some(inherited) = env::var_os("PATH") {
                directories.extend(env::split_paths(&inherited));
            }
            environment.push(("PATH".into(), env::join_paths(directories)?));
        }
        Ok(environment)
    }
}
