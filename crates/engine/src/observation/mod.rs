//! Read-only local observations, separate from remote checks and operation journals.
pub mod watch;
use crate::{cancel::Cancel, domain::RepositoryResult, local};
use std::path::Path;

#[derive(Debug, Clone, Copy)]
pub enum Scope {
    Summary,
    Visible {
        history: HistoryWindow,
        include_ignored: bool,
    },
}
#[derive(Debug, Clone, Copy)]
pub struct HistoryWindow {
    pub count: usize,
    pub anchor: Option<local::history::Oid>,
}
#[derive(Debug)]
pub struct Detail {
    pub files: Result<local::files::Files, String>,
    pub branches: Result<local::branches::catalog::Snapshot, String>,
    pub stashes: Result<local::stash::Snapshot, String>,
    pub overview: Result<local::overview::Snapshot, String>,
    pub history: Result<Vec<local::history::Page>, String>,
}
#[derive(Debug)]
pub struct Snapshot {
    pub result: RepositoryResult,
    pub detail: Option<Detail>,
}
fn section<T>(result: anyhow::Result<T>) -> Result<T, String> {
    result.map_err(|error| format!("{error:#}"))
}
impl crate::service::Service {
    pub fn observe(
        &self,
        path: &Path,
        change: &watch::Change,
        scope: Scope,
        cancel: &Cancel,
    ) -> anyhow::Result<Option<Snapshot>> {
        let _lock = self.repository_lock(path, cancel)?;
        if !matches!(
            scope,
            Scope::Visible {
                include_ignored: true,
                ..
            }
        ) && !change.relevant(path)?
        {
            return Ok(None);
        }
        let (result, _) = crate::check::local_repository(path, cancel);
        let detail = match scope {
            Scope::Summary => None,
            Scope::Visible { history, .. } => {
                let files = section(self.files(path, cancel));
                cancel.check()?;
                let branches = section(self.branches(path, cancel));
                cancel.check()?;
                let stashes = section(self.stashes(path, cancel));
                cancel.check()?;
                let overview = section(self.overview(path, cancel));
                cancel.check()?;
                let history = section(read_history(self, path, change, history, cancel));
                Some(Detail {
                    files,
                    branches,
                    stashes,
                    overview,
                    history,
                })
            }
        };
        cancel.check()?;
        Ok(Some(Snapshot { result, detail }))
    }
}

fn read_history(
    service: &crate::service::Service,
    path: &Path,
    change: &watch::Change,
    window: HistoryWindow,
    cancel: &Cancel,
) -> anyhow::Result<Vec<local::history::Page>> {
    let mut pages = Vec::new();
    if window.count == 0 || change.impact != watch::Impact::References {
        return Ok(pages);
    }
    let mut page = service.history(path, local::history::Request::Start, cancel)?;
    let mut loaded = 0;
    let mut found = window.anchor.is_none();
    loop {
        let continuation = page.continuation.clone();
        found |= page
            .entries
            .iter()
            .any(|entry| Some(entry.oid) == window.anchor);
        loaded += page.entries.len();
        pages.push(page);
        if loaded >= window.count && (found || loaded >= window.count.saturating_add(1000)) {
            break;
        }
        let local::history::Continuation::More(cursor) = continuation else {
            break;
        };
        page = service.history(path, local::history::Request::Continue(cursor), cancel)?;
    }
    Ok(pages)
}
