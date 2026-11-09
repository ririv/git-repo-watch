//! Resolve focus and selection from displayed snapshots without reading Git again.
use super::{Payload, Scope, Target};
use crate::{
    Action, Mode, Screen, branches, changes,
    command::target::{File, HistoricalFile, Status},
    diff, document, history, workspace,
};
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
    ConflictVersions,
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
            Self::ConflictVersions => i18n::tr!(CopyConflictUnavailable),
        }
    }
}
#[derive(Debug, Clone)]
pub(crate) enum Content {
    Text { target: Target, text: Arc<str> },
    Unavailable(Unavailable),
}
pub(crate) enum Amount {
    Files(usize),
    Commits(usize),
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
            document::View::Conflict(view) => match view.focus() {
                crate::conflict::Focus::Compare => match view.active_comparison() {
                    engine::local::files::conflict::Comparison::Text(_) => {
                        Self::text(Target::Conflict, view.export())
                    }
                    _ => Self::Unavailable(Unavailable::ConflictVersions),
                },
                crate::conflict::Focus::Result => match &view.result.content {
                    crate::conflict::result::Content::Source {
                        body: crate::conflict::Body::Text { source, .. },
                        ..
                    } => Self::text(Target::ConflictResult, source.clone()),
                    crate::conflict::result::Content::Source { .. } => {
                        Self::Unavailable(Unavailable::ConflictVersions)
                    }
                    crate::conflict::result::Content::Document(document) => {
                        Self::document(document)
                    }
                },
            },
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
#[derive(Debug, Clone)]
pub(crate) enum CommitSelection {
    Graph {
        oid: engine::local::history::Oid,
        marked: Vec<engine::local::history::Oid>,
        content: Content,
    },
    Selected {
        oid: engine::local::history::Oid,
        marked: Vec<engine::local::history::Oid>,
        content: Content,
        file: Option<HistoricalFile>,
    },
    Unavailable(Content),
}
impl CommitSelection {
    fn content(&self) -> &Content {
        match self {
            Self::Graph { content, .. }
            | Self::Selected { content, .. }
            | Self::Unavailable(content) => content,
        }
    }
    fn oid(&self) -> Option<engine::local::history::Oid> {
        match self {
            Self::Graph { oid, .. } | Self::Selected { oid, .. } => Some(*oid),
            Self::Unavailable(_) => None,
        }
    }
}
#[derive(Debug, Clone, Default)]
pub(crate) enum Selection {
    #[default]
    None,
    Repository,
    Browser {
        working: Option<crate::working_target::Target>,
        content: Content,
        opener: Option<crate::browser::opening::OpenTarget>,
    },
    Diff {
        working: Option<crate::working_target::Target>,
        content: Content,
        marked: Vec<engine::local::files::Location>,
        diff_selection: Option<changes::SelectionKey>,
    },
    HistoricalDiff {
        content: Content,
        file: Option<HistoricalFile>,
    },
    Log(CommitSelection),
    Branch(Content),
    Stash {
        content: Content,
        file: Option<HistoricalFile>,
    },
    Error(Content),
}
impl Selection {
    pub fn diff_selection(&self) -> Option<&changes::SelectionKey> {
        match self {
            Self::Diff { diff_selection, .. } => diff_selection.as_ref(),
            _ => None,
        }
    }

    pub fn copy_scope(&self) -> Option<Scope> {
        Some(match self {
            Self::None => return None,
            Self::Repository => Scope::Repository,
            Self::Browser { .. } => Scope::Browser,
            Self::Diff { .. } | Self::HistoricalDiff { .. } => Scope::Diff,
            Self::Log(_) => Scope::Log,
            Self::Branch(_) => Scope::Branch,
            Self::Stash { .. } => Scope::Stash,
            Self::Error(_) => Scope::Error,
        })
    }
    pub fn terminal_target(&self, root: &Path) -> Option<PathBuf> {
        match self {
            Self::Repository
            | Self::Log(_)
            | Self::HistoricalDiff { .. }
            | Self::Branch(_)
            | Self::Stash { .. } => Some(root.to_owned()),
            _ => self.working_target().map(|target| target.directory(root)),
        }
    }
    pub fn editor_target(&self, root: &Path) -> Option<PathBuf> {
        match self {
            Self::Repository => Some(root.to_owned()),
            _ => self.working_target().map(|target| target.path(root)),
        }
    }

    fn working_target(&self) -> Option<&crate::working_target::Target> {
        match self {
            Self::Browser { working, .. } | Self::Diff { working, .. } => working.as_ref(),
            _ => None,
        }
    }

    pub fn file_target(&self) -> Option<File<'_>> {
        match self {
            Self::HistoricalDiff { file, .. }
            | Self::Log(CommitSelection::Selected { file, .. })
            | Self::Stash { file, .. } => file.as_ref().map(File::Historical),
            _ => self.working_target().map(File::Working),
        }
    }

