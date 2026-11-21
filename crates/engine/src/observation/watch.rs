//! Native filesystem notifications coalesced into bounded repository invalidations.
use anyhow::Result;
use notify::{Event, EventKind, RecursiveMode, Watcher};
use std::{
    collections::{BTreeMap, BTreeSet},
    io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, mpsc},
    thread,
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Impact {
    Worktree,
    References,
}
#[derive(Debug, Clone)]
pub struct Change {
    pub impact: Impact,
    paths: BTreeSet<PathBuf>,
    first: Instant,
    last: Instant,
}
impl Change {
    pub fn all() -> Self {
        Self {
            impact: Impact::References,
            paths: BTreeSet::new(),
            first: Instant::now(),
            last: Instant::now(),
        }
    }
    pub fn merge(&mut self, other: Self) {
        if other.impact == Impact::References {
            self.impact = Impact::References;
        }
        // 空集合代表整仓失效；事件风暴不累积无限路径。
        if self.paths.is_empty()
            || other.paths.is_empty()
            || self.paths.len() + other.paths.len() > 32
        {
            self.paths.clear();
        } else {
            self.paths.extend(other.paths);
        }
        self.first = self.first.min(other.first);
        self.last = self.last.max(other.last);
    }
    pub fn relevant(&self, root: &Path) -> Result<bool> {
        if self.impact == Impact::References || self.paths.is_empty() {
            return Ok(true);
        }
        let repo = crate::local::open(root)?;
        let index = repo.index()?;
        for path in &self.paths {
            let relative = path.strip_prefix(root)?;
            if index.get_path(relative, 0).is_some() || !repo.status_should_ignore(relative)? {
                return Ok(true);
            }
        }
        Ok(false)
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct Root {
    path: PathBuf,
    git: PathBuf,
    common: PathBuf,
}
impl Root {
    fn open(path: PathBuf) -> Result<Self> {
        let repo = crate::local::open(&path)?;
        Ok(Self {
            git: std::fs::canonicalize(repo.path())?,
            common: crate::local::common_dir(&repo)?,
            path,
        })
    }
    fn classify(&self, path: &Path) -> Option<Impact> {
        for dir in [&self.git, &self.common] {
            if let Ok(relative) = path.strip_prefix(dir) {
                let first = relative
                    .components()
                    .next()
                    .map(|part| part.as_os_str().to_string_lossy());
                return match first.as_deref() {
                    Some("objects" | "logs") => None,
                    Some("modules") => Some(Impact::Worktree),
                    Some(name) if name.ends_with(".lock") => None,
                    Some(
                        "index" | "MERGE_HEAD" | "MERGE_MSG" | "CHERRY_PICK_HEAD" | "REVERT_HEAD"
                        | "sequencer" | "rebase-merge" | "rebase-apply",
                    ) => Some(Impact::Worktree),
                    Some(
                        "HEAD" | "refs" | "packed-refs" | "config" | "config.worktree" | "shallow"
                        | "commondir",
                    )
                    | None => Some(Impact::References),
                    _ => None,
                };
            }
        }
        if path == self.path.join(".git") || path == self.path {
            Some(Impact::References)
        } else if path.starts_with(&self.path) {
            Some(Impact::Worktree)
        } else {
            None
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnavailableReason {
    MissingPath,
    PermissionDenied,
    RepositoryNotFound,
    OpenFailed,
}
impl UnavailableReason {
    pub fn label(self) -> &'static str {
        match self {
            Self::MissingPath => i18n::tr!(RepositoryPathMissing),
            Self::PermissionDenied => i18n::tr!(RepositoryReadDenied),
            Self::RepositoryNotFound => i18n::tr!(RepositoryMissingAtPath),
            Self::OpenFailed => i18n::tr!(LocalRepositoryOpenFailed),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Unavailable {
    pub reason: UnavailableReason,
    pub detail: String,
}
impl Unavailable {
    fn from_error(path: &Path, error: anyhow::Error) -> Self {
        use UnavailableReason::*;
        let reason = match std::fs::metadata(path) {
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
                ) =>
            {
                MissingPath
            }
            Err(error) if error.kind() == io::ErrorKind::PermissionDenied => PermissionDenied,
            _ if error
                .downcast_ref::<io::Error>()
                .is_some_and(|error| error.kind() == io::ErrorKind::PermissionDenied) =>
            {
                PermissionDenied
            }
            Ok(metadata)
                if !metadata.is_dir()
                    || error.downcast_ref::<git2::Error>().is_some_and(|error| {
                        error.code() == git2::ErrorCode::NotFound
                            && error.class() == git2::ErrorClass::Repository
                    }) =>
            {
                RepositoryNotFound
            }
            _ => OpenFailed,
        };
        Self {
            reason,
            detail: format!("{error:#}"),
        }
    }
}

#[derive(Debug, Default)]
struct Inbox {
    roots: Vec<Root>,
    expected: Vec<PathBuf>,
    reconfigure: bool,
    changes: BTreeMap<PathBuf, Change>,
    unavailable: BTreeMap<PathBuf, Unavailable>,
    errors: BTreeMap<PathBuf, String>,
}
impl Inbox {
    fn event(&mut self, event: Event) {
        if matches!(event.kind, EventKind::Access(_)) {
            return;
        }
        if event.need_rescan()
            || self.expected.iter().any(|root| {
                event
                    .paths
                    .iter()
                    .any(|path| path == root || *path == root.join(".git"))
            })
        {
            self.reconfigure = true;
        }
        for root in &self.roots {
            for path in &event.paths {
                let Some(impact) = root.classify(path) else {
                    continue;
                };
                let now = Instant::now();
                let change = Change {
                    impact,
                    paths: if !path.starts_with(&root.git)
                        && !path.starts_with(&root.common)
                        && path.starts_with(&root.path)
                    {
                        BTreeSet::from([path.clone()])
                    } else {
                        BTreeSet::new()
                    },
                    first: now,
                    last: now,
                };
                self.changes
                    .entry(root.path.clone())
                    .and_modify(|current| current.merge(change.clone()))
                    .or_insert(change);
            }
        }
        if event.need_rescan() {
            for root in &self.roots {
                self.changes.insert(root.path.clone(), Change::all());
            }
        }
    }
}
/// Updates distinguish a repository that cannot be opened from a failed filesystem subscription.
pub struct Updates {
    pub changes: BTreeMap<PathBuf, Change>,
    pub unavailable: BTreeMap<PathBuf, Unavailable>,
    pub watch_errors: BTreeMap<PathBuf, String>,
}

#[derive(Debug)]
pub struct Monitor {
    tx: Option<mpsc::SyncSender<Vec<PathBuf>>>,
    inbox: Arc<Mutex<Inbox>>,
    handle: Option<thread::JoinHandle<()>>,
    configured: Vec<PathBuf>,
}
impl Monitor {
    pub fn start() -> Self {
        let inbox = Arc::new(Mutex::new(Inbox::default()));
        let target = inbox.clone();
        let (tx, rx) = mpsc::sync_channel::<Vec<PathBuf>>(1);
        let handle = thread::spawn(move || {
            let callback = target.clone();
            let watcher = notify::recommended_watcher(move |event: notify::Result<Event>| {
                let mut inbox = callback.lock().expect("observation inbox");
                match event {
                    Ok(event) => inbox.event(event),
                    Err(error) => {
                        let paths = inbox
                            .roots
                            .iter()
                            .map(|root| root.path.clone())
                            .collect::<Vec<_>>();
                        for path in paths {
                            inbox.errors.insert(path, error.to_string());
                        }
                    }
                }
            });
            let mut watched: BTreeMap<PathBuf, RecursiveMode> = BTreeMap::new();
            let mut watcher = watcher;
            let mut paths = Vec::new();
            let mut retry = Instant::now();
            loop {
                match rx.recv_timeout(Duration::from_millis(250)) {
                    Ok(request) => {
                        paths = request;
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        let mut inbox = target.lock().expect("observation inbox");
                        let rebuild = std::mem::take(&mut inbox.reconfigure);
                        let has_failures =
                            !inbox.errors.is_empty() || !inbox.unavailable.is_empty();
                        if !rebuild && (!has_failures || retry.elapsed() < Duration::from_secs(2)) {
                            continue;
                        }
                    }
                }
                retry = Instant::now();
                let mut roots = Vec::new();
                let mut unavailable = BTreeMap::new();
                let mut errors = BTreeMap::new();
                for path in &paths {
                    match Root::open(path.clone()) {
                        Ok(root) => roots.push(root),
                        Err(error) => {
                            unavailable.insert(path.clone(), Unavailable::from_error(path, error));
                        }
                    }
                }
                let candidates = roots
                    .iter()
                    .flat_map(|root| [root.path.clone(), root.git.clone(), root.common.clone()])
                    .collect::<BTreeSet<_>>();
                // 已有递归根覆盖的路径不重复注册；父目录只负责根被移动、删除或重建的事件。
                let mut desired = candidates
                    .iter()
                    .filter(|path| {
                        !candidates
                            .iter()
                            .any(|parent| parent != *path && path.starts_with(parent))
                    })
                    .map(|path| (path.clone(), RecursiveMode::Recursive))
                    .collect::<BTreeMap<_, _>>();
                for path in &paths {
                    if let Some(parent) = path.parent()
                        && parent.is_dir()
                        && !desired.iter().any(|(root, mode)| {
                            *mode == RecursiveMode::Recursive && parent.starts_with(root)
                        })
                    {
                        desired
                            .entry(parent.to_owned())
                            .or_insert(RecursiveMode::NonRecursive);
                    }
                }
                match &mut watcher {
                    Ok(watcher) => {
                        let rearm = {
                            let inbox = target.lock().expect("observation inbox");
                            roots.iter().any(|root| {
                                inbox.errors.contains_key(&root.path)
                                    || inbox.unavailable.contains_key(&root.path)
                            })
                        };
                        if desired != watched || rearm {
                            let mut batch = watcher.paths_mut();
                            for path in watched.keys() {
                                if (desired.get(path) != watched.get(path) || rearm)
                                    && let Err(error) = batch.remove(path)
                                    && !matches!(error.kind, notify::ErrorKind::WatchNotFound)
                                {
                                    for root in &paths {
                                        errors.insert(root.clone(), error.to_string());
                                    }
                                }
                            }
                            let mut installed = watched.clone();
                            installed.retain(|path, _| desired.contains_key(path));
                            for (path, mode) in &desired {
                                if watched.get(path) != Some(mode) || rearm {
                                    match batch.add(path, *mode) {
                                        Ok(()) => {
                                            installed.insert(path.clone(), *mode);
                                        }
                                        Err(error) => {
                                            for root in &paths {
                                                errors.insert(root.clone(), error.to_string());
                                            }
                                        }
                                    }
                                }
                            }
                            if let Err(error) = batch.commit() {
                                for path in &paths {
                                    errors.insert(path.clone(), error.to_string());
                                }
                            }
                            watched = installed;
                        }
                    }
                    Err(error) => {
                        for path in &paths {
                            errors.insert(path.clone(), error.to_string());
                        }
                    }
                }
                let mut inbox = target.lock().expect("observation inbox");
                inbox.changes.retain(|path, _| paths.contains(path));
                for root in &roots {
                    if !inbox.roots.contains(root)
                        || inbox.errors.contains_key(&root.path)
                        || inbox.unavailable.contains_key(&root.path)
                    {
                        inbox
                            .changes
                            .entry(root.path.clone())
                            .or_insert_with(Change::all);
                    }
                }
                inbox.expected = paths.clone();
                inbox.roots = roots;
                inbox.unavailable = unavailable;
                inbox.errors = errors;
            }
        });
        Self {
            tx: Some(tx),
            inbox,
            handle: Some(handle),
            configured: Vec::new(),
        }
    }
    pub fn ready(&self, path: &Path) -> bool {
        self.inbox
            .lock()
            .expect("observation inbox")
            .roots
            .iter()
            .any(|root| root.path == path)
    }
    pub fn configure(&mut self, mut paths: Vec<PathBuf>) {
        paths.sort();
        paths.dedup();
        if self.configured != paths
            && self
                .tx
                .as_ref()
                .is_some_and(|tx| tx.try_send(paths.clone()).is_ok())
        {
            self.configured = paths;
        }
    }
    pub fn take(&self) -> Updates {
        let mut inbox = self.inbox.lock().expect("observation inbox");
        let now = Instant::now();
        let paths = inbox
            .changes
            .iter()
            .filter(|(_, change)| {
                now.duration_since(change.last) >= Duration::from_millis(250)
                    || now.duration_since(change.first) >= Duration::from_secs(1)
            })
            .map(|(path, _)| path.clone())
            .collect::<Vec<_>>();
        let changes = paths
            .into_iter()
            .map(|path| {
                let change = inbox.changes.remove(&path).expect("pending observation");
                (path, change)
            })
            .collect();
        Updates {
            changes,
            unavailable: inbox.unavailable.clone(),
            watch_errors: inbox.errors.clone(),
        }
    }
}
impl Drop for Monitor {
    fn drop(&mut self) {
        self.tx.take();
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify::event::{AccessKind, ModifyKind};
    use test_support::{Fixture, git, write};
    #[test]
    fn native_events_keep_index_refs_and_submodule_state_but_ignore_object_and_access_noise() {
        let f = Fixture::create();
        let root = Root::open(f.repo.clone()).unwrap();
        for (relative, expected) in [
            ("index", Some(Impact::Worktree)),
            ("HEAD", Some(Impact::References)),
            ("refs/heads/main", Some(Impact::References)),
            ("objects/ab/hash", None),
            ("logs/HEAD", None),
            ("index.lock", None),
        ] {
            assert_eq!(
                root.classify(&root.git.join(relative)),
                expected,
                "{relative}"
            );
        }
        let mut inbox = Inbox {
            roots: vec![root],
            ..Default::default()
        };
        inbox.event(Event::new(EventKind::Access(AccessKind::Read)).add_path(f.repo.join("file")));
        assert!(inbox.changes.is_empty());
        for index in 0..500 {
            inbox.event(
                Event::new(EventKind::Modify(ModifyKind::Any))
                    .add_path(f.repo.join(format!("file-{index}"))),
            );
        }
        assert_eq!(inbox.changes.len(), 1);
        assert!(inbox.changes[&f.repo].paths.is_empty());
    }
    #[test]
    fn ignore_filter_preserves_tracked_files_and_native_index_events() {
        let f = Fixture::create();
        f.commit("tracked.txt", "tracked\n");
        write(f.repo.join(".gitignore"), "build/\ntracked.txt\n");
        std::fs::create_dir(f.repo.join("build")).unwrap();
        write(f.repo.join("build/output"), "build artifact\n");
        let change = |path: PathBuf| Change {
            impact: Impact::Worktree,
            paths: BTreeSet::from([path]),
            first: Instant::now(),
            last: Instant::now(),
        };
        assert!(
            !change(f.repo.join("build/output"))
                .relevant(&f.repo)
                .unwrap()
        );
        assert!(
            change(f.repo.join("tracked.txt"))
                .relevant(&f.repo)
                .unwrap()
        );
        git(&f.repo, &["add", "tracked.txt"]);
        let mut inbox = Inbox {
            roots: vec![Root::open(f.repo.clone()).unwrap()],
            ..Default::default()
        };
        inbox.event(
            Event::new(EventKind::Modify(ModifyKind::Any)).add_path(f.repo.join(".git/index")),
        );
        assert!(inbox.changes[&f.repo].relevant(&f.repo).unwrap());
    }
    #[test]
    fn shared_metadata_invalidates_each_registered_worktree() {
        let f = Fixture::create();
        let linked = f.repo.parent().unwrap().join("linked");
        git(
            &f.repo,
            &["worktree", "add", "-b", "linked", linked.to_str().unwrap()],
        );
        let mut inbox = Inbox {
            roots: vec![
                Root::open(f.repo.clone()).unwrap(),
                Root::open(linked.clone()).unwrap(),
            ],
            ..Default::default()
        };
        inbox.event(
            Event::new(EventKind::Modify(ModifyKind::Any))
                .add_path(f.repo.join(".git/refs/heads/main")),
        );
        assert_eq!(inbox.changes.len(), 2);
        assert!(
            inbox
                .changes
                .values()
                .all(|change| change.impact == Impact::References)
        );
        inbox.changes.clear();
        let git = inbox.roots[1].git.clone();
        inbox.event(Event::new(EventKind::Modify(ModifyKind::Any)).add_path(git.join("index")));
        assert_eq!(inbox.changes.len(), 1);
        assert!(inbox.changes.contains_key(&linked));
    }
}
