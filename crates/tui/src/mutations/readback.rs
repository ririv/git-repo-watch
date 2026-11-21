//! Read only the affected foreground list after a repository mutation.
use crate::{Mode, Screen, branches, stashes};
use anyhow::Result;
use engine::{cancel::Cancel, local, service::Service};
use std::path::Path;
#[derive(Clone, Copy)]
pub(crate) enum Requested {
    None,
    Files,
    Branches,
    Worktrees,
    Stashes,
}
impl Requested {
    pub fn for_mode(mode: &Mode) -> Self {
        match mode {
            Mode::Files(_) | Mode::Changes(_) => Self::Files,
            Mode::Branches(_) => Self::Branches,
            Mode::Worktrees(_) => Self::Worktrees,
            Mode::Stashes(_) => Self::Stashes,
            _ => Self::None,
        }
    }
    pub fn read(self, service: &Service, path: &Path) -> Result<Readback> {
        // 写操作结束后独立读回实际状态，取消信号不能跳过已经产生的变更。
        let cancel = Cancel::default();
        self.read_with_cancel(service, path, &cancel)
    }
    pub(crate) fn read_with_cancel(
        self,
        service: &Service,
        path: &Path,
        cancel: &Cancel,
    ) -> Result<Readback> {
        Ok(match self {
            Self::None => Readback::Unchanged,
            Self::Files => Readback::Files(service.files(path, cancel)?),
            Self::Branches => Readback::Branches(service.branches(path, cancel)?),
            Self::Worktrees => Readback::Worktrees(service.worktrees(path, cancel)?),
            Self::Stashes => Readback::Stashes(service.stashes(path, cancel)?),
        })
    }
}
#[derive(Debug, Clone)]
pub(crate) enum Readback {
    Unchanged,
    Files(local::files::Files),
    Branches(local::branches::catalog::Snapshot),
    Worktrees(local::worktrees::Snapshot),
    Stashes(local::stash::Snapshot),
}
impl Screen {
    pub(crate) fn merge_mutation_view(&mut self, readback: Readback) {
        merge_mode(&mut self.mode, readback.clone());
        for visit in &mut self.history {
            visit.refresh(readback.clone());
        }
    }
}
pub(crate) fn merge_mode(mode: &mut Mode, readback: Readback) {
    match (readback, mode) {
        (Readback::Worktrees(snapshot), Mode::Worktrees(session))
            if session.snapshot.path == snapshot.path =>
        {
            session.refresh(snapshot)
        }
        (Readback::Files(snapshot), Mode::Browser(session)) if session.root == snapshot.path => {
            session.refresh()
        }
        (Readback::Files(snapshot), Mode::Files(session))
            if session.snapshot.path == snapshot.path =>
        {
            *session = session.refreshed(snapshot)
        }
        (Readback::Files(snapshot), Mode::Changes(session))
            if session.files.snapshot.path == snapshot.path =>
        {
            session.refreshed(snapshot)
        }
        (Readback::Stashes(snapshot), Mode::Stashes(view))
            if view.session().snapshot.path == snapshot.path =>
        {
            let selected = view.selected().map(|entry| entry.oid);
            let next = view.session().refreshed(snapshot);
            if selected
                .is_some_and(|oid| next.snapshot.entries.iter().any(|entry| entry.oid == oid))
            {
                match view.as_mut() {
                    stashes::View::List(session)
                    | stashes::View::Detail { session, .. }
                    | stashes::View::Diff { session, .. } => *session = next,
                }
            } else {
                **view = stashes::View::List(next);
            }
        }
        (Readback::Branches(snapshot), Mode::Branches(view))
            if view.session().snapshot.path == snapshot.path =>
        {
            let next = view.session().refreshed(snapshot);
            **view = match view.as_ref() {
                branches::View::Detail { entry, scroll, .. } => match next
                    .snapshot
                    .entries
                    .iter()
                    .find(|e| e.reference == entry.reference)
                    .cloned()
                {
                    Some(entry) => branches::View::Detail {
                        session: next,
                        entry: std::sync::Arc::new(entry),
                        scroll: *scroll,
                    },
                    None => branches::View::List(next),
                },
                branches::View::List(_) => branches::View::List(next),
            };
        }
        _ => {}
    }
    // 输入栏独立保存；此处只合并快照，不同步或替换新命令。
}
