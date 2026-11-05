//! Read checkout identity without graph walks or network access.
use crate::{cancel::Cancel, local::open};
use anyhow::{Context, Result};
use std::path::Path;

#[derive(Debug, Clone)]
pub enum Head {
    Branch(String),
    Detached(String),
    Unborn(String),
}
pub fn read(path: &Path, cancel: &Cancel) -> Result<Head> {
    cancel.check()?;
    let repo = open(path)?;
    let value = match repo.head() {
        Ok(head) if repo.head_detached()? => {
            Head::Detached(head.peel_to_commit()?.id().to_string())
        }
        Ok(head) => Head::Branch(
            head.shorthand()
                .context(i18n::tr!(BranchNameIsNotUtf8))?
                .to_owned(),
        ),
        Err(error) if error.code() == git2::ErrorCode::UnbornBranch => {
            let head = repo.find_reference("HEAD")?;
            Head::Unborn(
                head.symbolic_target()?
                    .context(i18n::tr!(HeadHasNoSymbolicBranch))?
                    .strip_prefix("refs/heads/")
                    .context(i18n::tr!(HeadIsNotAttachedToALocal))?
                    .to_owned(),
            )
        }
        Err(error) => return Err(error.into()),
    };
    cancel.check()?;
    Ok(value)
}
