//! Non-interactive CLI adapters; prompts never pass through a shell or process arguments.
use super::{Agent, Config};
use crate::{cancel::Cancel, process};
use anyhow::{Context, Result, ensure};
use std::{
    ffi::OsString,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    time::Duration,
};

const OUTPUT_LIMIT: usize = 64 * 1024;
pub(super) fn run(
    config: &Config,
    prompt: &str,
    cancel: &Cancel,
    executable: &Path,
) -> Result<String> {
    // 临时工作目录不包含仓库文件；只通过标准输入发送已审核的暂存快照。
    let directory = tempfile::tempdir()?;
    let output_path = directory.path().join("message.txt");
    let args: Vec<OsString> = match config.agent {
        Agent::Codex => [
            "exec",
            "--sandbox",
            "read-only",
            "--ephemeral",
            "--skip-git-repo-check",
            "--color",
            "never",
            "-c",
            "approval_policy=\"never\"",
            "-c",
            "features.shell_tool=false",
            "-c",
            "features.apply_patch_freeform=false",
            "-c",
            "features.multi_agent=false",
            "-c",
            "features.apps=false",
            "-c",
            "features.plugins=false",
            "--output-last-message",
        ]
        .into_iter()
        .map(OsString::from)
        .chain([output_path.clone().into_os_string()])
        .collect(),
        Agent::Claude => [
            "--print",
            "--output-format",
            "json",
            "--tools",
            "",
            "--strict-mcp-config",
            "--mcp-config",
            "{\"mcpServers\":{}}",
            "--no-session-persistence",
            "--disable-slash-commands",
            "--no-chrome",
            "--setting-sources",
            "user",
            "--settings",
            "{\"disableAllHooks\":true}",
        ]
        .into_iter()
        .map(OsString::from)
        .collect(),
    };
    let mut args = args;
    let program = config.program();
    if !program.model.is_empty() {
        args.extend([OsString::from("--model"), OsString::from(&program.model)]);
    }
    if config.agent == Agent::Codex {
        args.push("-".into());
    }
    let output = process::run_with_input(
        executable,
        &args,
        directory.path(),
        prompt.as_bytes(),
        cancel,
        Duration::from_secs(120),
        OUTPUT_LIMIT,
    )?;
    let stdout = output.success(config.agent.label())?;
    match config.agent {
        Agent::Codex => read_message(&output_path),
        Agent::Claude => {
            let output: serde_json::Value =
                serde_json::from_slice(&stdout).context(i18n::tr!(AgentMessageInvalid))?;
            ensure!(
                output["subtype"] == "success" && output["is_error"] == false,
                "{}: {}",
                config.agent.label(),
                output
            );
            output["result"]
                .as_str()
                .map(str::to_owned)
                .context(i18n::tr!(AgentMessageInvalid))
        }
    }
}
fn read_message(path: &Path) -> Result<String> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take((OUTPUT_LIMIT + 1) as u64)
        .read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= OUTPUT_LIMIT, i18n::tr!(AgentMessageInvalid));
    Ok(String::from_utf8(bytes)?)
}

pub(super) fn resolve(agent: Agent) -> Result<PathBuf> {
    let command = agent.name();
    let missing = || anyhow::anyhow!("{}: {command}", i18n::tr!(AgentNotFound));
    let path = std::env::var_os("PATH").ok_or_else(missing)?;
    let cwd = std::env::current_dir()?;
    resolve_in(command, &path, &cwd)
}
fn resolve_in(command: &str, path: &std::ffi::OsStr, cwd: &Path) -> Result<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    for directory in std::env::split_paths(&path) {
        let candidate = cwd.join(directory).join(command);
        match std::fs::metadata(&candidate) {
            Ok(metadata) if metadata.is_file() && metadata.permissions().mode() & 0o111 != 0 => {
                return Ok(candidate);
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error).with_context(|| candidate.display().to_string()),
        }
    }
    anyhow::bail!("{}: {command}", i18n::tr!(AgentNotFound))
}

#[cfg(test)]
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
