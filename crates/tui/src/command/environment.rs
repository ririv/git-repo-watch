//! Literal inputs for editing the selected repository's Git environment.
use anyhow::{Context, Result, ensure};
use engine::git_environment::{self as env, Edit, Environment, Launcher};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Path,
    Set,
    Unset,
    Launcher,
}

impl Field {
    pub(super) fn syntax(self) -> &'static str {
        match self {
            Self::Path => "<directory>… | --clear",
            Self::Set => "<NAME=value>",
            Self::Unset => "<NAME>",
            Self::Launcher => "<program> [args…] | --clear",
        }
    }

    pub(crate) fn edit(self, value: &str) -> Result<Edit> {
        let words = env::parse_words(value)?;
        ensure!(!words.is_empty(), i18n::tr!(GitEnvironmentInvalidInput));
        let clear = words == ["--clear"];
        Ok(match self {
            Self::Path => Edit::Path(if clear {
                vec![]
            } else {
                words.into_iter().map(Into::into).collect()
            }),
            Self::Set => {
                let [assignment] = words.as_slice() else {
                    anyhow::bail!(i18n::tr!(GitEnvironmentInvalidInput));
                };
                let (name, value) = assignment
                    .split_once('=')
                    .context(i18n::tr!(GitEnvironmentInvalidInput))?;
                env::validate_variable(name)?;
                Edit::Set {
                    name: name.into(),
                    value: value.into(),
                }
            }
            Self::Unset => {
                let [name] = words.as_slice() else {
                    anyhow::bail!(i18n::tr!(GitEnvironmentInvalidInput));
                };
                env::validate_variable(name)?;
                Edit::Unset(name.clone())
            }
            Self::Launcher if clear => Edit::Launcher(None),
            Self::Launcher => {
                let mut words = words.into_iter();
                let program = words
                    .next()
                    .context(i18n::tr!(GitEnvironmentInvalidInput))?;
                ensure!(!program.is_empty(), i18n::tr!(GitEnvironmentInvalidInput));
                Edit::Launcher(Some(Launcher {
                    program: program.into(),
                    args: words.collect(),
                }))
            }
        })
    }

    pub(crate) fn value(self, environment: Option<&Environment>) -> String {
        let Some(environment) = environment else {
            return String::new();
        };
        match self {
            Self::Path => env::display_words(
                environment
                    .path_prepend
                    .iter()
                    .map(|path| path.to_str().expect("configured path is UTF-8")),
            ),
            Self::Launcher => environment
                .launcher
                .as_ref()
                .map(|launcher| {
                    env::display_words(
                        std::iter::once(
                            launcher
                                .program
                                .to_str()
                                .expect("configured launcher is UTF-8"),
                        )
                        .chain(launcher.args.iter().map(String::as_str)),
                    )
                })
                .unwrap_or_default(),
            Self::Set | Self::Unset => String::new(),
        }
    }
}
