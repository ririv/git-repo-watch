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

#[derive(Debug, Clone)]
pub(crate) struct Session {
    pub source: u64,
    pub references: Arc<References>,
    pub records: Arc<Vec<Record>>,
    pub continuation: Continuation,
    pub state: ListState,
    pub query: String,
    pub pan: usize,
    pub horizontal: crate::horizontal::Viewport,
    pub width: usize,
    layout: graph::Layout<Oid>,
}
impl Session {
    pub fn new(page: Page, cancel: &Cancel) -> Result<Self> {
        let mut session = Self {
            source: NEXT_SOURCE.fetch_add(1, Ordering::Relaxed),
            references: page.references.clone(),
            records: Arc::new(Vec::new()),
            continuation: Continuation::End,
            state: ListState::default(),
            query: String::new(),
            pan: 0,
            horizontal: Default::default(),
            width: 1,
            layout: graph::Layout::default(),
        };
        session.append(page, cancel)?;
        session
            .state
            .select((!session.records.is_empty()).then_some(0));
        Ok(session)
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
    pub fn advance(&mut self, delta: isize) {
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
        self.query = query;
        if let Some(index) = self.matches().nearest(self.state.selected()) {
            self.state.select(Some(index));
        }
    }
    pub fn next_match(&mut self, backwards: bool) {
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
