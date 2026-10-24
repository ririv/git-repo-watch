//! Repository file browsing composes a lazy tree and independent latest-only previews.
mod input;
mod load;
pub(super) mod preview;
mod tree;
mod worker;
use crate::{Service, horizontal, spinner, workspace::Arrangement};
use engine::local::browser::{Directory, Entry, SearchExtent};
use ratatui::widgets::ListState;
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};
use worker::Worker;

impl crate::Screen {
    pub(super) fn poll_browser(&mut self) -> bool {
        if let crate::Mode::Browser(session) = &mut self.mode {
            session.poll(&self.service) | session.animate()
        } else {
            false
        }
    }
    pub(super) fn open_browser_diff(&mut self) {
        let crate::Mode::Browser(session) = &self.mode else {
            return;
        };
        let Some(entry) = session
            .selected()
            .filter(|entry| entry.status().is_some_and(|status| status.changed()))
        else {
            self.message = i18n::tr!(BrowserNoDiff).into();
            return;
        };
        let root = session.root.clone();
        self.start_repository(
            root.clone(),
            i18n::tr!(FileDiff),
            crate::Effect::ReadOnly,
            move |service, cancel, _| {
                let mut files = crate::FileSession::new(service.files(&root, &cancel)?);
                let rows = files.rows();
                let cursor = rows
                    .iter()
                    .position(|change| {
                        change.location.paths().contains(&entry.path.as_path())
                            && change.area != engine::local::files::Area::Staged
                    })
                    .or_else(|| {
                        rows.iter().position(|change| {
                            change.location.paths().contains(&entry.path.as_path())
                        })
                    })
                    .ok_or_else(|| anyhow::anyhow!(i18n::tr!(BrowserNoDiff)))?;
                files.cursor = cursor;
                Ok(crate::Outcome::BrowserDiff { files })
            },
        );
    }
}

#[derive(Debug)]
enum Contents {
    Unloaded,
    Loading,
    Ready(Directory),
    Refreshing(Directory),
    Failed(String),
}
impl Contents {
    fn directory(&self) -> Option<&Directory> {
        match self {
            Self::Ready(directory) | Self::Refreshing(directory) => Some(directory),
            _ => None,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Key {
    revision: u64,
    pub path: PathBuf,
}
#[derive(Debug)]
pub(super) enum Preview {
    Empty,
    Loading(Key),
    Ready {
        key: Key,
        document: preview::Document,
    },
    Failed {
        key: Key,
        error: String,
    },
}
impl Preview {
    fn key(&self) -> Option<&Key> {
        match self {
            Self::Empty => None,
            Self::Loading(key) | Self::Ready { key, .. } | Self::Failed { key, .. } => Some(key),
        }
    }
}
#[derive(Debug)]
enum Search {
    Idle,
    Loading(String),
    Ready {
        query: String,
        entries: Vec<Entry>,
        extent: SearchExtent,
    },
    Failed(String),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Focus {
    Tree,
    Preview,
}
#[derive(Debug, Clone, Copy)]
pub(super) struct Row<'a> {
    pub entry: &'a Entry,
    /// Stable identity of the first directory in a compact chain.
    pub head: &'a Path,
    pub depth: usize,
}
#[derive(Debug)]
pub(super) struct Session {
    pub root: PathBuf,
    directories: BTreeMap<PathBuf, Contents>,
    expanded: BTreeSet<PathBuf>,
    queued: VecDeque<PathBuf>,
    directory_reader: Worker<PathBuf, Directory>,
    preview_reader: Worker<Key, preview::Document>,
    search_reader: Worker<String, engine::local::browser::Search>,
    search: Search,
    pub query: String,
    pub state: ListState,
    pub focus: Focus,
    pub arrangement: Arrangement,
    pub preview: Preview,
    pub scroll: usize,
    pub horizontal: horizontal::Viewport,
    revision: u64,
    animation: spinner::Spinner,
}
impl Session {
    pub fn new(root: PathBuf) -> Self {
        let mut session = Self {
            root,
            directories: BTreeMap::new(),
            expanded: [PathBuf::new()].into(),
            queued: VecDeque::new(),
            directory_reader: Default::default(),
            preview_reader: Default::default(),
            search_reader: Default::default(),
            search: Search::Idle,
            query: String::new(),
            state: ListState::default(),
            focus: Focus::Tree,
            arrangement: Arrangement::Tiled,
            preview: Preview::Empty,
            scroll: 0,
            horizontal: Default::default(),
            revision: 0,
            animation: spinner::Spinner::new(Instant::now()),
        };
        session.load(PathBuf::new());
        session
    }
    fn sync_preview(&mut self) {
        let entry = self.selected().filter(|entry| !entry.directory());
        if let Some(entry) = entry {
            if self.preview.key().is_some_and(|key| key.path == entry.path) {
                return;
            }
            self.revision += 1;
            self.preview = Preview::Loading(Key {
                revision: self.revision,
                path: entry.path,
            });
        } else {
            self.preview = Preview::Empty;
            self.preview_reader.clear();
        }
        self.scroll = 0;
        self.horizontal = Default::default();
    }
    pub fn search(&mut self, query: String) {
        if query == self.query {
            return;
        }
        self.query = query.clone();
        self.search_reader.clear();
        self.search = if query.is_empty() {
            Search::Idle
        } else {
            Search::Loading(query)
        };
        self.state = ListState::default();
        self.focus = Focus::Tree;
        self.preview = Preview::Empty;
        self.preview_reader.clear();
        if self.query.is_empty() {
            self.probe_visible();
            self.select(0);
        }
    }
}
