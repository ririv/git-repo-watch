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

type WatchPaths = BTreeMap<PathBuf, RecursiveMode>;

#[derive(Debug, Default)]
enum Subscription {
    #[default]
    Idle,
    Registering {
        revision: u64,
        error: Option<String>,
    },
    Listening,
    Failed(String),
}
impl Subscription {
    fn error(&self) -> Option<&str> {
        match self {
            Self::Registering { error, .. } => error.as_deref(),
            Self::Failed(error) => Some(error),
            _ => None,
        }
    }
}

struct Registration {
    revision: u64,
    paths: WatchPaths,
    rearm: bool,
}

#[derive(Debug, Default)]
struct Inbox {
    roots: Vec<Root>,
    expected: Vec<PathBuf>,
    reconfigure: bool,
    changes: BTreeMap<PathBuf, Change>,
    unavailable: BTreeMap<PathBuf, Unavailable>,
    event_errors: BTreeMap<PathBuf, String>,
    subscription: Subscription,
}
impl Inbox {
    fn retry_subscription(&self) -> bool {
        !matches!(self.subscription, Subscription::Registering { .. })
            && (self.subscription.error().is_some() || !self.event_errors.is_empty())
    }

    fn publish_roots(
        &mut self,
        paths: &[PathBuf],
        roots: Vec<Root>,
        unavailable: BTreeMap<PathBuf, Unavailable>,
    ) {
        self.changes.retain(|path, _| paths.contains(path));
        self.event_errors.retain(|path, _| paths.contains(path));
        for root in &roots {
            if !self.roots.contains(root) {
                self.changes.insert(root.path.clone(), Change::all());
            }
        }
        self.expected = paths.to_vec();
        self.roots = roots;
        self.unavailable = unavailable;
    }

