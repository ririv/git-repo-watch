//! Clipboard requests carry immutable selections captured by the command draft.
mod selection;
mod worker;
use crate::Screen;
pub(crate) use selection::{Amount, Selection};
pub(crate) use worker::Writer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Scope {
    Browser,
    Repository,
    Diff,
    Log,
    Branch,
    Stash,
    Error,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Target {
    RepositoryPath,
    FilePaths,
    Commit,
    Branch,
    Stash,
    Diff,
    Conflict,
    ConflictResult,
    Error,
}
impl Target {
    pub fn label(self) -> &'static str {
        match self {
            Self::RepositoryPath => i18n::tr!(CopyRepositoryPath),
            Self::FilePaths => i18n::tr!(CopyFilePaths),
            Self::Commit => i18n::tr!(CopyCommit),
            Self::Stash => i18n::tr!(StashObject),
            Self::Branch => i18n::tr!(CopyBranch),
            Self::Diff => i18n::tr!(CopyDiff),
            Self::Conflict => i18n::tr!(CopyConflictVersions),
            Self::ConflictResult => i18n::tr!(CopyConflictResult),
            Self::Error => i18n::tr!(CopyError),
        }
    }
}
pub(crate) struct Payload {
    pub target: Target,
    pub text: String,
}
impl Screen {
    pub(crate) fn copy_to_clipboard(
        &mut self,
        scope: Scope,
        selection: Selection,
        path: Option<std::path::PathBuf>,
    ) {
        match selection
            .payload(scope, path.as_deref())
            .and_then(|payload| self.clipboard.submit(payload))
        {
            Ok(()) => self.message = format!("{}…", i18n::tr!(CopyWriting)),
            Err(error) => self.message = format!("{}: {error:#}", i18n::tr!(CopyFailed)),
        }
    }
    pub(crate) fn poll_clipboard(&mut self) -> bool {
        let Some((target, result)) = self.clipboard.poll() else {
            return false;
        };
        self.message = match result {
            Ok(()) => format!("{}: {}", i18n::tr!(CopySucceeded), target.label()),
            Err(error) => format!("{}: {error}", i18n::tr!(CopyFailed)),
        };
        true
    }
}
