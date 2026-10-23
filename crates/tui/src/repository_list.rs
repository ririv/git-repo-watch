//! Session-local repository filters, ordering, and command menu transitions.
use crate::{Action, Editor, Mode, Page, Row, Screen, bar::InputBar};
use chrono::{DateTime, FixedOffset};
use engine::{domain::RepositoryResult, service::CachedReport};
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
    pub(crate) fn matches(self, result: Option<&RepositoryResult>) -> bool {
        match self {
            Self::All => true,
            Self::Unchecked => result.is_none(),
            Self::Clean => matches!(result, Some(RepositoryResult::Clean { .. })),
            Self::Failed => matches!(result, Some(RepositoryResult::Failed { .. })),
            Self::Problems => result.is_some_and(|r| engine::domain::Filter::Problems.matches(r)),
            Self::WithoutUpstream => {
                result.is_some_and(|r| engine::domain::Filter::WithoutUpstream.matches(r))
            }
            Self::Pushes => result.is_some_and(|r| engine::domain::Filter::Pushes.matches(r)),
        }
    }
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::All => i18n::tr!(All),
            Self::Problems => i18n::tr!(Problems),
            Self::Clean => i18n::tr!(Clean),
            Self::Failed => i18n::tr!(CheckFailed),
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

#[derive(Debug, Clone, Copy)]
pub(crate) enum Menu {
    Filter,
    Sort,
}

impl Screen {
    pub(crate) fn repository_list_active(&self) -> bool {
        self.page == Page::Repositories && matches!(self.mode, Mode::Browsing)
    }
    pub(crate) fn list_menu(&mut self, menu: Menu) {
        if !self.repository_list_active() {
            self.message = i18n::tr!(RepositoryListOnly).into();
            return;
        }
        let (name, current) = match menu {
            Menu::Filter => ("filter", Action::Filter(self.filter)),
            Menu::Sort => ("sort", Action::Sort(self.order)),
        };
        let input = format!("{name} ");
        let selected = crate::command::matches(&input)
            .iter()
            .position(|command| command.action == current)
            .expect("current list option has a command");
        self.bar = InputBar::Command {
            input: crate::command::Input::global(Editor::new(format!("/{input}"))),
            selected,
            menu: Default::default(),
            path: self.selected_path(),
            selection: self.copy_selection(),
        };
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
        self.focus(path);
    }
    pub(crate) fn sort_rows(&self, rows: &mut [Row]) {
        match self.order {
            Order::Priority => rows.sort_by(|a, b| {
                let priority =
                    |row: &Row| row.result.as_ref().map_or(2, RepositoryResult::priority);
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
