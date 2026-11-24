//! Agent selection over explicit streaming protocol adapters.
use super::{Agent, Config, Format, Update};
use crate::cancel::Cancel;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub(crate) fn run(
    config: &Config,
    prompt: &str,
    format: Format<'_>,
    cancel: &Cancel,
    executable: &Path,
    emit: &mut (impl FnMut(Update) + Send),
) -> Result<String> {
    let directory = tempfile::tempdir()?;
    match config.agent {
        Agent::Codex => super::codex::run(
            config,
            prompt,
            format,
            cancel,
            executable,
            directory.path(),
            emit,
        ),
        Agent::Claude => super::claude::run(
            config,
            prompt,
            format,
            cancel,
            executable,
            directory.path(),
            emit,
        ),
    }
}
pub(crate) fn resolve(agent: Agent) -> Result<PathBuf> {
    let command = agent.name();
    let missing = || anyhow::anyhow!("{}: {command}", i18n::tr!(AgentNotFound));
    let path = std::env::var_os("PATH").ok_or_else(missing)?;
    let cwd = std::env::current_dir()?;
    resolve_in(command, &path, &cwd)
}
fn resolve_in(command: &str, path: &std::ffi::OsStr, cwd: &Path) -> Result<PathBuf> {
    crate::platform::files::find_program(command, path, cwd)?
        .with_context(|| format!("{}: {command}", i18n::tr!(AgentNotFound)))
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::{fs, os::unix::fs::PermissionsExt};
    #[test]
    fn path_lookup_uses_only_explicit_order_and_executable_files() {
        let root = tempfile::tempdir().unwrap();
        for directory in ["first", "second"] {
            fs::create_dir(root.path().join(directory)).unwrap();
        }
        let path =
            std::env::join_paths([root.path().join("first"), root.path().join("second")]).unwrap();
        let first = root.path().join("first/codex");
        let second = root.path().join("second/codex");
        fs::write(&first, "not executable").unwrap();
        fs::write(&second, "#!/bin/sh\n").unwrap();
        fs::set_permissions(&second, fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(resolve_in("codex", &path, root.path()).unwrap(), second);
        fs::set_permissions(&first, fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(resolve_in("codex", &path, root.path()).unwrap(), first);
        assert!(resolve_in("claude", &path, root.path()).is_err());
        assert!(
            resolve_in(
                "codex",
                root.path().join("missing").as_os_str(),
                root.path()
            )
            .is_err()
        );
    }
}
