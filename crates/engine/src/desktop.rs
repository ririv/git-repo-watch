//! Explicit HTTP and local-file targets opened by the platform's default application.
use crate::cancel::Cancel;
use anyhow::{Context, Result, ensure};
use std::{
    ffi::OsString,
    fs::File,
    io::{Read, Seek, SeekFrom},
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Web(String);
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Local(PathBuf);
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Web(Web),
    Local(Local),
}
impl Target {
    pub fn reference(value: &str, base: Option<&Path>) -> Option<Self> {
        if value.is_empty() || value.chars().any(char::is_control) {
            return None;
        }
        if let Some(rest) = value
            .strip_prefix("https://")
            .or_else(|| value.strip_prefix("http://"))
        {
            let authority = rest.split(['/', '?', '#']).next()?;
            if authority.is_empty() || authority.chars().any(char::is_whitespace) {
                return None;
            }
            return Some(Self::Web(Web(value.to_owned())));
        }
        if let Some(rest) = value.strip_prefix("file://") {
            let path = rest.strip_prefix("localhost").unwrap_or(rest);
            if !path.starts_with('/') || path.contains(['?', '#']) {
                return None;
            }
            return Self::path(decode_path(path)?);
        }
        if value.starts_with('#') || value.contains("://") || value.split('/').next()?.contains(':')
        {
            return None;
        }
        let value = value.split(['#', '?']).next()?;
        if value.is_empty() {
            return None;
        }
        let decoded = decode_path(value)?;
        let path = if let Some(tail) = value.strip_prefix("~/") {
            PathBuf::from(std::env::var_os("HOME")?).join(tail)
        } else {
            let path = decoded.as_path();
            if path.is_absolute() {
                path.to_owned()
            } else {
                base?.join(path)
            }
        };
        Self::path(path)
    }
    pub fn path(path: PathBuf) -> Option<Self> {
        (path.is_absolute()
            && !path
                .as_os_str()
                .as_encoded_bytes()
                .iter()
                .any(|byte| *byte < 32 || *byte == 127))
        .then_some(Self::Local(Local(path)))
    }
    pub fn uri(&self) -> String {
        match self {
            Self::Web(Web(value)) => value.clone(),
            Self::Local(Local(path)) => {
                let mut uri = String::from("file://");
                for byte in path.as_os_str().as_encoded_bytes() {
                    if byte.is_ascii_alphanumeric() || b"/-._~".contains(byte) {
                        uri.push(*byte as char);
                    } else {
                        use std::fmt::Write;
                        write!(uri, "%{byte:02X}").expect("write to String");
                    }
                }
                uri
            }
        }
    }
    pub fn label(&self) -> String {
        match self {
            Self::Web(Web(value)) => value.clone(),
            Self::Local(Local(path)) => path.display().to_string(),
        }
    }
    fn argument(&self) -> OsString {
        match self {
            Self::Web(Web(value)) => value.into(),
            Self::Local(Local(path)) => path.as_os_str().to_owned(),
        }
    }
}
fn decode_path(value: &str) -> Option<PathBuf> {
    let mut bytes = Vec::new();
    let raw = value.as_bytes();
    let mut i = 0;
    while i < raw.len() {
        if raw[i] == b'%' {
            let pair = std::str::from_utf8(raw.get(i + 1..i + 3)?).ok()?;
            bytes.push(u8::from_str_radix(pair, 16).ok()?);
            i += 3;
        } else {
            bytes.push(raw[i]);
            i += 1;
        }
    }
    if bytes.iter().any(|byte| *byte < 32 || *byte == 127) {
        return None;
    }
    use std::os::unix::ffi::OsStringExt;
    Some(PathBuf::from(OsString::from_vec(bytes)))
}
#[cfg(test)]
mod tests;
#[derive(Debug)]
pub struct Launch {
    target: String,
    child: Child,
    error: File,
}
impl Launch {
    pub fn poll(&mut self) -> Result<Option<()>> {
        let Some(status) = self.child.try_wait()? else {
            return Ok(None);
        };
        if !status.success() {
            self.error.seek(SeekFrom::Start(0))?;
            let mut message = String::new();
            self.error
                .by_ref()
                .take(8192)
                .read_to_string(&mut message)?;
            anyhow::bail!("{}: {} ({status})", self.target, message.trim());
        }
        Ok(Some(()))
    }
}
pub fn launch(target: &Target, cancel: &Cancel) -> Result<Launch> {
    cancel.check()?;
    if let Target::Local(Local(path)) = target {
        let exists = path
            .try_exists()
            .with_context(|| format!("{}: {}", i18n::tr!(LinkOpenFailed), path.display()))?;
        ensure!(exists, i18n::tf!(LinkPathMissing, path.display()));
    }
    #[cfg(target_os = "macos")]
    let program = Path::new("/usr/bin/open");
    #[cfg(target_os = "linux")]
    let program = Path::new("xdg-open");
    launch_using(program, target)
}
fn launch_using(program: &Path, target: &Target) -> Result<Launch> {
    launch_command(program, &[target.argument()], None, target.label())
}
pub(crate) fn launch_command(
    program: &Path,
    arguments: &[OsString],
    directory: Option<&Path>,
    target: String,
) -> Result<Launch> {
    let error = tempfile::tempfile()?;
    // 桌面应用接管后可能长期驻留，不复用 Git 命令的进程组终止与超时回收。
    let mut command = Command::new(program);
    if let Some(directory) = directory {
        command.current_dir(directory);
    }
    let child = command
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(error.try_clone()?)
        .process_group(0)
        .spawn()
        .with_context(|| i18n::tf!(UnableToStart, program.display()))?;
    Ok(Launch {
        target,
        child,
        error,
    })
}
