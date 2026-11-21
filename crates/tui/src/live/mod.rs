//! Passive local refreshes preserve interaction state and remote check history.
pub(crate) mod merge;
use crate::{Mode, Screen, history, workspace};
use engine::{
    cancel::Cancel,
    domain::RepositoryResult,
    observation::{
        self,
        watch::{Change, Monitor, UnavailableReason},
    },
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::Arc,
    thread,
};

#[derive(Default)]
pub(crate) enum State {
    #[default]
    Disabled,
    Active(Box<Controller>),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WindowFocus {
    Focused,
    Unfocused,
}
pub(crate) struct Controller {
    monitor: Monitor,
    pending: BTreeMap<PathBuf, Change>,
    readers: BTreeMap<PathBuf, Reader>,
    pub results: BTreeMap<PathBuf, RepositoryResult>,
    unavailable: BTreeMap<PathBuf, UnavailableRepository>,
    pub errors: BTreeMap<PathBuf, String>,
    watch_errors: BTreeMap<PathBuf, String>,
    detailed: BTreeSet<PathBuf>,
}
struct UnavailableRepository {
    reason: UnavailableReason,
    result: RepositoryResult,
}
struct Observed {
    snapshot: observation::Snapshot,
    graph: Result<Option<history::Session>, String>,
}
struct Reader {
    order: Option<(u64, chrono::DateTime<chrono::FixedOffset>)>,
    cancel: Cancel,
    handle: thread::JoinHandle<anyhow::Result<Option<Observed>>>,
}
impl State {
    #[cfg(test)]
    pub fn diagnostics(&self) -> String {
        match self {
            Self::Disabled => "disabled".into(),
            Self::Active(controller) => format!(
                "pending={:?} readers={:?} results={:?}",
                controller.pending.keys(),
                controller
                    .readers
                    .iter()
                    .map(|(path, reader)| (path, reader.handle.is_finished()))
                    .collect::<Vec<_>>(),
                controller.results.keys()
            ),
        }
    }
    pub fn start() -> Self {
        Self::Active(Box::new(Controller {
            monitor: Monitor::start(),
            pending: BTreeMap::new(),
            readers: BTreeMap::new(),
            results: BTreeMap::new(),
            unavailable: BTreeMap::new(),
            errors: BTreeMap::new(),
            watch_errors: BTreeMap::new(),
            detailed: BTreeSet::new(),
        }))
    }
    fn pause(&mut self) {
        if let Self::Active(controller) = self {
            for (path, reader) in &controller.readers {
                reader.cancel.cancel();
                controller
                    .pending
                    .entry(path.clone())
                    .and_modify(|change| change.merge(Change::all()))
                    .or_insert_with(Change::all);
            }
        }
    }
    pub fn is_reading(&self, path: &Path) -> bool {
        matches!(self, Self::Active(controller) if controller.readers.contains_key(path))
    }
    #[cfg(test)]
    pub fn reading(&self) -> usize {
        match self {
            Self::Disabled => 0,
            Self::Active(controller) => controller.readers.len(),
        }
    }
    #[cfg(test)]
    pub fn pending(&self) -> usize {
        match self {
            Self::Disabled => 0,
            Self::Active(controller) => controller.pending.len(),
        }
    }
    pub fn result(&self, path: &Path) -> Option<&RepositoryResult> {
        match self {
            Self::Disabled => None,
            Self::Active(controller) => controller
                .unavailable
                .get(path)
                .map(|unavailable| &unavailable.result)
                .or_else(|| controller.results.get(path)),
        }
    }
    pub fn unavailable(&self, path: &Path) -> bool {
        self.unavailable_reason(path).is_some()
    }
    pub fn unavailable_reason(&self, path: &Path) -> Option<UnavailableReason> {
        match self {
            Self::Disabled => None,
            Self::Active(controller) => controller.unavailable.get(path).map(|entry| entry.reason),
        }
    }
    pub fn invalidate(&mut self, path: &Path) {
        if let Self::Active(controller) = self {
            controller.results.remove(path);
        }
    }
    pub fn error(&self) -> Option<String> {
        match self {
            Self::Disabled => None,
            Self::Active(controller) => controller
                .watch_errors
                .first_key_value()
                .or_else(|| controller.errors.first_key_value())
                .map(|(path, error)| i18n::tf!(LocalObservationFailed, path.display(), error))
                .or_else(|| {
                    let (
                        path,
                        UnavailableRepository {
                            result: RepositoryResult::Failed { error, .. },
                            ..
                        },
                    ) = controller.unavailable.first_key_value()?
                    else {
                        return None;
                    };
                    Some(i18n::tf!(LocalObservationFailed, path.display(), error))
                }),
        }
    }
    pub fn shutdown(&mut self) {
        *self = Self::Disabled;
    }
}
impl Drop for Controller {
    fn drop(&mut self) {
        for reader in self.readers.values() {
            reader.cancel.cancel();
        }
        for (_, reader) in std::mem::take(&mut self.readers) {
            let _ = reader.handle.join();
        }
    }
}
impl Screen {
    pub(crate) fn window_focus_changed(&mut self, focus: WindowFocus) {
        if self.window_focus != focus {
            self.window_focus = focus;
            if focus == WindowFocus::Unfocused {
                self.live.pause();
            }
        }
    }
    pub(crate) fn poll_live(&mut self) -> bool {
        let selected = self.selected_path();
        let scope = self.scope_path();
        let mut state = std::mem::take(&mut self.live);
        let State::Active(controller) = &mut state else {
            return false;
        };
        let mut paths = self.config.repositories.clone();
        paths.extend(self.service.session_paths().map(Path::to_owned));
        paths.sort();
        paths.dedup();
        controller.monitor.configure(paths.clone());
        controller.pending.retain(|path, _| paths.contains(path));
        controller.results.retain(|path, _| paths.contains(path));
        controller
            .unavailable
            .retain(|path, _| paths.contains(path));
        controller.errors.retain(|path, _| paths.contains(path));
        let updates = controller.monitor.take();
        let mut changed = controller.watch_errors != updates.watch_errors;
        controller.watch_errors = updates.watch_errors;
        for (path, problem) in &updates.unavailable {
            if !paths.contains(path)
                || matches!(controller.unavailable.get(path),
                    Some(UnavailableRepository { reason, result: RepositoryResult::Failed { error, .. } })
                    if *reason == problem.reason && *error == problem.detail)
            {
                continue;
            }
            controller.results.remove(path);
            self.remote_info.invalidate(path);
            controller.unavailable.insert(
                path.clone(),
                UnavailableRepository {
                    reason: problem.reason,
                    result: RepositoryResult::Failed {
                        path: path.clone(),
                        error: problem.detail.clone(),
                        pushes: self
                            .stored_result_for(path)
                            .map_or_else(Vec::new, |result| result.pushes().to_vec()),
                    },
                },
            );
            changed = true;
        }
        for (path, change) in updates.changes {
            controller
                .pending
                .entry(path)
                .and_modify(|current| current.merge(change.clone()))
                .or_insert(change);
        }
        let detailed = scope
            .into_iter()
            .chain(self.workspace.path().map(Path::to_owned))
            .collect::<BTreeSet<_>>();
        for path in detailed.difference(&controller.detailed) {
            if controller.monitor.ready(path) {
                controller.pending.insert(path.clone(), Change::all());
            }
        }
        controller.detailed = detailed.clone();
        let done = controller
            .readers
            .iter()
            .filter(|(_, reader)| reader.handle.is_finished())
            .map(|(path, _)| path.clone())
            .collect::<Vec<_>>();
        for path in done {
            let reader = controller
                .readers
                .remove(&path)
                .expect("finished local reader");
            let result = reader
                .handle
                .join()
                .unwrap_or_else(|_| Err(anyhow::anyhow!(i18n::tr!(BackgroundWorkerPanicked))));
            if !paths.contains(&path) {
                continue;
            }
            if reader.cancel.is_cancelled()
                || updates.unavailable.contains_key(&path)
                || self.window_focus == WindowFocus::Unfocused
                || self.local_refresh_blocked(&path)
                || reader.order != self.observation_order(&path)
            {
                controller.pending.insert(path, Change::all());
                continue;
            }
            match result {
                Ok(Some(observed)) => {
                    if let Ok(Some(graph)) = &observed.graph
                        && graph.records.len() < self.live_history_window(&path).count
                        && graph.cursor().is_some()
                    {
                        controller.pending.insert(path, Change::all());
                        continue;
                    }
                    self.remote_info.invalidate(&path);
                    let snapshot = observed.snapshot;
                    let previous = self.stored_result_for(&path);
                    // 重新读到状态后，才移除路径不可访问的标记。
                    controller.unavailable.remove(&path);
                    controller
                        .results
                        .insert(path.clone(), merge::result(snapshot.result, previous));
                    controller.errors.remove(&path);
                    self.merge_local_detail(
                        &path,
                        snapshot.detail,
                        observed.graph,
                        &mut controller.errors,
                    );
                    changed = true;
                }
                Ok(None) => {}
                Err(error) => {
                    controller.errors.insert(path, format!("{error:#}"));
                    changed = true;
                }
            }
        }
        let available = self
            .config
            .max_concurrency
            .saturating_sub(controller.readers.len());
        let ready = controller
            .pending
            .keys()
            .filter(|path| {
                self.window_focus == WindowFocus::Focused
                    && !controller.readers.contains_key(*path)
                    && !updates.unavailable.contains_key(*path)
                    && !self.local_refresh_blocked(path)
            })
            .take(available)
            .cloned()
            .collect::<Vec<_>>();
        for path in ready {
            let change = controller
                .pending
                .remove(&path)
                .expect("queued local refresh");
            let scope = if detailed.contains(&path) {
                let history = self.live_history_window(&path);
                observation::Scope::Visible {
                    history,
                    include_ignored: matches!(&self.mode, Mode::Browser(session) if session.root == path),
                }
            } else {
                observation::Scope::Summary
            };
            let order = self.observation_order(&path);
            let cancel = Cancel::default();
            let task_cancel = cancel.clone();
            let service = Arc::clone(&self.service);
            let task_path = path.clone();
            let language = i18n::language();
            let handle = thread::spawn(move || {
                let _language = i18n::using(language);
                let Some(mut snapshot) =
                    service.observe(&task_path, &change, scope, &task_cancel)?
                else {
                    return Ok(None);
                };
                let pages = snapshot
                    .detail
                    .as_mut()
                    .map(|detail| std::mem::replace(&mut detail.history, Ok(Vec::new())))
                    .unwrap_or(Ok(Vec::new()));
                let graph = pages.and_then(|pages| {
                    let mut pages = pages.into_iter();
                    let Some(page) = pages.next() else {
                        return Ok(None);
                    };
                    let mut graph = history::Session::new(page, &task_cancel)
                        .map_err(|error| format!("{error:#}"))?;
                    for page in pages {
                        graph
                            .append(page, &task_cancel)
                            .map_err(|error| format!("{error:#}"))?;
                    }
                    Ok(Some(graph))
                });
                Ok(Some(Observed { snapshot, graph }))
            });
            controller.readers.insert(
                path,
                Reader {
                    order,
                    cancel,
                    handle,
                },
            );
        }
        self.live = state;
        if changed {
            self.focus(selected);
        }
        changed
    }
    fn local_refresh_blocked(&self, path: &Path) -> bool {
        self.repository_tasks.contains_key(path)
            || self
                .check_task
                .as_ref()
                .is_some_and(|task| task.checking(path))
            || matches!(&self.mode, Mode::Busy(_))
            || (self.workspace.path() == Some(path) && self.workspace.loading())
    }
    fn live_history_window(&self, path: &Path) -> observation::HistoryWindow {
        let foreground = match &self.mode {
            Mode::History(view) if view.session().references.path == path => Some(view.session()),
            _ => None,
        };
        let cached = self
            .workspace
            .session()
            .filter(|_| self.workspace.path() == Some(path))
            .and_then(|session| match &session.history {
                workspace::Panel::Ready(graph) => Some(graph),
                _ => None,
            });
        let graph = foreground.or(cached);
        observation::HistoryWindow {
            count: graph.map_or(engine::local::history::PAGE_SIZE, |graph| {
                graph.records.len()
            }),
            anchor: graph.and_then(|graph| {
                graph
                    .records
                    .iter()
                    .enumerate()
                    .rfind(|(index, record)| {
                        graph.state.selected() == Some(*index)
                            || graph.selection.contains(&record.entry.oid)
                    })
                    .map(|(_, record)| record.entry.oid)
            }),
        }
    }
    fn merge_local_detail(
        &mut self,
        path: &Path,
        detail: Option<observation::Detail>,
        graph: Result<Option<history::Session>, String>,
        errors: &mut BTreeMap<PathBuf, String>,
    ) {
        let Some(detail) = detail else {
            return;
        };
        let mut failures = Vec::new();
        for readback in [
            detail
                .files
                .map(crate::mutations::readback::Readback::Files),
            detail
                .branches
                .map(crate::mutations::readback::Readback::Branches),
            detail
                .stashes
                .map(crate::mutations::readback::Readback::Stashes),
        ] {
            match readback {
                Ok(readback) => {
                    merge::mode(&mut self.mode, &readback);
                    for visit in &mut self.history {
                        visit.observe(&readback);
                    }
                    self.workspace.observe(&readback);
                }
                Err(error) => failures.push(error),
            }
        }
        match detail.overview {
            Ok(snapshot) => {
                self.replay
                    .observe(path.to_owned(), snapshot.replay.clone());
                self.overview.observe(snapshot);
            }
            Err(error) => failures.push(error),
        }
        match graph {
            Ok(Some(graph)) => {
                merge::history_mode(&mut self.mode, &graph);
                for visit in &mut self.history {
                    visit.observe_history(&graph);
                }
                self.workspace.observe_history(&graph);
            }
            Ok(None) => {}
            Err(error) => failures.push(error),
        }
        if !failures.is_empty() {
            errors.insert(path.to_owned(), failures.join(" · "));
        }
    }
}
