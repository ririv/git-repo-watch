//! Editor preferences follow Git resolution and preserve shell command semantics.
mod catalog;
#[cfg(target_os = "macos")]
mod macos;
pub mod terminal;
use crate::{cancel::Cancel, transport::Git};
use anyhow::{Context, Result, ensure};
pub use catalog::Known;
use serde::{Deserialize, Serialize};
use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interface {
    Desktop,
    Terminal,
}
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub enum Preference {
    #[default]
    Git,
    Known(Known),
    Command(Command),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command(String);
impl Command {
    pub fn parse(value: String) -> Result<Self> {
        ensure!(
            !value.trim().is_empty() && value.len() <= 4096 && !value.contains('\0'),
            i18n::tr!(ExternalEditorInvalid)
        );
        Ok(Self(value))
    }
    pub fn text(&self) -> &str {
        &self.0
    }
    fn interface(&self) -> Interface {
        let Some(words) = shlex::split(&self.0) else {
            return Interface::Terminal;
        };
        if words
            .iter()
            .any(|word| matches!(word.as_str(), "-w" | "--wait"))
            || self
                .0
                .contains([';', '|', '&', '$', '`', '<', '>', '\n', '(', ')'])
        {
            return Interface::Terminal;
        }
        words
            .first()
            .and_then(|word| Path::new(word).file_name())
            .and_then(OsStr::to_str)
            .and_then(Known::executable)
            .map_or(Interface::Terminal, |known| known.spec().interface)
    }
}
impl Preference {
    pub fn value(&self) -> String {
        match self {
            Self::Git => "git".into(),
            Self::Known(known) => known.spec().token.into(),
            Self::Command(command) => command.0.clone(),
        }
    }
    pub fn label(&self) -> String {
        match self {
            Self::Git => i18n::tr!(ExternalEditorGit).into(),
            Self::Known(known) => known.spec().label.into(),
            Self::Command(command) => command.0.clone(),
        }
    }
}
impl TryFrom<String> for Preference {
    type Error = anyhow::Error;
    fn try_from(value: String) -> Result<Self> {
        if value == "git" {
            Ok(Self::Git)
        } else if let Some(known) = Known::named(&value) {
            Ok(Self::Known(known))
        } else {
            Ok(Self::Command(Command::parse(value)?))
        }
    }
}
impl From<Preference> for String {
    fn from(value: Preference) -> Self {
        value.value()
    }
}
#[derive(Debug, Clone)]
pub struct Environment {
    pub path: OsString,
    pub home: PathBuf,
}
impl Environment {
    pub fn capture(home: &Path) -> Self {
        Self {
            path: std::env::var_os("PATH").unwrap_or_default(),
            home: home.to_owned(),
        }
    }
    fn program(&self, name: &str) -> Result<Option<PathBuf>> {
        Ok(crate::platform::files::find_program(
            name, &self.path, &self.home,
        )?)
    }
}
#[derive(Debug, Clone)]
enum Installation {
    Program(PathBuf),
    #[cfg(target_os = "macos")]
    Application(PathBuf),
}
fn installation(
    known: Known,
    environment: &Environment,
    cancel: &Cancel,
) -> Result<Option<Installation>> {
    cancel.check()?;
    let spec = known.spec();
    for program in spec.commands {
        if let Some(path) = environment.program(program)? {
            return Ok(Some(Installation::Program(path)));
        }
    }
    #[cfg(target_os = "macos")]
    if spec.interface == Interface::Desktop
        && let Some(path) = macos::standard_application(known, &environment.home, cancel)?
    {
        return Ok(Some(Installation::Application(path)));
    }
    Ok(None)
}
#[derive(Debug, Clone)]
pub struct Detected {
    pub preference: Preference,
    pub label: String,
    pub detail: String,
}
pub fn detect(
    git: &Git,
    root: &Path,
    environment: &Environment,
    cancel: &Cancel,
) -> Result<Vec<Detected>> {
    let command = git.editor(root, cancel)?;

    let mut result = vec![Detected {
        preference: Preference::Git,
        label: i18n::tr!(ExternalEditorGit).into(),
        detail: command,
    }];
    for known in Known::ALL {
        cancel.check()?;
        if let Some(installation) = installation(*known, environment, cancel)? {
            #[cfg(target_os = "macos")]
            let path = match installation {
                Installation::Program(path) | Installation::Application(path) => path,
            };
            #[cfg(not(target_os = "macos"))]
            let Installation::Program(path) = installation;
            result.push(Detected {
                preference: Preference::Known(*known),
                label: known.spec().label.into(),
                detail: path.display().to_string(),
            });
        }
    }
    Ok(result)
}
#[derive(Debug, Clone)]
pub struct Plan {
    pub program: PathBuf,
    pub arguments: Vec<OsString>,
    pub directory: PathBuf,
    pub interface: Interface,
    pub target: PathBuf,
}
pub fn prepare(
    git: &Git,
    preference: &Preference,
    target: &Path,
    root: &Path,
    environment: &Environment,
    cancel: &Cancel,
) -> Result<Plan> {
    cancel.check()?;
    ensure!(
        target.is_absolute() && root.is_absolute(),
        i18n::tr!(EnterAnAbsolutePath)
    );
    ensure!(
        target.try_exists()?,
        i18n::tf!(LinkPathMissing, target.display())
    );
    let (program, arguments, interface, folders) = match preference {
        Preference::Known(known) => {
            let installation = installation(*known, environment, cancel)?
                .with_context(|| i18n::tf!(ExternalEditorMissing, known.spec().label))?;
            let (program, arguments) = match installation {
                Installation::Program(program) => (program, vec![target.as_os_str().to_owned()]),
                #[cfg(target_os = "macos")]
                Installation::Application(application) => (
                    PathBuf::from("/usr/bin/open"),
                    vec![
                        "-a".into(),
                        application.into_os_string(),
                        target.as_os_str().to_owned(),
                    ],
                ),
            };
            (
                program,
                arguments,
                known.spec().interface,
                known.spec().folders,
            )
        }
        Preference::Git | Preference::Command(_) => {
            let command = match preference {
                Preference::Git => Command::parse(git.editor(root, cancel)?)?,
                Preference::Command(command) => command.clone(),
                _ => unreachable!(),
            };
            #[cfg(unix)]
            let (program, arguments) = (
                PathBuf::from("/bin/sh"),
                vec![
                    "-c".into(),
                    format!("{} \"$@\"", command.text()).into(),
                    command.text().into(),
                    target.as_os_str().to_owned(),
                ],
            );
            // Git for Windows 使用随 Git 安装的 shell 解析编辑器命令。
            #[cfg(windows)]
            let (program, arguments) = (
                crate::transport::program()?,
                vec![
                    "-c".into(),
                    format!("alias.gw-external-editor=!{}", command.text()).into(),
                    "gw-external-editor".into(),
                    target.as_os_str().to_owned(),
                ],
            );
            (program, arguments, command.interface(), true)
        }
    };
    ensure!(
        !target.is_dir() || folders,
        i18n::tr!(ExternalEditorNoFolders)
    );
    Ok(Plan {
        program,
        arguments,
        interface,
        directory: root.to_owned(),
        target: target.to_owned(),
    })
}
impl Plan {
    pub fn launch(&self, cancel: &Cancel) -> Result<crate::desktop::Launch> {
        cancel.check()?;
        ensure!(
            self.interface == Interface::Desktop,
            i18n::tr!(ExternalEditorNeedsTerminal)
        );
        crate::desktop::launch_command(
            &self.program,
            &self.arguments,
            Some(&self.directory),
            self.target.display().to_string(),
        )
    }
}
