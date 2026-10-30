//! A discovery request owns temporary exclusions; configuration never supplies them.
use anyhow::{Result, ensure};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub root: PathBuf,
    pub excluded: Vec<PathBuf>,
}
impl Request {
    pub fn from_args(args: &[String], home: &Path) -> Result<Self> {
        let Some(root) = args.first() else {
            anyhow::bail!(i18n::tr!(ScanUsage));
        };
        ensure!(
            !root.is_empty() && !root.starts_with("--"),
            i18n::tr!(ScanUsage)
        );
        let root = crate::paths::expand_home(root, home)?;
        let mut excluded = Vec::new();
        if args.len() > 1 {
            ensure!(
                args[1] == "--exclude" && args.len() > 2,
                i18n::tr!(ScanUsage)
            );
            for value in &args[2..] {
                ensure!(
                    !value.starts_with("--") && !value.is_empty(),
                    i18n::tr!(ScanUsage)
                );
                excluded.push(crate::paths::expand_home(value, home)?);
            }
        }
        Ok(Self { root, excluded })
    }
}
