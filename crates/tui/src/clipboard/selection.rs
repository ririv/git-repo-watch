//! Resolve focus and selection from displayed snapshots without reading Git again.
use super::{Payload, Scope, Target};
use crate::{Action, Mode, Screen, branches, changes, diff, document, history, workspace};
use anyhow::{Context, Result, bail};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Debug, Clone, Copy)]
pub(crate) enum Unavailable {
    Empty,
    Loading,
    Image,
    Truncated,
    NoPatch,
    InvalidPath,
}
impl Unavailable {
    fn message(self) -> &'static str {
        match self {
            Self::Empty => i18n::tr!(CopySelectionRequired),
            Self::Loading => i18n::tr!(CopySelectionLoading),
            Self::Image => i18n::tr!(CopyDiffImage),
            Self::Truncated => i18n::tr!(CopyDiffTruncated),
            Self::NoPatch => i18n::tr!(CopyDiffRequired),
            Self::InvalidPath => i18n::tr!(CopyPathNotUtf8),
        }
    }
}
#[derive(Debug, Clone)]
pub(crate) enum Content {
    Text { target: Target, text: Arc<str> },
    Unavailable(Unavailable),
}
impl Content {
    fn text(target: Target, text: impl Into<Arc<str>>) -> Self {
        Self::Text {
            target,
            text: text.into(),
        }
    }
    fn paths(paths: impl IntoIterator<Item = PathBuf>) -> Self {
        let paths = paths.into_iter().collect::<Vec<_>>();
        if paths.is_empty() {
            return Self::Unavailable(Unavailable::Empty);
        }
        let Some(paths) = paths.iter().map(|p| p.to_str()).collect::<Option<Vec<_>>>() else {
            // 路径必须能无损复制，不能用替代字符修改文件名。
            return Self::Unavailable(Unavailable::InvalidPath);
        };
        Self::text(Target::FilePaths, paths.join("\n"))
    }
    fn document(content: &document::View) -> Self {
        match content {
            document::View::Images(_) => Self::Unavailable(Unavailable::Image),
            document::View::Text(view) => match &view.export {
                diff::Export::Complete(text) if text.is_empty() => {
                    Self::Unavailable(Unavailable::NoPatch)
                }
                diff::Export::Complete(text) => Self::text(Target::Diff, text.clone()),
                diff::Export::Truncated => Self::Unavailable(Unavailable::Truncated),
                diff::Export::Unavailable => Self::Unavailable(Unavailable::NoPatch),
            },
        }
    }
}
#[derive(Debug, Clone, Default)]
pub(crate) enum Selection {
    #[default]
    None,
    Browser {
        content: Content,
        opener: Option<crate::browser::opening::OpenTarget>,
    },
    Diff(Content),
    Log(Content),
    Branch(Content),
    Stash(Content),
    Error(Content),
}
impl Selection {
    fn content(&self, scope: Scope) -> Option<&Content> {
        match (self, scope) {
            (Self::Browser { content, .. }, Scope::Browser) => Some(content),
            (Self::Diff(content), Scope::Diff)
            | (Self::Log(content), Scope::Log)
            | (Self::Branch(content), Scope::Branch)
            | (Self::Stash(content), Scope::Stash)
            | (Self::Error(content), Scope::Error) => Some(content),
            _ => None,
        }
    }
    pub fn complete(&self, input: &mut crate::command::Input, command: &crate::command::Command) {
        input.complete(command);
        if matches!(
            command.action,
            Action::StashApply | Action::StashPop | Action::StashDrop
        ) && let Self::Stash(Content::Text { text, .. }) = self
        {
            input.editor_mut().insert(text);
        }
    }
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error(_))
    }
    pub fn opener(&self) -> Option<&crate::browser::opening::OpenTarget> {
        match self {
            Self::Browser { opener, .. } => opener.as_ref(),
            _ => None,
        }
    }
    pub fn admits(&self, action: Action, path: Option<&Path>) -> bool {
        if self.is_error() {
            return matches!(action, Action::Copy(Scope::Error) | Action::Quit);
        }
        match action {
            Action::BrowserToggle | Action::BrowserSplit => self.opener().is_some(),
            Action::BrowserSource => self.opener().is_some_and(|target| {
                target.presentation
                    != crate::browser::opening::Presentation::Single(
                        crate::browser::opening::TextMode::Source,
                    )
            }),
            Action::BrowserPreview => self.opener().is_some_and(|target| {
                target.presentation
                    != crate::browser::opening::Presentation::Single(
                        crate::browser::opening::TextMode::Preview,
                    )
            }),
            Action::Copy(Scope::Repository) => path.is_some(),
            Action::Copy(scope) => self.content(scope).is_some(),
            _ => true,
        }
    }
    pub fn annotation(&self, action: Action) -> Option<&'static str> {
        match action {
            Action::BrowserToggle => self
                .opener()
                .map(|target| target.opening(target.presentation.active()).label()),
            Action::BrowserSplit => self.opener().map(|target| match target.presentation {
                crate::browser::opening::Presentation::Single(_) => i18n::tr!(FileSingle),
                crate::browser::opening::Presentation::Split(_) => i18n::tr!(FileSplit),
            }),
            Action::Copy(Scope::Repository) => Some(Target::RepositoryPath.label()),
            Action::Copy(scope) => match self.content(scope)? {
                Content::Text { target, .. } => Some(target.label()),
                Content::Unavailable(_) => Some(i18n::tr!(CopyUnavailable)),
            },
            _ => None,
        }
    }
    pub fn payload(&self, scope: Scope, path: Option<&Path>) -> Result<Payload> {
        if scope == Scope::Repository {
            return Ok(Payload {
                target: Target::RepositoryPath,
                text: path
                    .context(i18n::tr!(SelectARepositoryFirst))?
                    .to_str()
                    .context(i18n::tr!(CopyPathNotUtf8))?
                    .to_owned(),
            });
        }
        match self
            .content(scope)
            .context(i18n::tr!(CopySelectionRequired))?
        {
            Content::Text { target, text } => Ok(Payload {
                target: *target,
                text: text.to_string(),
            }),
            Content::Unavailable(reason) => bail!(reason.message()),
        }
    }
}
impl Screen {
    pub(crate) fn copy_selection(&self) -> Selection {
        match &self.mode {
            Mode::Browser(session) => Selection::Browser {
                content: Content::paths(session.selected().map(|entry| entry.path)),
                opener: session.open_target(),
            },
            Mode::Files(files) => Selection::Diff(file_paths(files)),
            Mode::Changes(session) => Selection::Diff(match session.focus {
                changes::Focus::Files => file_paths(&session.files),
                changes::Focus::Preview => match &session.preview {
                    changes::Preview::Empty => Content::Unavailable(Unavailable::Empty),
                    changes::Preview::Loading(_) => Content::Unavailable(Unavailable::Loading),
                    changes::Preview::Failed { error, .. } => {
                        Content::text(Target::Error, error.clone())
                    }
                    changes::Preview::Ready { content, .. } => Content::document(content),
                },
            }),
            Mode::History(view) => match view.as_ref() {
                history::View::Graph(session) => Selection::Log(commit(session)),
                history::View::Commit(commit) => Selection::Log(Content::paths(
                    commit
                        .files
                        .selected()
                        .and_then(|i| commit.rows().get(i).copied())
                        .map(|change| change.location.name().to_owned()),
                )),
                history::View::Diff { content, .. } => Selection::Diff(Content::document(content)),
            },
            Mode::Stashes(view) => Selection::Stash(match view.selected() {
                Some(entry) => Content::text(Target::Stash, entry.oid.to_string()),
                None => Content::Unavailable(Unavailable::Empty),
            }),
            Mode::Branches(view) => Selection::Branch(match view.as_ref() {
                branches::View::List(session) => branch(session),
                branches::View::Detail { entry, .. } => {
                    Content::text(Target::Branch, entry.name.clone())
                }
            }),
            Mode::Error { detail, .. } => {
                Selection::Error(Content::text(Target::Error, detail.clone()))
            }
            Mode::Panel(section) => {
                let session = self
                    .workspace
                    .session()
                    .expect("repository pane has an open workspace");
                match section {
                    workspace::Section::Worktree => {
                        Selection::Diff(panel(&session.files, file_paths))
                    }
                    workspace::Section::History => Selection::Log(panel(&session.history, commit)),
                    workspace::Section::Branches => {
                        Selection::Branch(panel(&session.branches, branch))
                    }
                    workspace::Section::Overview => Selection::None,
                }
            }
            _ => Selection::None,
        }
    }
}
fn file_paths(files: &crate::FileSession) -> Content {
    // 与暂存的勾选语义一致；相同路径同时在两组出现时只复制一次。
    let mut paths = Vec::new();
    let mut seen = BTreeSet::new();
    for change in files.selection() {
        let path = change.location.name().to_owned();
        if seen.insert(path.clone()) {
            paths.push(path);
        }
    }
    Content::paths(paths)
}
fn commit(session: &history::Session) -> Content {
    match session.selected() {
        Some(entry) => Content::text(Target::Commit, entry.oid.to_string()),
        None => Content::Unavailable(Unavailable::Empty),
    }
}
fn branch(session: &branches::Session) -> Content {
    match session.selected() {
        Some(entry) => Content::text(Target::Branch, entry.name.clone()),
        None => Content::Unavailable(Unavailable::Empty),
    }
}
fn panel<T>(panel: &workspace::Panel<T>, select: impl FnOnce(&T) -> Content) -> Content {
    match panel {
        workspace::Panel::Ready(value) => select(value),
        workspace::Panel::Loading => Content::Unavailable(Unavailable::Loading),
        workspace::Panel::Failed(error) => Content::text(Target::Error, error.clone()),
    }
}
