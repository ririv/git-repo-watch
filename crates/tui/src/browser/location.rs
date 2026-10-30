//! File locations expose ancestor navigation independently from view history.
use super::*;

#[derive(Debug, Clone, Copy)]
pub(crate) enum Location<'a> {
    Root,
    Directory(&'a Path),
    File(&'a Path),
}
impl<'a> Location<'a> {
    pub fn path(self) -> &'a Path {
        match self {
            Self::Root => Path::new(""),
            Self::Directory(path) | Self::File(path) => path,
        }
    }
    pub fn directories(self) -> usize {
        self.path()
            .components()
            .count()
            .saturating_sub(usize::from(matches!(self, Self::File(_))))
    }
}

#[derive(Debug, Default)]
pub(super) enum Navigation {
    #[default]
    Idle,
    Pending(PathBuf),
    Located(PathBuf),
    Failed {
        path: PathBuf,
        error: String,
    },
}
impl Navigation {
    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::Idle => None,
            Self::Pending(path) | Self::Located(path) | Self::Failed { path, .. } => Some(path),
        }
    }
}

impl Session {
    pub fn location(&self) -> Location<'_> {
        if let Some(path) = self.navigation.path() {
            return if path.as_os_str().is_empty() {
                Location::Root
            } else {
                Location::Directory(path)
            };
        }
        if let Some(snapshot) = self.panes.feedback.previous() {
            return Location::File(&snapshot.target.path);
        }
        match self.selected_row() {
            Some(row) if row.entry.directory() => Location::Directory(&row.entry.path),
            Some(row) => Location::File(&row.entry.path),
            None => Location::Root,
        }
    }
    pub fn visit_directory(&mut self, depth: usize) -> bool {
        let location = self.location();
        if depth > location.directories() {
            return false;
        }
        let path = location
            .path()
            .components()
            .take(depth)
            .collect::<PathBuf>();
        self.save_position();
        self.query.clear();
        self.search = Search::Idle;
        self.search_reader.clear();
        self.panes.clear();
        self.focus = Focus::Tree;
        self.horizontal.seek(0);
        self.state.select(None);
        for ancestor in path.ancestors() {
            self.expanded.insert(ancestor.to_owned());
            self.load(ancestor.to_owned());
        }
        self.navigation = Navigation::Pending(path);
        self.settle_navigation();
        true
    }
    pub(super) fn settle_navigation(&mut self) -> bool {
        let (path, pending) = match &self.navigation {
            Navigation::Pending(path) => (path.clone(), true),
            Navigation::Located(path) => (path.clone(), false),
            Navigation::Idle | Navigation::Failed { .. } => return false,
        };
        for ancestor in path.ancestors() {
            match self.directories.get(ancestor) {
                Some(Contents::Failed(error)) => {
                    self.navigation = Navigation::Failed {
                        path,
                        error: error.clone(),
                    };
                    return true;
                }
                Some(Contents::Ready(_) | Contents::Refreshing(_)) => {}
                _ => return false,
            }
        }
        let previous = self.state.selected();
        if path.as_os_str().is_empty() {
            self.state.select(None);
            self.navigation = Navigation::Located(path);
        } else if let Some(index) = self.rows().iter().position(|row| row.entry.path == path) {
            self.state.select(Some(index));
            self.navigation = Navigation::Located(path);
        } else {
            self.navigation = Navigation::Failed {
                error: format!("{}: {}", path.display(), i18n::tr!(BrowserMissing)),
                path,
            };
        }
        pending || previous != self.state.selected()
    }
}