    pub fn unavailable(&self, scope: Scope, path: Option<&Path>) -> Option<&'static str> {
        if scope == Scope::Repository {
            return path.is_none().then(|| i18n::tr!(SelectARepositoryFirst));
        }
        match self.content(scope) {
            Some(Content::Text { .. }) => None,
            Some(Content::Unavailable(reason)) => Some(reason.message()),
            None => Some(i18n::tr!(CopySelectionRequired)),
        }
    }
    pub fn amount(&self, scope: Scope) -> Option<Amount> {
        match self.content(scope)? {
            Content::Text {
                target: Target::FilePaths,
                text,
            } => Some(Amount::Files(text.lines().count())),
            Content::Text {
                target: Target::Commit,
                text,
            } => Some(Amount::Commits(text.lines().count())),
            _ => None,
        }
    }
    fn content(&self, scope: Scope) -> Option<&Content> {
        match (self, scope) {
            (Self::Browser { content, .. }, Scope::Browser) => Some(content),
            (Self::Log(selection), Scope::Log) => Some(selection.content()),
            (Self::Diff { content, .. }, Scope::Diff)
            | (Self::HistoricalDiff { content, .. }, Scope::Diff)
            | (Self::Branch(content), Scope::Branch)
            | (Self::Stash { content, .. }, Scope::Stash)
            | (Self::Error(content), Scope::Error) => Some(content),
            _ => None,
        }
    }
    pub fn complete(&self, input: &mut crate::command::Input, command: &crate::command::Command) {
        input.complete(command);
        if matches!(
            command.action,
            Action::StashApply | Action::StashPop | Action::StashDrop
        ) && let Self::Stash {
            content: Content::Text { text, .. },
            ..
        } = self
        {
            input.editor_mut().insert(text);
        }
    }
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error(_))
    }
    pub fn marked_file_count(&self) -> usize {
        match self {
            Self::Diff { marked, .. } => marked
                .iter()
                .map(|location| location.name())
                .collect::<BTreeSet<_>>()
                .len(),
            _ => 0,
        }
    }
    pub fn marked_files(&self) -> Vec<PathBuf> {
        match self {
            Self::Diff { marked, .. } => marked
                .iter()
                .flat_map(|location| location.paths().into_iter().map(Path::to_owned))
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
            _ => vec![],
        }
    }
    pub fn opener(&self) -> Option<&crate::browser::opening::OpenTarget> {
        match self {
            Self::Browser { opener, .. } => opener.as_ref(),
            _ => None,
        }
    }
    pub fn commit(&self) -> Option<engine::local::history::Oid> {
        match self {
            Self::Log(selection) => selection.oid(),
            _ => None,
        }
    }
    pub fn commits(&self) -> Vec<engine::local::history::Oid> {
        match self {
            Self::Log(
                CommitSelection::Graph { marked, .. } | CommitSelection::Selected { marked, .. },
            ) if !marked.is_empty() => marked.clone(),
            _ => self.commit().into_iter().collect(),
        }
    }
    pub(crate) fn history_selection_status(&self) -> Status {
        match self {
            Self::Log(CommitSelection::Graph { .. } | CommitSelection::Unavailable(_)) => {
                Status::Present
            }
            _ => Status::OutOfScope(i18n::tr!(CommandRequiresHistory)),
        }
    }

    pub(crate) fn commit_selection_status(&self) -> Status {
        match self {
            Self::Log(selection) if selection.oid().is_some() => Status::Present,
            Self::Log(_) => Status::Unavailable(i18n::tr!(ReplaySelectCommit)),
            _ => Status::OutOfScope(i18n::tr!(CommandRequiresHistory)),
        }
    }

    pub fn annotation(&self, action: Action) -> Option<&'static str> {
        match action {
            Action::BrowserModeMenu => self.opener().map(|target| match target.presentation {
                crate::browser::opening::Presentation::Single(mode) => target.opening(mode).label(),
                crate::browser::opening::Presentation::Split(_) => i18n::tr!(FileSplit),
            }),
            Action::BrowserLayoutMenu => self.opener().map(|target| match target.presentation {
                crate::browser::opening::Presentation::Single(_) => i18n::tr!(FileSingle),
                crate::browser::opening::Presentation::Split(_) => i18n::tr!(FileSplit),
            }),
            Action::Copy => Some(match self.copy_scope() {
                Some(Scope::Repository) => Target::RepositoryPath.label(),
                Some(scope) => match self.content(scope) {
                    Some(Content::Text { target, .. }) => target.label(),
                    _ => i18n::tr!(CopyUnavailable),
                },
                None => i18n::tr!(CopyUnavailable),
            }),
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
                working: session
                    .selected()
                    .map(crate::working_target::Target::browser),
                content: Content::paths(session.selected().map(|entry| entry.path)),
                opener: session.open_target(),
            },
            Mode::Files(files) => Selection::Diff {
                working: files.working_target(),
                content: file_paths(files),
                marked: marked_paths(files),
                diff_selection: None,
            },
            Mode::Changes(session) => Selection::Diff {
                working: session.files.working_target(),
                marked: marked_paths(&session.files),
                diff_selection: if session.focus == changes::Focus::Preview {
                    session.selection_key()
                } else {
                    None
                },
                content: match session.focus {
                    changes::Focus::Files => file_paths(&session.files),
                    changes::Focus::Preview => match &session.preview {
                        changes::Preview::Empty => Content::Unavailable(Unavailable::Empty),
                        changes::Preview::Directory { path, .. } => {
                            Content::paths(std::iter::once(path.clone()))
                        }
                        changes::Preview::Loading(_) => Content::Unavailable(Unavailable::Loading),
                        changes::Preview::Failed { error, .. } => {
                            Content::text(Target::Error, error.clone())
                        }
                        changes::Preview::Ready { content, .. } => Content::document(content),
                    },
                },
            },
            Mode::History(view) => match view.as_ref() {
                history::View::Graph(session) => log(session),
                history::View::Commit(commit) => Selection::Log(CommitSelection::Selected {
                    oid: commit.inspect.detail.oid(),
                    marked: commit.graph.picks(),
                    file: historical_file(&commit.inspect, false),
                    content: Content::paths(
                        commit
                            .inspect
                            .files
                            .selected()
                            .and_then(|i| commit.inspect.rows().get(i).copied())
                            .map(|change| change.location.name().to_owned()),
                    ),
                }),
                history::View::Diff {
                    commit, content, ..
                } => Selection::HistoricalDiff {
                    content: Content::document(content),
                    file: historical_file(&commit.inspect, true),
                },
            },
            Mode::Stashes(view) => match view.as_ref() {
                crate::stashes::View::Diff {
                    inspect, content, ..
                } => Selection::HistoricalDiff {
                    content: Content::document(content),
                    file: historical_file(inspect, true),
                },
                other => Selection::Stash {
                    content: match other.selected() {
                        Some(entry) => Content::text(Target::Stash, entry.oid.to_string()),
                        None => Content::Unavailable(Unavailable::Empty),
                    },
                    file: match other {
                        crate::stashes::View::Detail { inspect, .. } => {
                            historical_file(inspect, false)
                        }
                        _ => None,
                    },
                },
            },
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
                    workspace::Section::Worktree => Selection::Diff {
                        diff_selection: None,
                        working: match &session.files {
                            workspace::Panel::Ready(files) => files.working_target(),
                            _ => None,
                        },
                        content: panel(&session.files, file_paths),
                        marked: match &session.files {
                            workspace::Panel::Ready(files) => marked_paths(files),
                            _ => vec![],
                        },
                    },
                    workspace::Section::History => match &session.history {
                        workspace::Panel::Ready(session) => log(session),
                        other => Selection::Log(CommitSelection::Unavailable(panel(other, commit))),
                    },
                    workspace::Section::Branches => {
                        Selection::Branch(panel(&session.branches, branch))
                    }
                    workspace::Section::Overview => Selection::Repository,
                }
            }
            Mode::Management(_) => Selection::Repository,
            Mode::Browsing
                if matches!(self.page, crate::Page::Repositories | crate::Page::Detail) =>
            {
                Selection::Repository
            }
            _ => Selection::None,
        }
    }
}
fn historical_file(inspect: &crate::inspection::Session, opened: bool) -> Option<HistoricalFile> {
    use crate::inspection::Focus;
    let selected = inspect.selected_index();
    let index = match (opened, inspect.focus) {
        (true, _) | (_, Focus::Files) => selected,
        (_, Focus::Preview) => inspect.review.visible_file(selected),
        (_, Focus::Message) => None,
    }?;
    Some(HistoricalFile {
        object: inspect.detail.oid(),
        path: inspect.detail.change(index).location.name().to_owned(),
    })
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

fn log(session: &history::Session) -> Selection {
    Selection::Log(match session.selected() {
        Some(entry) => CommitSelection::Graph {
            oid: entry.oid,
            marked: session.picks(),
            content: if session.selection.count() == 0 {
                commit(session)
            } else {
                Content::text(
                    Target::Commit,
                    session
                        .picks()
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join("\n"),
                )
            },
        },
        None => CommitSelection::Unavailable(Content::Unavailable(Unavailable::Empty)),
    })
}

fn marked_paths(files: &crate::FileSession) -> Vec<engine::local::files::Location> {
    files
        .snapshot
        .changes
        .iter()
        .enumerate()
        .filter(|(index, _)| files.selected.contains(index))
        .map(|(_, change)| change.location.clone())
        .collect()
}
