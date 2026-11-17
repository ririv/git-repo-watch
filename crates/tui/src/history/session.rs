//! Cached graph records preserve topology while search locates rather than hides commits.
use anyhow::{Result, ensure};
use engine::{
    cancel::Cancel,
    local::history::{self, Continuation, Entry, Oid, Page, References},
};
use ratatui::widgets::ListState;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
const MAX_LANES: usize = 64;
static NEXT_SOURCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone)]
pub(crate) struct Record {
    pub entry: Entry,
    pub route: graph::Row,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum GraphFocus {
    #[default]
    Graph,
    Files,
}

#[derive(Debug, Clone)]
pub(crate) struct Session {
    pub source: u64,
    pub focus: GraphFocus,
    pub references: Arc<References>,
    pub records: Arc<Vec<Record>>,
    pub continuation: Continuation,
    pub state: ListState,
    pub query: String,
    pub horizontal: crate::horizontal::Viewport,
    pub width: usize,
    pub selection: super::selection::Selection,
    layout: graph::Layout<Oid>,
}
impl Session {
    pub fn new(page: Page, cancel: &Cancel) -> Result<Self> {
        let mut session = Self {
            focus: GraphFocus::Graph,
            source: NEXT_SOURCE.fetch_add(1, Ordering::Relaxed),
            references: page.references.clone(),
            records: Arc::new(Vec::new()),
            continuation: Continuation::End,
            state: ListState::default(),
            query: String::new(),
            horizontal: Default::default(),
            width: 1,
            selection: Default::default(),
            layout: graph::Layout::default(),
        };
        session.append(page, cancel)?;
        session
            .state
            .select((!session.records.is_empty()).then_some(0));
        Ok(session)
    }
    pub fn refreshed(&self, mut next: Self) -> Self {
        let selected = self.selected().map(|entry| entry.oid);
        next.focus = self.focus;
        next.query = self.query.clone();
        next.horizontal = self.horizontal.clone();
        next.selection = self.selection.clone();
        next.selection
            .retain(|oid| next.records.iter().any(|record| record.entry.oid == *oid));
        next.state = self.state;
        next.state.select(
            selected
                .and_then(|oid| {
                    next.records
                        .iter()
                        .position(|record| record.entry.oid == oid)
                })
                .or_else(|| {
                    next.records
                        .len()
                        .checked_sub(1)
                        .map(|last| self.state.selected().unwrap_or(0).min(last))
                }),
        );
        next
    }
    pub fn append(&mut self, page: Page, cancel: &Cancel) -> Result<()> {
        ensure!(
            Arc::ptr_eq(&self.references, &page.references),
            i18n::tr!(HistorySnapshotUnavailable)
        );
        let records = Arc::make_mut(&mut self.records);
        for entry in page.entries {
            cancel.check()?;
            ensure!(
                entry.graph_parents().len() <= MAX_LANES,
                i18n::tr!(HistoryGraphTooWide)
            );
            let row = self.layout.push(&entry.oid, entry.graph_parents());
            ensure!(row.width < 2 * MAX_LANES, i18n::tr!(HistoryGraphTooWide));
            self.width = self.width.max(row.width);
            records.push(Record { entry, route: row });
        }
        self.continuation = page.continuation;
        Ok(())
    }
    pub fn selected(&self) -> Option<&Entry> {
        self.state
            .selected()
            .and_then(|index| self.records.get(index))
            .map(|record| &record.entry)
    }
    pub fn graph_offset(&self, columns: u16) -> usize {
        self.horizontal
            .offset()
            .min(self.width.saturating_sub(usize::from(columns)))
    }
    pub fn picks(&self) -> Vec<Oid> {
        // 图按拓扑从新到旧排列；执行次序固定为旧到新，不依赖勾选先后。
        self.records
            .iter()
            .rev()
            .filter(|record| self.selection.contains(&record.entry.oid))
            .map(|record| record.entry.oid)
            .collect()
    }
    pub fn mark(&mut self, action: super::SelectionAction) -> Result<()> {
        match action {
            super::SelectionAction::ToggleMode => {
                if self.selection.active() {
                    self.selection.finish();
                } else {
                    self.selection.begin();
                }
            }
            super::SelectionAction::All => {
                ensure!(
                    self.records.len() <= engine::operations::replay::batch::MAX_COMMITS,
                    i18n::tr!(ReplayBatchTooLarge)
                );
                // 历史检索只定位、不隐藏提交；全选始终覆盖已加载的整份列表，不触发分页。
                *self.selection.begin() =
                    self.records.iter().map(|record| record.entry.oid).collect();
            }
            super::SelectionAction::Done => self.selection.finish(),
            super::SelectionAction::Clear => self.selection.begin().clear(),
            super::SelectionAction::ToggleCommit => {
                if let Some(oid) = self.selected().map(|entry| entry.oid) {
                    let marked = self.selection.begin();
                    if !marked.remove(&oid) {
                        ensure!(
                            marked.len() < engine::operations::replay::batch::MAX_COMMITS,
                            i18n::tr!(ReplayBatchTooLarge)
                        );
                        marked.insert(oid);
                    }
                }
            }
        }
        Ok(())
    }
    pub fn advance(&mut self, delta: isize) {
        self.selection.settle();
        if !self.records.is_empty() {
            self.state.select(Some(
                self.state
                    .selected()
                    .map(|index| index.saturating_add_signed(delta))
                    .unwrap_or(0)
                    .min(self.records.len() - 1),
            ));
        }
    }
    pub fn extend_selection(&mut self, delta: isize) -> Result<()> {
        let Some(from) = self.state.selected() else {
            return Ok(());
        };
        let tip = from
            .saturating_add_signed(delta)
            .min(self.records.len() - 1);
        if tip != from {
            self.selection
                .extend(from, tip, |index| self.records[index].entry.oid)?;
            self.state.select(Some(tip));
        }
        Ok(())
    }
    pub fn matches_entry(&self, entry: &Entry) -> bool {
        let needle = self.query.to_lowercase();
        entry.oid.to_string().contains(&needle)
            || entry.subject.to_lowercase().contains(&needle)
            || entry.author.to_lowercase().contains(&needle)
            || self
                .references
                .labels
                .get(&entry.oid)
                .is_some_and(|labels| {
                    labels
                        .iter()
                        .any(|label| label.name().to_lowercase().contains(&needle))
                })
    }
    pub fn matches(&self) -> crate::search::Matches {
        crate::search::Matches::new(
            self.records
                .iter()
                .map(|record| self.matches_entry(&record.entry)),
        )
    }
    pub fn search(&mut self, query: String) {
        self.selection.settle();
        self.query = query;
        if let Some(index) = self.matches().nearest(self.state.selected()) {
            self.state.select(Some(index));
        }
    }
    pub fn next_match(&mut self, backwards: bool) {
        self.selection.settle();
        if !self.query.is_empty()
            && let Some(index) = self.matches().step(self.state.selected(), backwards)
        {
            self.state.select(Some(index));
        }
    }
    pub fn cursor(&self) -> Option<history::Cursor> {
        match &self.continuation {
            Continuation::End => None,
            Continuation::More(cursor) => Some(cursor.clone()),
        }
    }
}