    fn registered(&mut self, revision: u64, result: notify::Result<()>) {
        if !matches!(self.subscription, Subscription::Registering { revision: current, .. } if current == revision)
        {
            return;
        }
        self.subscription = match result {
            Ok(()) => {
                // 注册空档内可能没有事件，重新检查路径并读取仓库状态。
                self.reconfigure = true;
                for root in &self.roots {
                    self.changes
                        .entry(root.path.clone())
                        .and_modify(|change| change.merge(Change::all()))
                        .or_insert_with(Change::all);
                }
                Subscription::Listening
            }
            Err(error) => Subscription::Failed(error.to_string()),
        };
    }

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
        Self::spawn(native_registration)
    }

    fn spawn<F>(registration: impl FnOnce(Arc<Mutex<Inbox>>) -> F + Send + 'static) -> Self
    where
        F: FnMut(&WatchPaths, bool) -> notify::Result<()>,
    {
        let inbox = Arc::new(Mutex::new(Inbox::default()));
        let target = inbox.clone();
        let (tx, rx) = mpsc::sync_channel(1);
        let handle = thread::spawn(move || {
            let (requests, incoming) = mpsc::sync_channel::<Registration>(1);
            let events = target.clone();
            let watcher = thread::spawn(move || {
                let mut register = registration(events.clone());
                while let Ok(mut request) = incoming.recv() {
                    for mut next in incoming.try_iter() {
                        next.rearm |= request.rearm;
                        request = next;
                    }
                    let result = register(&request.paths, request.rearm);
                    events
                        .lock()
                        .expect("observation inbox")
                        .registered(request.revision, result);
                }
            });
            scan(&target, rx, &requests);
            drop(requests);
            let _ = watcher.join();
        });
        Self {
            tx: Some(tx),
            inbox,
            handle: Some(handle),
            configured: Vec::new(),
        }
    }
    /// Whether the repository is known locally and can be read.
    pub fn ready(&self, path: &Path) -> bool {
        self.inbox
            .lock()
            .expect("observation inbox")
            .roots
            .iter()
            .any(|root| root.path == path)
    }
    /// Whether the subscription is active and its queued changes have been taken.
    pub fn watch_settled(&self, path: &Path) -> bool {
        let inbox = self.inbox.lock().expect("observation inbox");
        matches!(inbox.subscription, Subscription::Listening)
            && inbox.roots.iter().any(|root| root.path == path)
            && !inbox.event_errors.contains_key(path)
            && !inbox.changes.contains_key(path)
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
        let mut watch_errors = inbox
            .subscription
            .error()
            .map(|error| {
                inbox
                    .expected
                    .iter()
                    .map(|path| (path.clone(), error.to_owned()))
                    .collect::<BTreeMap<_, _>>()
            })
            .unwrap_or_default();
        watch_errors.extend(inbox.event_errors.clone());
        Updates {
            changes,
            unavailable: inbox.unavailable.clone(),
            watch_errors,
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

fn scan(
    target: &Mutex<Inbox>,
    rx: mpsc::Receiver<Vec<PathBuf>>,
    requests: &mpsc::SyncSender<Registration>,
) {
    let mut paths = Vec::new();
    let mut planned = WatchPaths::new();
    let mut revision = 0;
    let mut pending: Option<Registration> = None;
    let mut last_scan = Instant::now();
    loop {
        let scan_roots = match rx.recv_timeout(Duration::from_millis(250)) {
            Ok(request) => {
                paths = request;
                true
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                let mut inbox = target.lock().expect("observation inbox");
                let rebuild = std::mem::take(&mut inbox.reconfigure);
                rebuild
                    || ((!inbox.unavailable.is_empty() || inbox.retry_subscription())
                        && last_scan.elapsed() >= Duration::from_secs(2))
            }
        };
        if scan_roots {
            last_scan = Instant::now();
            let mut roots = Vec::new();
            let mut unavailable = BTreeMap::new();
            for path in &paths {
                match Root::open(path.clone()) {
                    Ok(root) => roots.push(root),
                    Err(error) => {
                        unavailable.insert(path.clone(), Unavailable::from_error(path, error));
                    }
                }
            }
            let desired = watch_paths(&paths, &roots);
            let mut inbox = target.lock().expect("observation inbox");
            let rearm = roots
                .iter()
                .any(|root| inbox.unavailable.contains_key(&root.path))
                || inbox.retry_subscription();
            inbox.publish_roots(&paths, roots, unavailable);
            if desired != planned || rearm {
                revision += 1;
                let error = inbox.subscription.error().map(str::to_owned);
                inbox.subscription = Subscription::Registering { revision, error };
                inbox.event_errors.clear();
                planned = desired;
                pending = Some(Registration {
                    revision,
                    paths: planned.clone(),
                    rearm: rearm || pending.as_ref().is_some_and(|queued| queued.rearm),
                });
            }
        }
        if let Some(request) = pending.take() {
            match requests.try_send(request) {
                Ok(()) => {}
                Err(mpsc::TrySendError::Full(request)) => pending = Some(request),
                Err(mpsc::TrySendError::Disconnected(request)) => {
                    target.lock().expect("observation inbox").registered(
                        request.revision,
                        Err(notify::Error::generic(i18n::tr!(
                            BackgroundWorkerTerminatedUnexpectedly
                        ))),
                    )
                }
            }
        }
    }
}

fn watch_paths(paths: &[PathBuf], roots: &[Root]) -> WatchPaths {
    let candidates = roots
        .iter()
        .flat_map(|root| [root.path.clone(), root.git.clone(), root.common.clone()])
        .collect::<BTreeSet<_>>();
    // 已有递归根覆盖的路径不重复注册；父目录监听根被移动、删除或重建。
    let mut desired = candidates
        .iter()
        .filter(|path| {
            !candidates
                .iter()
                .any(|parent| parent != *path && path.starts_with(parent))
        })
        .map(|path| (path.clone(), RecursiveMode::Recursive))
        .collect::<WatchPaths>();
    for path in paths {
        if let Some(parent) = path.parent()
            && parent.is_dir()
            && !desired
                .iter()
                .any(|(root, mode)| *mode == RecursiveMode::Recursive && parent.starts_with(root))
        {
            desired
                .entry(parent.to_owned())
                .or_insert(RecursiveMode::NonRecursive);
        }
    }
    desired
}

fn native_registration(
    inbox: Arc<Mutex<Inbox>>,
) -> impl FnMut(&WatchPaths, bool) -> notify::Result<()> {
    let mut watched = WatchPaths::new();
    let mut watcher = None;
    let mut registration_failed = false;
    move |desired, rearm| {
        if watcher.is_none() {
            let target = inbox.clone();
            watcher = Some(notify::recommended_watcher(
                move |event: notify::Result<Event>| {
                    let mut inbox = target.lock().expect("observation inbox");
                    match event {
                        Ok(event) => inbox.event(event),
                        Err(error) => {
                            let error = error.to_string();
                            let paths = inbox
                                .roots
                                .iter()
                                .map(|root| root.path.clone())
                                .collect::<Vec<_>>();
                            inbox
                                .event_errors
                                .extend(paths.into_iter().map(|path| (path, error.clone())));
                        }
                    }
                },
            )?);
        }
        let result = register_paths(
            watcher.as_mut().expect("created native watcher"),
            &mut watched,
            desired,
            rearm || registration_failed,
        );
        registration_failed = result.is_err();
        result
    }
}

fn register_paths(
    watcher: &mut impl Watcher,
    watched: &mut WatchPaths,
    desired: &WatchPaths,
    rearm: bool,
) -> notify::Result<()> {
    if watched == desired && !rearm {
        return Ok(());
    }
    let mut batch = watcher.paths_mut();
    let mut error = None;
    for path in watched.keys() {
        if (desired.get(path) != watched.get(path) || rearm)
            && let Err(reason) = batch.remove(path)
            && !matches!(reason.kind, notify::ErrorKind::WatchNotFound)
        {
            error = Some(reason);
        }
    }
    let mut installed = watched.clone();
    installed.retain(|path, _| desired.contains_key(path));
    for (path, mode) in desired {
        if watched.get(path) != Some(mode) || rearm {
            match batch.add(path, *mode) {
                Ok(()) => {
                    installed.insert(path.clone(), *mode);
                }
                Err(reason) => error = Some(reason),
            }
        }
    }
    if let Err(reason) = batch.commit() {
        error = Some(reason);
    }
    *watched = installed;
    error.map_or(Ok(()), Err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify::event::{AccessKind, ModifyKind};
    use test_support::{Fixture, git, write};
    #[test]
    fn discovers_repositories_while_registration_waits_then_reconciles() {
        let first = Fixture::create();
        let second = Fixture::create();
        let (entered, registering) = mpsc::channel();
        let (release, resume) = mpsc::channel();
        let mut monitor = Monitor::spawn(move |_| {
            move |_, _| {
                entered.send(()).unwrap();
                let _ = resume.recv();
                Ok(())
            }
        });
        // 断言失败时先释放订阅线程，再回收观察器。
        let release = release;

        monitor.configure(vec![first.repo.clone()]);
        registering.recv_timeout(Duration::from_secs(8)).unwrap();
        let changes = collect_changes(&monitor, 1);
        assert!(changes.contains(&first.repo));

        monitor.configure(vec![first.repo.clone(), second.repo.clone()]);
        let changes = collect_changes(&monitor, 1);
        assert_eq!(changes, BTreeSet::from([second.repo.clone()]));

        release.send(()).unwrap();
        registering.recv_timeout(Duration::from_secs(8)).unwrap();
        assert!(monitor.inbox.lock().unwrap().changes.is_empty());

        release.send(()).unwrap();
        assert_eq!(
            collect_changes(&monitor, 2),
            BTreeSet::from([first.repo.clone(), second.repo.clone()])
        );
        assert!(monitor.take().watch_errors.is_empty());
    }

    #[test]
    fn registration_rechecks_a_repository_moved_while_starting() {
        let f = Fixture::create();
        let (release, resume) = mpsc::channel();
        let mut monitor = Monitor::spawn(move |_| {
            let mut resume = Some(resume);
            move |_, _| {
                if let Some(resume) = resume.take() {
                    let _ = resume.recv();
                }
                Ok(())
            }
        });
        let release = release;
        monitor.configure(vec![f.repo.clone()]);
        assert_eq!(
            collect_changes(&monitor, 1),
            BTreeSet::from([f.repo.clone()])
        );

        std::fs::rename(&f.repo, f.repo.with_file_name("moved")).unwrap();
        release.send(()).unwrap();

        let start = Instant::now();
        loop {
            let updates = monitor.take();
            if let Some(unavailable) = updates.unavailable.get(&f.repo) {
                assert_eq!(unavailable.reason, UnavailableReason::MissingPath);
                break;
            }
            assert!(start.elapsed() < Duration::from_secs(8));
            thread::sleep(Duration::from_millis(5));
        }
    }

    fn collect_changes(monitor: &Monitor, count: usize) -> BTreeSet<PathBuf> {
        let mut paths = BTreeSet::new();
        let start = Instant::now();
        while paths.len() < count {
            let updates = monitor.take();
            assert!(updates.unavailable.is_empty());
            paths.extend(updates.changes.into_keys());
            assert!(
                start.elapsed() < Duration::from_secs(8),
                "expected {count} invalidations, got {paths:?}"
            );
            if paths.len() < count {
                thread::sleep(Duration::from_millis(5));
            }
        }
        paths
    }

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
