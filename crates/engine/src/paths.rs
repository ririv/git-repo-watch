//! Explicit user-home expansion shared by command adapters and path completion.
use anyhow::{Result, bail, ensure};
use std::path::{Path, PathBuf};

/// Expand only `~` and `~/…`; keep all other path characters literal.
pub fn expand_home(value: &str, user_home: &Path) -> Result<PathBuf> {
    let suffix = match value.strip_prefix('~') {
        None => return Ok(value.into()),
        Some("") => "",
        Some(suffix) if suffix.starts_with('/') => suffix.trim_start_matches('/'),
        Some(_) => bail!(i18n::tr!(UnsupportedNamedHome)),
    };
    ensure!(
        user_home.is_absolute(),
        i18n::tr!(UserHomeDirectoryIsNotAnAbsolute)
    );
    Ok(user_home.join(suffix))
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_support::fixture_path;

    #[test]
    fn home_prefix_expands_without_evaluating_other_path_characters() {
        let home = Path::new(fixture_path!("/example/用户 目录"));
        for value in ["~", "~/", "~//"] {
            assert_eq!(expand_home(value, home).unwrap(), home);
        }
        for (value, suffix) in [
            ("~/Proj", "Proj"),
            ("~//Proj", "Proj"),
            (
                "~/中文 空格/$HOME/[x]/$(literal)/~other",
                "中文 空格/$HOME/[x]/$(literal)/~other",
            ),
        ] {
            assert_eq!(expand_home(value, home).unwrap(), home.join(suffix));
        }
        for value in ["/absolute/~", "relative/~", "$HOME/Proj"] {
            assert_eq!(expand_home(value, home).unwrap(), Path::new(value));
        }
        assert_eq!(
            expand_home("~/Proj", Path::new(fixture_path!("/"))).unwrap(),
            Path::new(fixture_path!("/Proj"))
        );
    }

    #[test]
    fn named_users_and_invalid_home_are_explicit_errors() {
        for value in ["~someone", "~someone/Proj", "~\\Proj"] {
            assert!(expand_home(value, Path::new(fixture_path!("/example/home"))).is_err());
        }
        for home in [Path::new(""), Path::new("relative")] {
            assert!(expand_home("~/Proj", home).is_err());
            assert_eq!(
                expand_home("/absolute", home).unwrap(),
                Path::new("/absolute")
            );
        }
    }
}
