//! Session-local filters and persisted repository ordering share command transitions.
mod status;
use crate::{Mode, Page, Row, Screen};
use chrono::{DateTime, FixedOffset};
use engine::{domain::RepositoryResult, service::CachedReport};
pub(crate) use status::{Counts, Status};
use std::{cmp::Reverse, path::Path};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Filter {
    All,
    Problems,
    Clean,
    Failed,
    Unchecked,
    WithoutUpstream,
    Pushes,
}
impl Filter {
    pub(crate) fn matches(self, result: Option<&RepositoryResult>, execution: Status) -> bool {
        match self {
            Self::All => true,
            Self::Unchecked => execution == Status::Unchecked,
            Self::Clean => execution == Status::Normal,
            Self::Failed => execution == Status::Failed,
            Self::Problems => matches!(execution, Status::NeedsAttention | Status::Failed),
            Self::WithoutUpstream => {
                result.is_some_and(|r| engine::domain::Filter::WithoutUpstream.matches(r))
            }
            Self::Pushes => result.is_some_and(|r| engine::domain::Filter::Pushes.matches(r)),
        }
    }
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::All => i18n::tr!(All),
            Self::Problems => i18n::tr!(RepositoryErrorsFilter),
            Self::Clean => i18n::tr!(Clean),
            Self::Failed => i18n::tr!(Failed),
            Self::Unchecked => i18n::tr!(Unchecked),
            Self::WithoutUpstream => i18n::tr!(NoUpstream),
            Self::Pushes => i18n::tr!(Pushes),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Order {
    Priority,
    Name,
    Path,
    Updated,
}
impl Order {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Priority => i18n::tr!(RepositoryOrderPriority),
            Self::Name => i18n::tr!(RepositoryOrderName),
            Self::Path => i18n::tr!(RepositoryOrderPath),
            Self::Updated => i18n::tr!(RepositoryOrderUpdated),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Menu {
    Filter,
    Sort,
}

impl Screen {
    pub(crate) fn repository_status(&self, path: &Path) -> Status {
        if self.repository_errors.contains_key(path) {
            Status::Failed
        } else {
            Status::of(self.result_for(path))
        }
    }
    pub(crate) fn open_repository_list(&mut self, path: Option<std::path::PathBuf>) {
        if self.page == Page::Detail {
            self.park_workspace();
        }
        self.page = Page::Repositories;
        self.mode = Mode::Browsing;
        self.history.clear();
        self.scroll = 0;
        self.focus(path);
        self.sync_bar();
    }
    pub(crate) fn repository_list_active(&self) -> bool {
        self.page == Page::Repositories && matches!(self.mode, Mode::Browsing)
    }
    pub(crate) fn list_menu(&mut self, menu: Menu) {
        if !self.repository_list_active() {
            self.message = i18n::tr!(RepositoryListOnly).into();
            return;
        }
        let kind = match menu {
            Menu::Filter => crate::command::options::Kind::Filter,
            Menu::Sort => crate::command::options::Kind::Sort,
        };
        self.open_option_menu(kind, self.selected_path(), self.copy_selection());
    }

    pub(crate) fn filter_repositories(&mut self, filter: Filter) {
        if !self.repository_list_active() {
            self.message = i18n::tr!(RepositoryListOnly).into();
            return;
        }
        let path = self.selected_path();
        self.filter = filter;
        self.focus(path);
    }
    pub(crate) fn sort_repositories(&mut self, order: Order) {
        if !self.repository_list_active() {
            self.message = i18n::tr!(RepositoryListOnly).into();
            return;
        }
        let path = self.selected_path();
        self.order = order;
        self.save_view_preference(engine::state::view::Edit::RepositoryOrder(order.into()));
        self.focus(path);
    }
    pub(crate) fn sort_rows(&self, rows: &mut [Row]) {
        match self.order {
            Order::Priority => rows.sort_by(|a, b| {
                let priority = |row: &Row| self.repository_status(&row.path).priority();
                priority(a)
                    .cmp(&priority(b))
                    .then_with(|| a.path.cmp(&b.path))
            }),
            Order::Name => rows.sort_by_cached_key(|row| {
                (
                    row.path
                        .file_name()
                        .unwrap_or(row.path.as_os_str())
                        .to_string_lossy()
                        .to_lowercase(),
                    row.path.clone(),
                )
            }),
            Order::Path => rows.sort_by(|a, b| a.path.cmp(&b.path)),
            Order::Updated => rows.sort_by_cached_key(|row| {
                (
                    Reverse(self.repository_updated_at(&row.path)),
                    row.path.clone(),
                )
            }),
        }
    }
    fn repository_updated_at(&self, path: &Path) -> Option<DateTime<FixedOffset>> {
        if let Some(update) = self.latest_update(path) {
            return Some(update.finished_at);
        }
        let CachedReport::Available { report, .. } = &self.snapshot.report else {
            return None;
        };
        report.repositories.iter().find(|r| r.path() == path)?;
        // 整轮报告中的旧记录允许没有单仓库时间，沿用该报告约定的完成时间。
        let at = report
            .repository_finished_at
            .get(path)
            .unwrap_or(&report.finished_at);
        Some(DateTime::parse_from_rfc3339(at).expect("validated repository report timestamp"))
    }
}
